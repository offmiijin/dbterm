//! Histórico das conexões rápidas: o que fica salvo entre execuções.
//!
//! A senha é guardada, mas cifrada: em memória ela fica em claro para o app
//! usar; no disco vai só um blob do AES-256-GCM. Veja `crypto` para o limite
//! dessa proteção.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::{Connection, Field};
use crate::crypto::{self, Key};

/// Quantas conexões a tela inicial mostra.
pub const RECENTES: usize = 3;

/// Uma conexão salva. A senha só existe em claro aqui, na memória da sessão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub engine: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    pub password: Option<String>,
    pub last_used: u64,
}

impl Saved {
    /// Motor do MySQL, o único suportado por enquanto.
    pub const MYSQL: &'static str = "mysql";

    /// Cópia do formulário, incluindo a senha digitada.
    pub fn from_form(connection: &Connection) -> Self {
        let senha = connection.value(Field::Password);
        Self {
            engine: Self::MYSQL.to_owned(),
            host: connection.value(Field::Host).trim().to_owned(),
            port: connection.value(Field::Port).trim().parse().unwrap_or(3306),
            user: connection.value(Field::User).trim().to_owned(),
            database: connection.value(Field::Database).trim().to_owned(),
            password: if senha.is_empty() {
                None
            } else {
                Some(senha.to_owned())
            },
            last_used: now(),
        }
    }

    /// Mesma conexão, independente de quando foi usada e da senha guardada.
    fn same(&self, other: &Self) -> bool {
        self.engine == other.engine
            && self.host == other.host
            && self.port == other.port
            && self.user == other.user
            && self.database == other.database
    }
}

/// Como a conexão aparece no JSON: a senha, quando existe, já está cifrada.
#[derive(Debug, Serialize, Deserialize)]
struct Stored {
    engine: String,
    host: String,
    port: u16,
    user: String,
    database: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(default)]
    last_used: u64,
}

impl Stored {
    /// Do disco para a memória, decifrando a senha quando a chave permite.
    fn abrir(self, key: Option<&Key>) -> Saved {
        let password = self.password.as_deref().and_then(|blob| key?.open(blob));
        Saved {
            engine: self.engine,
            host: self.host,
            port: self.port,
            user: self.user,
            database: self.database,
            password,
            last_used: self.last_used,
        }
    }
}

impl Saved {
    /// Da memória para o disco, cifrando a senha; sem chave, a senha é
    /// descartada em vez de ir em texto puro.
    fn guardar(&self, key: Option<&Key>) -> Stored {
        let password = self.password.as_deref().and_then(|senha| key?.seal(senha));
        Stored {
            engine: self.engine.clone(),
            host: self.host.clone(),
            port: self.port,
            user: self.user.clone(),
            database: self.database.clone(),
            password,
            last_used: self.last_used,
        }
    }
}

/// As conexões salvas, da mais recente para a mais antiga.
#[derive(Debug, Default)]
pub struct History {
    connections: Vec<Saved>,
    /// Onde ler/gravar; `None` quando a sessão não persiste (testes).
    path: Option<PathBuf>,
    /// Chave da cifra da senha; `None` quando não deu para obtê-la.
    key: Option<Key>,
}

