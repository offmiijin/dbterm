//! Histórico das conexões rápidas: o que fica salvo entre execuções.
//!
//! A senha é guardada, mas cifrada: em memória ela fica em claro para o app
//! usar; no disco vai só um blob do AES-256-GCM. A chave dessa cifra mora no
//! cofre do sistema (`keychain`), com o arquivo `chave.bin` como fallback. Veja
//! `crypto` para o limite dessa proteção.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::{Connection, Field};
use crate::crypto::{self, Key};
use crate::keychain::{Keychain, Store};

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

    /// URL `mysql://...` para reabrir a conexão guardada, com a senha em
    /// claro que veio do histórico.
    pub fn url(&self) -> Result<String, String> {
        self.form().url()
    }

    /// O formulário equivalente a esta conexão salva, senha inclusa.
    pub fn form(&self) -> Connection {
        let mut form = Connection::default();
        *form.value_mut(Field::Host) = self.host.clone();
        *form.value_mut(Field::Port) = self.port.to_string();
        *form.value_mut(Field::User) = self.user.clone();
        *form.value_mut(Field::Password) = self.password.clone().unwrap_or_default();
        *form.value_mut(Field::Database) = self.database.clone();
        form
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
        let key = chave(&Keychain, default_key_path().as_deref());
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

    /// Todas as conexões salvas, para a navegação da tela de trabalho.
    pub fn all(&self) -> &[Saved] {
        &self.connections
    }

    /// Marca a conexão como a mais recente: sobe para o topo, sem duplicar.
    pub fn touch(&mut self, saved: Saved) {
        self.connections.retain(|entry| !entry.same(&saved));
        self.connections.push(saved);
        self.sort();
        self.save();
    }

    /// Troca a conexão editada pela que estava em `index`, no mesmo lugar da
    /// lista: a data de uso — e portanto a ordem — não muda.
    pub fn replace(&mut self, index: usize, mut saved: Saved) {
        let Some(atual) = self.connections.get(index) else {
            return;
        };
        saved.last_used = atual.last_used;
        self.connections[index] = saved;
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

/// `chave.bin`: fallback e lugar da chave nas versões anteriores ao cofre.
fn default_key_path() -> Option<PathBuf> {
    default_path().map(|path| path.with_file_name("chave.bin"))
}

/// Move a chave do `chave.bin` para o cofre do sistema e apaga o arquivo.
///
/// Chamado no início do binário. Só apaga depois que o cofre comprovadamente
/// tem essa chave: se a gravação falha, ou se o cofre já tem outra chave, o
/// arquivo fica onde está.
pub fn migrar_chave() {
    migrar(&Keychain, default_key_path().as_deref());
}

/// A chave da cifra: o `chave.bin` legado tem prioridade (é ele que decifra os
/// dados atuais), depois o cofre; sem nenhum dos dois, sorteia uma nova e tenta
/// o cofre antes de recorrer ao arquivo.
fn chave(cofre: &impl Store, path: Option<&Path>) -> Option<Key> {
    if let Some(arquivo) = path.and_then(Key::read) {
        return Some(arquivo);
    }
    if let Some(bytes) = cofre.read() {
        return Some(Key::from_bytes(bytes));
    }
    let nova = Key::random();
    if cofre.write(nova.bytes()) {
        return Some(nova);
    }
    // Sem cofre, o arquivo 0600 continua sendo o lugar da chave.
    path.and_then(Key::load_or_create)
}

/// Leva a chave de `path` para o cofre e apaga o arquivo quando dá.
fn migrar(cofre: &impl Store, path: Option<&Path>) {
    let Some(do_arquivo) = path.and_then(Key::read) else {
        return;
    };
    let Some(path) = path else {
        return;
    };
    let no_cofre = match cofre.read() {
        Some(bytes) => &bytes == do_arquivo.bytes(),
        None => cofre.write(do_arquivo.bytes()),
    };
    if no_cofre {
        let _ = fs::remove_file(path);
    }
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
    fn url_da_conexao_salva_leva_a_senha_e_o_schema() {
        let mut entry = saved("db.local", 1);
        entry.port = 3307;
        entry.password = Some(String::from("p:a@"));
        assert_eq!(
            entry.url().unwrap(),
            "mysql://root:p%3Aa%40@db.local:3307/app"
        );
    }

    #[test]
    fn url_da_conexao_salva_dispensa_senha_e_schema() {
        let mut entry = saved("db.local", 1);
        entry.database.clear();
        assert_eq!(entry.url().unwrap(), "mysql://root@db.local:3306");
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
    fn replace_edita_a_conexao_sem_mudar_o_lugar_na_lista() {
        let mut history = History::with(vec![saved("a", 10), saved("b", 20)]);
        let mut editada = saved("novo", 99);
        editada.port = 3307;

        history.replace(1, editada);

        let hosts: Vec<&str> = history
            .all()
            .iter()
            .map(|entry| entry.host.as_str())
            .collect();
        assert_eq!(hosts, ["b", "novo"], "a ordem é a mesma");
        assert_eq!(history.all()[1].port, 3307);
        assert_eq!(history.all()[1].last_used, 10, "a data de uso é preservada");
    }

    #[test]
    fn replace_ignora_indice_fora_da_lista() {
        let mut history = History::with(vec![saved("a", 10)]);
        history.replace(7, saved("novo", 99));
        assert_eq!(history.all()[0].host, "a");
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

    /// Cofre de mentira, para exercitar a escolha da chave sem tocar no cofre
    /// do sistema operacional.
    #[derive(Default)]
    struct CofreFalso {
        chave: std::cell::RefCell<Option<[u8; 32]>>,
        aceita: bool,
    }

    impl CofreFalso {
        /// Cofre que recusa gravação, como num SSH sem D-Bus.
        fn mudo() -> Self {
            Self::default()
        }

        fn com(chave: [u8; 32]) -> Self {
            Self {
                chave: std::cell::RefCell::new(Some(chave)),
                aceita: true,
            }
        }
    }

    impl Store for CofreFalso {
        fn read(&self) -> Option<[u8; 32]> {
            *self.chave.borrow()
        }

        fn write(&self, chave: &[u8; 32]) -> bool {
            if !self.aceita {
                return false;
            }
            *self.chave.borrow_mut() = Some(*chave);
            true
        }
    }

    /// Caminho único no diretório temporário, já sem arquivo.
    fn caminho(nome: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbterm-history-{}", std::process::id()));
        let path = dir.join(nome);
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn chave_nova_vai_para_o_cofre_e_nao_cria_arquivo() {
        let path = caminho("nova.bin");
        let cofre = CofreFalso {
            aceita: true,
            ..CofreFalso::default()
        };

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert!(!path.exists());
        assert_eq!(cofre.read().as_ref(), Some(chave.bytes()));
    }

    #[test]
    fn sem_cofre_a_chave_nova_vai_para_o_arquivo() {
        let path = caminho("fallback.bin");
        let cofre = CofreFalso::mudo();

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert_eq!(
            Key::read(&path).as_ref().map(Key::bytes),
            Some(chave.bytes())
        );
        assert_eq!(cofre.read(), None);
    }

    #[test]
    fn chave_do_cofre_e_usada_sem_criar_arquivo() {
        let path = caminho("cofre.bin");
        let cofre = CofreFalso::com([9; 32]);

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert_eq!(chave.bytes(), &[9; 32]);
        assert!(!path.exists());
    }

    #[test]
    fn arquivo_legado_tem_prioridade_sobre_o_cofre() {
        let path = caminho("prioridade.bin");
        crypto::write_private(&path, &[1; 32]).expect("grava");
        let cofre = CofreFalso::com([2; 32]);

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert_eq!(chave.bytes(), &[1; 32]);
    }

    #[test]
    fn migrar_move_a_chave_para_o_cofre_e_apaga_o_arquivo() {
        let path = caminho("migra.bin");
        crypto::write_private(&path, &[3; 32]).expect("grava");
        let cofre = CofreFalso {
            aceita: true,
            ..CofreFalso::default()
        };

        migrar(&cofre, Some(&path));

        assert!(!path.exists());
        assert_eq!(cofre.read(), Some([3; 32]));
    }

    #[test]
    fn migrar_sem_cofre_mantem_o_arquivo() {
        let path = caminho("migra-sem-cofre.bin");
        crypto::write_private(&path, &[4; 32]).expect("grava");
        let cofre = CofreFalso::mudo();

        migrar(&cofre, Some(&path));

        assert!(path.exists());
        assert_eq!(cofre.read(), None);
    }

    #[test]
    fn migrar_nao_sobrescreve_cofre_com_outra_chave() {
        let path = caminho("migra-conflito.bin");
        crypto::write_private(&path, &[5; 32]).expect("grava");
        let cofre = CofreFalso::com([6; 32]);

        migrar(&cofre, Some(&path));

        assert_eq!(cofre.read(), Some([6; 32]));
        assert_eq!(Key::read(&path).as_ref().map(Key::bytes), Some(&[5; 32]));
    }
}
