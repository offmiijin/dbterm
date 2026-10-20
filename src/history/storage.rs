//! Persistência: o JSON no disco, com a senha cifrada, e os caminhos.
//!
//! Sem chave, a senha é descartada em vez de ir em texto puro.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::crypto::Key;

use super::model::Saved;

/// Como a conexão aparece no JSON: a senha, quando existe, já está cifrada.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Stored {
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
    pub(super) fn abrir(self, key: Option<&Key>) -> Saved {
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
    pub(super) fn guardar(&self, key: Option<&Key>) -> Stored {
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

/// `$XDG_CONFIG_HOME/dbterm/conexoes.json` ou `~/.config/...`.
pub(super) fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("dbterm").join("conexoes.json"))
}

/// `chave.bin`: fallback e lugar da chave nas versões anteriores ao cofre.
pub(super) fn default_key_path() -> Option<PathBuf> {
    default_path().map(|path| path.with_file_name("chave.bin"))
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
