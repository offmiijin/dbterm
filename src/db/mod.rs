//! Banco de dados: contrato com a UI e ponte para o MySQL.
//!
//! A UI nunca fala com o banco direto. Ela envia [`Request`] por canal e lê
//! [`Response`] no loop de eventos, então uma query lenta nunca trava o desenho.
//! O contrato fica aqui; a conversa com o MySQL vive em `mysql` e a thread de
//! trabalho em `worker`.

mod mysql;
mod preview;
mod worker;

pub use preview::{SqlPart, update_preview};
pub use worker::Db;

/// Quantas linhas a grade busca por página: ao encostar na última, ela pede a
/// página seguinte, até a tabela acabar.
pub const ROW_LIMIT: usize = 200;

/// Tipo da coluna reduzido ao que a UI precisa: número ou texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    /// Inteiro, decimal ou ponto flutuante: literal sem aspas.
    Number,
    /// Todo o resto (texto, data, blob...): literal entre aspas.
    Text,
}

/// Uma página de dados pronta para desenhar: tudo já virou texto.
#[derive(Debug)]
pub struct TableData {
    pub columns: Vec<String>,
    /// Tipo de cada coluna, na mesma ordem de [`TableData::columns`].
    pub kinds: Vec<ColumnKind>,
    pub rows: Vec<Vec<String>>,
    /// Índices das colunas da chave primária; vazio quando a tabela não tem PK.
    pub primary_key: Vec<usize>,
}

impl TableData {
    /// Índice, nome e valor atual de cada coluna da PK da linha `row`.
    pub(crate) fn key_columns(&self, row: usize) -> Vec<(usize, &str, &str)> {
        let Some(row) = self.rows.get(row) else {
            return Vec::new();
        };
        self.primary_key
            .iter()
            .filter_map(|&index| {
                Some((
                    index,
                    self.columns.get(index)?.as_str(),
                    row.get(index)?.as_str(),
                ))
            })
            .collect()
    }

    /// Nomes e valores atuais da chave primária da linha `row`, na ordem da PK.
    pub fn key_of(&self, row: usize) -> Vec<(String, String)> {
        self.key_columns(row)
            .into_iter()
            .map(|(_, name, value)| (name.to_owned(), value.to_owned()))
            .collect()
    }

    /// Tipo da coluna `index` (texto quando a página não trouxe o tipo).
    pub(crate) fn kind(&self, index: usize) -> ColumnKind {
        self.kinds.get(index).copied().unwrap_or(ColumnKind::Text)
    }
}

/// Um charset do servidor e a collation que ele usa por padrão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Charset {
    pub name: String,
    pub default_collation: String,
}

/// Uma collation do servidor e o charset a que ela pertence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collation {
    pub name: String,
    pub charset: String,
}

/// Propriedades de um banco (schema): o que o pop-up "Editar" mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseInfo {
    pub name: String,
    /// Charset padrão das tabelas novas do banco.
    pub charset: String,
    /// Collation padrão das tabelas novas do banco.
    pub collation: String,
    /// `None` quando o servidor não informa a opção (antes do MySQL 8.0.22).
    pub read_only: Option<bool>,
    /// Dados mais índices das tabelas do banco, em bytes.
    pub size_bytes: u64,
    /// Charsets que o servidor oferece, para a lista de seleção.
    pub charsets: Vec<Charset>,
    /// Collations que o servidor oferece, com o charset de cada uma.
    pub collations: Vec<Collation>,
}

/// As opções de um banco que mudaram; `None` fica como está no servidor.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DatabaseAlter {
    pub charset: Option<String>,
    pub collation: Option<String>,
    pub read_only: Option<bool>,
}

impl DatabaseAlter {
    /// Só o que difere entre o que está no servidor e o que foi editado.
    pub fn between(before: &DatabaseInfo, after: &DatabaseInfo) -> Self {
        Self {
            charset: (before.charset != after.charset).then(|| after.charset.clone()),
            collation: (before.collation != after.collation).then(|| after.collation.clone()),
            read_only: match (before.read_only, after.read_only) {
                (antes, depois) if antes != depois => depois,
                _ => None,
            },
        }
    }

    /// Nenhuma opção mudou: não há `ALTER` a executar.
    pub fn is_empty(&self) -> bool {
        self.charset.is_none() && self.collation.is_none() && self.read_only.is_none()
    }
}

