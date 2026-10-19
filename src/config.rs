//! Configuração de inicialização: a URL de conexão e o formulário da tela
//! de conexão.

/// Campo do formulário de conexão, na ordem em que aparece na tela.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Host,
    Port,
    User,
    Password,
    Database,
}

impl Field {
    /// Todos os campos, na ordem de exibição e de tabulação.
    pub const ALL: [Field; 5] = [
        Field::Host,
        Field::Port,
        Field::User,
        Field::Password,
        Field::Database,
    ];

    /// Rótulo mostrado à esquerda do valor.
    pub fn label(self) -> &'static str {
        match self {
            Field::Host => "host",
            Field::Port => "porta",
            Field::User => "usuário",
            Field::Password => "senha",
            Field::Database => "schema",
        }
    }
}

/// Valores digitados na tela de conexão.
#[derive(Debug)]
pub struct Connection {
    host: String,
    port: String,
    user: String,
    password: String,
    database: String,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: String::from("3306"),
            user: String::from("root"),
            password: String::new(),
            database: String::new(),
        }
    }
}

impl Connection {
    /// Valor atual do campo.
    pub fn value(&self, field: Field) -> &str {
        match field {
            Field::Host => &self.host,
            Field::Port => &self.port,
            Field::User => &self.user,
            Field::Password => &self.password,
            Field::Database => &self.database,
        }
    }

    /// Valor do campo, para edição.
    pub fn value_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Host => &mut self.host,
            Field::Port => &mut self.port,
            Field::User => &mut self.user,
            Field::Password => &mut self.password,
            Field::Database => &mut self.database,
        }
    }

    /// Monta a URL `mysql://...` a partir dos campos, recusando o que falta.
    ///
    /// O schema é opcional: sem ele a conexão abre no servidor e a lista de
    /// bancos escolhe onde trabalhar.
    ///
    /// Usuário, senha e schema vão codificados (`%XX`): a biblioteca do MySQL
    /// decodifica esses três ao ler a URL, então `@` ou `/` na senha não
    /// quebram o endereço.
    pub fn url(&self) -> Result<String, String> {
        let host = self.host.trim();
        let port = self.port.trim();
        let user = self.user.trim();
        let database = self.database.trim();
        if host.is_empty() {
            return Err(String::from("informe o host"));
        }
        if user.is_empty() {
            return Err(String::from("informe o usuário"));
        }
        let port: u16 = port
            .parse()
            .map_err(|_| format!("porta inválida: {port}"))?;

        let mut url = format!("mysql://{}", encode(user));
        if !self.password.is_empty() {
            url.push(':');
            url.push_str(&encode(&self.password));
        }
        url.push('@');
        url.push_str(host);
        url.push(':');
        url.push_str(&port.to_string());
        if !database.is_empty() {
            url.push('/');
            url.push_str(&encode(database));
        }
        Ok(url)
    }
}

/// Percent-encoding mínimo (`RFC 3986`): deixa passar os caracteres
/// incondicionalmente seguros e escapa o resto.
fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// URL do argumento ou, sem ele, de `DATABASE_URL` / `MYSQL_URL`.
///
/// `Ok(None)` quando nenhuma foi informada: nesse caso o dbterm abre na tela
/// de conexão.
pub fn url(argument: Option<String>) -> Result<Option<String>, String> {
    let url = argument
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .or_else(|| std::env::var("MYSQL_URL").ok());
    let Some(url) = url else {
        return Ok(None);
    };
    if !url.starts_with("mysql://") {
        return Err(format!("URL não suportada: {url}"));
    }
    Ok(Some(url))
}

#[cfg(test)]
mod tests {
    use super::{Connection, Field, url};

    #[test]
    fn aceita_somente_mysql() {
        assert!(url(Some(String::from("mysql://user@localhost/app"))).is_ok());
        assert!(url(Some(String::from("postgres://localhost/app"))).is_err());
        assert!(url(Some(String::from("localhost"))).is_err());
    }

    #[test]
    fn sem_url_nao_erra() {
        assert_eq!(url(None).ok().flatten(), env_url());
    }

    /// O ambiente de teste pode ter `DATABASE_URL`/`MYSQL_URL`; o resultado
    /// esperado é justamente a variável, se houver.
    fn env_url() -> Option<String> {
        std::env::var("DATABASE_URL")
            .ok()
            .or_else(|| std::env::var("MYSQL_URL").ok())
    }

    #[test]
    fn formulario_monta_a_url() {
        let mut connection = Connection::default();
        *connection.value_mut(Field::Database) = String::from("app");
        assert_eq!(connection.url().unwrap(), "mysql://root@127.0.0.1:3306/app");
    }

    #[test]
    fn formulario_codifica_senha_e_schema() {
        let mut connection = Connection::default();
        *connection.value_mut(Field::User) = String::from("an@");
        *connection.value_mut(Field::Password) = String::from("p:a/s");
        *connection.value_mut(Field::Database) = String::from("meu schema");
        assert_eq!(
            connection.url().unwrap(),
            "mysql://an%40:p%3Aa%2Fs@127.0.0.1:3306/meu%20schema"
        );
    }

    #[test]
    fn schema_e_opcional() {
        let connection = Connection::default();
        assert_eq!(connection.url().unwrap(), "mysql://root@127.0.0.1:3306");
    }

    #[test]
    fn formulario_recusa_campos_faltando() {
        let mut connection = Connection::default();
        *connection.value_mut(Field::Database) = String::from("app");

        *connection.value_mut(Field::Host) = String::new();
        assert!(connection.url().is_err(), "sem host");
        *connection.value_mut(Field::Host) = String::from("db.local");

        *connection.value_mut(Field::User) = String::new();
        assert!(connection.url().is_err(), "sem usuário");
        *connection.value_mut(Field::User) = String::from("root");

        *connection.value_mut(Field::Port) = String::from("abc");
        assert!(connection.url().is_err(), "porta não numérica");
    }
}
