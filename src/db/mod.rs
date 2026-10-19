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

#[derive(Debug)]
pub enum Request {
    /// Abre a conexão com a URL informada.
    Connect { url: String },
    /// Lista as tabelas do schema atual.
    Tables,
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