/// Propriedades de uma tabela: o que o pop-up "Editar" mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInfo {
    pub name: String,
    pub comment: String,
    pub engine: String,
    pub row_format: String,
    /// `None` quando a tabela não tem coluna `AUTO_INCREMENT`.
    pub auto_increment: Option<u64>,
    /// Charset padrão das colunas novas da tabela.
    pub charset: String,
    /// Collation padrão das colunas novas da tabela.
    pub collation: String,
    /// Dados mais índices da tabela, em bytes.
    pub size_bytes: u64,
    /// Engines que o servidor aceita, para a lista de seleção.
    pub engines: Vec<String>,
    /// Charsets que o servidor oferece, para a lista de seleção.
    pub charsets: Vec<Charset>,
    /// Collations que o servidor oferece, com o charset de cada uma.
    pub collations: Vec<Collation>,
}

/// Formatos de linha aceitos no `ALTER TABLE`. Os nomes seguem o vocabulário
/// do servidor (`SHOW TABLE STATUS` mostra `Dynamic`), para a lista da UI abrir
/// no valor atual; o `FIXED` é do MyISAM e o servidor recusa o que a engine da
/// tabela não suportar.
pub const ROW_FORMATS: [&str; 6] = [
    "Default",
    "Dynamic",
    "Compact",
    "Compressed",
    "Redundant",
    "Fixed",
];

/// As opções de uma tabela que mudaram; `None` fica como está no servidor.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TableAlter {
    /// Novo nome, quando a tabela foi renomeada.
    pub rename: Option<String>,
    pub comment: Option<String>,
    pub engine: Option<String>,
    pub row_format: Option<String>,
    pub auto_increment: Option<u64>,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

impl TableAlter {
    /// Só o que difere entre o que está no servidor e o que foi editado.
    pub fn between(before: &TableInfo, after: &TableInfo) -> Self {
        Self {
            rename: (before.name != after.name).then(|| after.name.clone()),
            comment: (before.comment != after.comment).then(|| after.comment.clone()),
            engine: (before.engine != after.engine).then(|| after.engine.clone()),
            row_format: (before.row_format != after.row_format).then(|| after.row_format.clone()),
            auto_increment: match (before.auto_increment, after.auto_increment) {
                (antes, depois) if antes != depois => depois,
                _ => None,
            },
            charset: (before.charset != after.charset).then(|| after.charset.clone()),
            collation: (before.collation != after.collation).then(|| after.collation.clone()),
        }
    }

    /// Nenhuma opção mudou: não há `ALTER` a executar.
    pub fn is_empty(&self) -> bool {
        self.rename.is_none()
            && self.comment.is_none()
            && self.engine.is_none()
            && self.row_format.is_none()
            && self.auto_increment.is_none()
            && self.charset.is_none()
            && self.collation.is_none()
    }
}

/// Nome de charset ou collation aceito no DDL: letras, números e `_`.
///
/// DDL não aceita placeholder para identificador, então esses nomes entram
/// na query: só o que passa por aqui pode ir para o texto do `ALTER`.
pub(crate) fn identificador(valor: &str) -> bool {
    !valor.is_empty()
        && valor
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Debug)]
pub enum Request {
    /// Abre a conexão com a URL informada.
    Connect { url: String },
    /// Lista as tabelas do schema atual.
    Tables,
    /// Lista os bancos (schemas) do servidor.
    Databases,
    /// Lê as propriedades do banco informado, para o pop-up "Editar".
    DatabaseInfo { database: String },
    /// Aplica as opções que mudaram no banco informado.
    AlterDatabase {
        database: String,
        alter: DatabaseAlter,
    },
    /// Lê as propriedades da tabela informada, para o pop-up "Editar".
    TableInfo { table: String },
    /// Aplica as opções que mudaram na tabela informada.
    AlterTable { table: String, alter: TableAlter },
    /// Passa a trabalhar no banco informado, na mesma conexão.
    UseDatabase { database: String },
    /// Busca uma página de `limit` linhas de uma tabela, a partir de `offset`.
    Rows {
        table: String,
        offset: usize,
        limit: usize,
    },
    /// Grava `value` (`None` = `NULL`) na coluna `column` da linha identificada
    /// pelos valores atuais da chave primária.
    Update {
        table: String,
        column: String,
        value: Option<String>,
        key: Vec<(String, String)>,
    },
}

