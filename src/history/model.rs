//! Uma conexão salva: o que a memória guarda e o formulário equivalente.
//!
//! A senha só existe em claro aqui, na memória da sessão; no disco vai
//! cifrada (veja `storage`).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{Connection, Field};

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
    pub(super) fn same(&self, other: &Self) -> bool {
        self.engine == other.engine
            && self.host == other.host
            && self.port == other.port
            && self.user == other.user
            && self.database == other.database
    }
}

pub(super) fn now() -> u64 {
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
}