impl History {
    /// Lê o arquivo do usuário; arquivo faltando ou inválido vira histórico
    /// vazio, nunca erro.
    pub fn load() -> Self {
        let path = default_path();
        let key = default_key_path().as_deref().and_then(Key::load_or_create);
        let connections = path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|json| serde_json::from_str::<Vec<Stored>>(&json).ok())
            .map(|stored| {
                stored
                    .into_iter()
                    .map(|entry| entry.abrir(key.as_ref()))
                    .collect()
            })
            .unwrap_or_default();
        let mut history = Self {
            connections,
            path,
            key,
        };
        history.sort();
        history
    }

    /// Um histórico só em memória, que não toca no disco.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Histórico em memória com entradas dadas, já ordenado.
    pub fn with(entries: Vec<Saved>) -> Self {
        let mut history = Self {
            connections: entries,
            ..Self::default()
        };
        history.sort();
        history
    }

    /// As conexões mais recentes, no máximo [`RECENTES`].
    pub fn recent(&self) -> &[Saved] {
        &self.connections[..self.connections.len().min(RECENTES)]
    }

    /// Marca a conexão como a mais recente: sobe para o topo, sem duplicar.
    pub fn touch(&mut self, saved: Saved) {
        self.connections.retain(|entry| !entry.same(&saved));
        self.connections.push(saved);
        self.sort();
        self.save();
    }

    fn sort(&mut self) {
        self.connections
            .sort_by(|a, b| b.last_used.cmp(&a.last_used));
    }

    /// Grava best-effort: falha de escrita não derruba a sessão.
    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        let stored: Vec<Stored> = self
            .connections
            .iter()
            .map(|entry| entry.guardar(self.key.as_ref()))
            .collect();
        if let Ok(json) = serde_json::to_string_pretty(&stored) {
            let _ = crypto::write_private(path, json.as_bytes());
        }
    }
}

/// `$XDG_CONFIG_HOME/dbterm/conexoes.json` ou `~/.config/...`.
fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("dbterm").join("conexoes.json"))
}

/// A chave da cifra mora ao lado do arquivo de conexões.
fn default_key_path() -> Option<PathBuf> {
    default_path().map(|path| path.with_file_name("chave.bin"))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(host: &str, last_used: u64) -> Saved {
        Saved {
            engine: String::from(Saved::MYSQL),
            host: String::from(host),
            port: 3306,
            user: String::from("root"),
            database: String::from("app"),
            password: None,
            last_used,
        }
    }

    #[test]
    fn recent_mostra_no_maximo_tres_e_ordena() {
        let history = History::with(vec![
            saved("a", 10),
            saved("b", 30),
            saved("c", 20),
            saved("d", 40),
        ]);
        let hosts: Vec<&str> = history
            .recent()
            .iter()
            .map(|entry| entry.host.as_str())
            .collect();
        assert_eq!(hosts, ["d", "b", "c"]);
    }

    #[test]
    fn touch_sobe_para_o_topo_sem_duplicar() {
        let mut history = History::with(vec![saved("a", 10), saved("b", 20)]);
        history.touch(saved("a", 30));
        let hosts: Vec<&str> = history
            .recent()
            .iter()
            .map(|entry| entry.host.as_str())
            .collect();
        assert_eq!(hosts, ["a", "b"]);
    }

    #[test]
    fn formulario_vira_conexao_com_senha() {
        let mut connection = Connection::default();
        *connection.value_mut(Field::Database) = String::from("app");
        *connection.value_mut(Field::Password) = String::from("segredo");

        let saved = Saved::from_form(&connection);
        assert_eq!(saved.engine, "mysql");
        assert_eq!(saved.host, "127.0.0.1");
        assert_eq!(saved.port, 3306);
        assert_eq!(saved.user, "root");
        assert_eq!(saved.database, "app");
        assert_eq!(saved.password.as_deref(), Some("segredo"));
    }

    #[test]
    fn senha_nao_vai_em_claro_para_o_json() {
        let key = Key::test();
        let mut conexao = saved("a", 10);
        conexao.password = Some(String::from("segredo"));

        let json = serde_json::to_string(&conexao.guardar(Some(&key))).expect("serializa");
        assert!(!json.contains("segredo"), "{json}");

        let voltou = serde_json::from_str::<Stored>(&json)
            .expect("parseia")
            .abrir(Some(&key));
        assert_eq!(voltou.password.as_deref(), Some("segredo"));
    }

    #[test]
    fn sem_chave_a_senha_nao_vai_para_o_disco() {
        let mut conexao = saved("a", 10);
        conexao.password = Some(String::from("segredo"));

        let json = serde_json::to_string(&conexao.guardar(None)).expect("serializa");
        assert!(!json.contains("segredo"), "{json}");
        assert!(!json.contains("password"), "{json}");
    }
}