#[derive(Debug)]
pub enum Response {
    Connected {
        server: String,
        database: String,
    },
    Tables(Vec<String>),
    /// Os bancos do servidor, em ordem alfabética.
    Databases(Vec<String>),
    /// As propriedades do banco pedido no `DatabaseInfo`.
    DatabaseInfo(DatabaseInfo),
    /// O `ALTER DATABASE` foi aplicado.
    DatabaseUpdated {
        database: String,
    },
    /// As propriedades da tabela pedida no `TableInfo`.
    TableInfo(TableInfo),
    /// O `ALTER TABLE` foi aplicado; `after` é o nome final, que muda no rename.
    TableUpdated {
        before: String,
        after: String,
    },
    /// O banco atual mudou; a UI já pode pedir as tabelas dele.
    Database(String),
    Rows {
        table: String,
        /// Quantas linhas já vinham carregadas antes desta página.
        offset: usize,
        /// Quantas linhas foram pedidas: a página veio inteira?
        limit: usize,
        data: TableData,
    },
    /// O `UPDATE` foi aplicado; a UI recarrega a tabela.
    Updated {
        table: String,
    },
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::{
        Charset, Collation, DatabaseAlter, DatabaseInfo, TableAlter, TableInfo, identificador,
    };

    fn info(charset: &str, collation: &str, read_only: Option<bool>) -> DatabaseInfo {
        DatabaseInfo {
            name: String::from("app"),
            charset: String::from(charset),
            collation: String::from(collation),
            read_only,
            size_bytes: 0,
            charsets: vec![Charset {
                name: String::from(charset),
                default_collation: String::from(collation),
            }],
            collations: vec![Collation {
                name: String::from(collation),
                charset: String::from(charset),
            }],
        }
    }

    #[test]
    fn alter_leva_so_o_que_mudou() {
        let antes = info("utf8mb4", "utf8mb4_0900_ai_ci", Some(false));
        let depois = info("utf8mb4", "utf8mb4_bin", Some(false));

        let alter = DatabaseAlter::between(&antes, &depois);

        assert_eq!(alter.charset, None);
        assert_eq!(alter.collation.as_deref(), Some("utf8mb4_bin"));
        assert_eq!(alter.read_only, None);
        assert!(!alter.is_empty());
    }

    #[test]
    fn alter_vazio_quando_nada_muda() {
        let antes = info("utf8mb4", "utf8mb4_bin", Some(true));
        let depois = antes.clone();

        let alter = DatabaseAlter::between(&antes, &depois);

        assert!(alter.is_empty());
    }

    #[test]
    fn alter_sem_opcao_informada_nao_inventa_mudanca() {
        // Servidor antigo: a opção chega como `None` dos dois lados.
        let antes = info("utf8mb4", "utf8mb4_bin", None);
        let depois = info("utf8mb4", "utf8mb4_bin", None);

        assert!(DatabaseAlter::between(&antes, &depois).is_empty());
    }

    #[test]
    fn identificador_recusa_o_que_nao_pode_entrar_no_ddl() {
        assert!(identificador("utf8mb4_0900_ai_ci"));
        assert!(identificador("binary"));
        assert!(!identificador(""));
        assert!(!identificador("utf8 mb4"));
        assert!(!identificador("utf8mb4;DROP"));
        assert!(!identificador("`x`"));
    }

    fn tabela(nome: &str) -> TableInfo {
        TableInfo {
            name: String::from(nome),
            comment: String::new(),
            engine: String::from("InnoDB"),
            row_format: String::from("Dynamic"),
            auto_increment: Some(1024),
            charset: String::from("utf8mb4"),
            collation: String::from("utf8mb4_0900_ai_ci"),
            size_bytes: 0,
            engines: Vec::new(),
            charsets: Vec::new(),
            collations: Vec::new(),
        }
    }

    #[test]
    fn alter_da_tabela_leva_so_o_que_mudou() {
        let antes = tabela("users");
        let mut depois = antes.clone();
        depois.name = String::from("clientes");
        depois.comment = String::from("cadastro");

        let alter = TableAlter::between(&antes, &depois);

        assert_eq!(alter.rename.as_deref(), Some("clientes"));
        assert_eq!(alter.comment.as_deref(), Some("cadastro"));
        assert_eq!(alter.engine, None);
        assert_eq!(alter.auto_increment, None);
        assert!(!alter.is_empty());
    }

    #[test]
    fn alter_da_tabela_vazio_quando_nada_muda() {
        let antes = tabela("users");
        assert!(TableAlter::between(&antes, &antes.clone()).is_empty());
    }

    #[test]
    fn alter_da_tabela_sem_auto_increment_nao_inventa_mudanca() {
        let mut antes = tabela("users");
        antes.auto_increment = None;
        let depois = antes.clone();

        assert_eq!(TableAlter::between(&antes, &depois).auto_increment, None);
    }
}
