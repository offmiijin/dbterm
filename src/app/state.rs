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

/// Aba do pop-up "Comandos", aberto com `ctrl+p`, `;` ou `#`. O prefixo do
/// texto é fixo e seleciona o comando: `;` é SQL e `#` é busca de tabela.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CommandTab {
    /// Filtra a grade com SQL livre (ex.: `;select COUNT(*) from users`).
    Sql,
    /// Busca uma tabela pelo nome (ex.: `#products`).
    Table,
}

impl CommandTab {
    /// Todas as abas, na ordem em que aparecem no pop-up.
    pub const ALL: [CommandTab; 2] = [CommandTab::Sql, CommandTab::Table];

    /// Rótulo mostrado na linha de abas.
    pub fn titulo(self) -> &'static str {
        match self {
            CommandTab::Sql => "SQL",
            CommandTab::Table => "Tabela",
        }
    }

    /// Ícone Nerd Font da aba, mostrado antes do rótulo.
    pub fn icone(self) -> char {
        match self {
            // database
            CommandTab::Sql => '\u{f1c0}',
            // table
            CommandTab::Table => '\u{f0ce}',
        }
    }

    /// Prefixo que marca o comando no texto do pop-up: o `;` seleciona o SQL
    /// e o `#`, a busca de tabela.
    pub fn atalho(self) -> char {
        match self {
            CommandTab::Sql => ';',
            CommandTab::Table => '#',
        }
    }

    /// A aba que o prefixo seleciona, se for o de algum comando.
    pub fn from_prefix(prefixo: char) -> Option<CommandTab> {
        Self::ALL
            .iter()
            .find(|aba| aba.atalho() == prefixo)
            .copied()
    }
}

/// Quantas tabelas a caixa de completar mostra.
const MAX_SUGESTOES: usize = 6;

/// Comando master em edição: a aba ativa e o texto digitado.
///
/// O primeiro caractere é o prefixo fixo do comando (`;` é SQL, `#` é busca
/// de tabela) e não faz parte do que é executado: ele nunca é apagado, só
/// trocado ao mudar de aba.
#[derive(Debug)]
pub struct Commands {
    pub tab: CommandTab,
    pub input: String,
    /// Tabela aberta na grade quando o pop-up abriu: é nela que o SQL busca.
    /// Só contexto e exibição — a navegação não muda com o pop-up aberto.
    pub tabela: Option<String>,
}

impl Default for Commands {
    /// Igual a [`Commands::new`]: aba SQL com o `;` inicial.
    fn default() -> Self {
        Self::new()
    }
}

impl Commands {
    /// Abre o pop-up na primeira aba (SQL), já com o `;` fixo.
    pub fn new() -> Self {
        Self::with_input(String::from(";"))
    }

    /// Abre o pop-up direto na aba pedida, só com o prefixo fixo.
    pub fn with_tab(tab: CommandTab) -> Self {
        Self {
            tab,
            input: tab.atalho().to_string(),
            tabela: None,
        }
    }

    /// Reabre o pop-up com o texto do buffer (o último comando executado). A
    /// aba acompanha o prefixo do texto; sem prefixo, volta ao `;` do SQL.
    /// O prefixo é garantido: texto sem prefixo ganha o `;` na frente.
    pub fn with_input(input: String) -> Self {
        if input.is_empty() {
            return Self::with_tab(CommandTab::Sql);
        }
        let tab = input
            .chars()
            .next()
            .and_then(CommandTab::from_prefix)
            .unwrap_or(CommandTab::Sql);
        let input = match input.chars().next() {
            Some(primeiro) if CommandTab::from_prefix(primeiro).is_some() => input,
            _ => format!("{}{input}", CommandTab::Sql.atalho()),
        };
        Self {
            tab,
            input,
            tabela: None,
        }
    }

    /// Reescreve o prefixo do texto para o da aba ativa, levando o resto
    /// junto: é o que o `tab` faz ao trocar de aba.
    pub fn apply_prefix(&mut self) {
        let prefixo = self.tab.atalho();
        let resto = match self.input.chars().next() {
            Some(primeiro) if CommandTab::from_prefix(primeiro).is_some() => {
                self.input[primeiro.len_utf8()..].to_owned()
            }
            _ => self.input.clone(),
        };
        self.input = format!("{prefixo}{resto}");
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

    /// A busca de tabela a executar: o texto sem o `#` inicial e sem espaços
    /// nas pontas. Vazio quando só há o `#`.
    pub fn table_query(&self) -> String {
        self.input
            .strip_prefix('#')
            .unwrap_or(self.input.as_str())
            .trim()
            .to_owned()
    }

    /// Índices das tabelas que completam a busca, na ordem da lista. Sem o
    /// `#`, a caixa mostra o começo da lista.
    pub fn suggestions(&self, tables: &[String]) -> Vec<usize> {
        if self.tab != CommandTab::Table {
            return Vec::new();
        }
        let busca = self.table_query().to_lowercase();
        tables
            .iter()
            .enumerate()
            .filter(|(_, tabela)| tabela.to_lowercase().contains(&busca))
            .take(MAX_SUGESTOES)
            .map(|(indice, _)| indice)
            .collect()
    }

    /// Digita no fim da linha, onde fica o cursor. Com só o prefixo na linha,
    /// digitar o prefixo de outro comando troca de aba em vez de acumular
    /// (ex.: `;` vira `#` e cai na Tabela); no resto, o prefixo é fixo e a
    /// aba só muda pelo `tab`.
    pub fn type_char(&mut self, caractere: char) {
        if self.input.chars().count() <= 1
            && let Some(tab) = CommandTab::from_prefix(caractere)
        {
            self.tab = tab;
            self.apply_prefix();
            return;
        }
        self.input.push(caractere);
    }

    /// Apaga o último caractere, sem nunca remover o prefixo fixo.
    pub fn backspace(&mut self) {
        if self.input.chars().count() <= 1 {
            return;
        }
        self.input.pop();
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
