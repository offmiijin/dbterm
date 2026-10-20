//! Estado e modelos de edição: telas, foco, modos e pop-ups.
//!
//! Tipos puros, sem loop de eventos nem acesso ao banco. Os comportamentos dos
//! pop-ups (ativar, escolher, digitar) moram em `edits`; o `App` mora em
//! `super` e as operações em `ops` e `responses`.

use crate::config::{Connection, DatabaseField, Field, TableField};
use crate::db::{DatabaseInfo, TableInfo};

/// Em qual tela o app está.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Lista de conexões rápidas, a tela inicial.
    Quick,
    /// Formulário de conexão, para uma conexão nova ou pré-preenchida.
    Connect,
    /// Tabelas e dados da conexão aberta.
    Browse,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Lista de conexões salvas.
    Connections,
    /// Lista de bancos do servidor conectado.
    Databases,
    /// Lista de tabelas do banco atual.
    Tables,
    /// Grade de dados à direita.
    Data,
}

impl Focus {
    /// O nível anterior da cascata; `None` na conexão, a raiz.
    pub fn above(self) -> Option<Focus> {
        match self {
            Focus::Connections => None,
            Focus::Databases => Some(Focus::Connections),
            Focus::Tables => Some(Focus::Databases),
            Focus::Data => Some(Focus::Tables),
        }
    }
}

pub enum Status {
    Connecting,
    Loading(String),
    Ready,
    /// Informação passageira (ex.: tabela sem chave primária).
    Notice(String),
    Failed(String),
}

/// Em que nível de navegação/edição a grade está.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Movimento vertical entre registros.
    Rows,
    /// Movimento horizontal entre colunas do registro selecionado.
    Columns,
    /// Digitando o novo valor da célula selecionada.
    Edit,
    /// Primeira confirmação: deseja alterar?
    Confirm,
    /// Segunda confirmação: tem certeza?
    ConfirmAgain,
    /// Confirmação de saída, aberta pelo `esc` nas tabelas.
    ConfirmQuit,
}

/// Aba do pop-up "Comandos", aberto com `ctrl+p`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommandTab {
    /// Filtra a grade com SQL livre (ex.: `;select COUNT(*) from users`).
    Sql,
}

impl CommandTab {
    /// Todas as abas, na ordem em que aparecem no pop-up.
    pub const ALL: [CommandTab; 1] = [CommandTab::Sql];

    /// Rótulo mostrado na linha de abas.
    pub fn titulo(self) -> &'static str {
        match self {
            CommandTab::Sql => "SQL",
        }
    }

    /// Tecla que abre a aba direto da navegação, sem passar pelo `ctrl+p`.
    pub fn atalho(self) -> char {
        match self {
            CommandTab::Sql => ';',
        }
    }
}

/// Comando master em edição: a aba ativa e o texto digitado.
///
/// Na aba SQL o texto começa com `;`, que marca o modo SQL e não faz parte
/// da consulta enviada ao servidor.
#[derive(Debug)]
pub struct Commands {
    pub tab: CommandTab,
    pub input: String,
}

impl Default for Commands {
    /// Igual a [`Commands::new`]: aba SQL com o `;` inicial.
    fn default() -> Self {
        Self::new()
    }
}

impl Commands {
    /// Abre o pop-up na primeira aba (SQL), já com o `;`.
    pub fn new() -> Self {
        Self {
            tab: CommandTab::Sql,
            input: String::from(";"),
        }
    }

    /// A consulta a executar: o texto sem o `;` inicial e sem espaços nas
    /// pontas. Vazio quando só há o `;`.
    pub fn sql(&self) -> String {
        self.input
            .strip_prefix(';')
            .unwrap_or(self.input.as_str())
            .trim()
            .to_owned()
    }

    /// Digita no fim da linha, onde fica o cursor.
    pub fn type_char(&mut self, caractere: char) {
        self.input.push(caractere);
    }

    /// Apaga o último caractere, sem tirar o `;` inicial da aba SQL.
    pub fn backspace(&mut self) {
        if self.input.len() > 1 {
            self.input.pop();
        }
    }
}

/// Edição em andamento de uma célula.
#[derive(Debug)]
pub struct Edit {
    pub row: usize,
    pub column: usize,
    /// Valor atual, mostrado na confirmação como "De".
    pub original: String,
    pub input: String,
}

/// Edição em andamento de uma conexão salva, aberta pelo `e` nas conexões.
#[derive(Debug)]
pub struct ConnectionEdit {
    /// Índice da conexão em `History::all`.
    pub saved: usize,
    /// Os campos, preenchidos com o que está salvo.
    pub connection: Connection,
    /// Campo em edição no pop-up.
    pub field: Field,
}

/// Lista de opções aberta sobre um pop-up de edição.
#[derive(Debug)]
pub struct Picker<F> {
    /// Campo que a lista está escolhendo.
    pub field: F,
    pub items: Vec<String>,
    /// Item destacado.
    pub index: usize,
}

/// Edição em andamento de um banco, aberta pelo `e` nos bancos.
#[derive(Debug)]
pub struct DatabaseEdit {
    /// Como o banco está no servidor: só o que mudar daqui vira `ALTER`.
    pub original: DatabaseInfo,
    /// Os valores como estão no pop-up.
    pub values: DatabaseInfo,
    /// Campo em edição no pop-up.
    pub field: DatabaseField,
    /// Lista aberta de um dos campos, enquanto o usuário escolhe.
    pub picker: Option<Picker<DatabaseField>>,
}

/// Edição em andamento de uma tabela, aberta pelo `e` nas tabelas.
#[derive(Debug)]
pub struct TableEdit {
    /// Como a tabela está no servidor: só o que mudar daqui vira `ALTER`.
    pub original: TableInfo,
    /// Os valores como estão no pop-up, menos o auto-increment.
    pub values: TableInfo,
    /// Texto do auto-increment: é número, mas se digita.
    pub auto_increment: String,
    /// Campo em edição no pop-up.
    pub field: TableField,
    /// Lista aberta de um dos campos, enquanto o usuário escolhe.
    pub picker: Option<Picker<TableField>>,
}
