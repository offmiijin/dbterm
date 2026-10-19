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
    use super::{Charset, Collation, DatabaseAlter, DatabaseInfo, identificador};

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
}
