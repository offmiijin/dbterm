//! Estado da aplicação e o loop de eventos.
//!
//! Regra do MVP: o estado só muda aqui; o desenho (`ui::draw`) é leitura pura.
//! O mapa de teclas fica em `input`.
//!
//! Exceção: o `TableState` guarda a rolagem e a coluna do widget, e o `Table`
//! as atualiza ao desenhar.

mod input;
#[cfg(test)]
mod tests;

use std::io;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::widgets::{ListState, TableState};

use crate::db::{Db, Request, Response, TableData};
use crate::ui;

/// Espera máxima por um evento antes de redesenhar (e drenar o banco).
const TICK: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Lista de tabelas à esquerda.
    Tables,
    /// Grade de dados à direita.
    Data,
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

/// Edição em andamento de uma célula.
#[derive(Debug)]
pub struct Edit {
    pub row: usize,
    pub column: usize,
    /// Valor atual, mostrado na confirmação como "De".
    pub original: String,
    pub input: String,
}

pub struct App {
    pub server: String,
    pub database: String,
    pub tables: Vec<String>,
    /// Tabela cujos dados estão em `data`.
    pub current: Option<String>,
    pub data: Option<TableData>,
    pub focus: Focus,
    pub status: Status,
    pub mode: Mode,
    /// Modo para o qual a confirmação de saída volta se cancelada.
    pub quit_return: Mode,
    pub edit: Option<Edit>,
    pub should_quit: bool,
    pub list_state: ListState,
    pub table_state: TableState,
    /// Índice da coluna de dados selecionada na grade. O widget intercala
    /// divisórias entre as colunas; esse detalhe fica em `ui`.
    pub column: Option<usize>,
    db: Db,
}

impl App {
    pub fn new(url: &str) -> Self {
        let db = Db::connect(url);
        db.request(Request::Tables);
        Self {
            server: String::new(),
            database: String::new(),
            tables: Vec::new(),
            current: None,
            data: None,
            focus: Focus::Tables,
            status: Status::Connecting,
            mode: Mode::Rows,
            quit_return: Mode::Rows,
            edit: None,
            should_quit: false,
            list_state: ListState::default(),
            table_state: TableState::default(),
            column: None,
            db,
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::draw(frame, &mut self))?;
            self.drain_db();
            if event::poll(TICK)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key(key),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn drain_db(&mut self) {
        while let Some(response) = self.db.poll() {
            self.on_response(response);
        }
    }

    fn on_response(&mut self, response: Response) {
        match response {
            Response::Connected { server, database } => {
                self.server = server;
                self.database = database;
            }
            Response::Tables(tables) => {
                self.tables = tables;
                self.status = Status::Ready;
                if !self.tables.is_empty() {
                    self.list_state.select(Some(0));
                    // Abrir a primeira tabela deixa a tela útil de imediato.
                    self.load(self.tables[0].clone());
                }
            }
            Response::Rows { table, data } => {
                let mesma_tabela = self.current.as_deref() == Some(table.as_str());
                let linhas = data.rows.len();
                let colunas = data.columns.len();
                self.current = Some(table);
                self.data = Some(data);
                if mesma_tabela && linhas > 0 {
                    // Recarregar a mesma tabela (ex.: depois de editar) não pode
                    // jogar a seleção de volta para a primeira linha.
                    let linha = self.table_state.selected().unwrap_or(0).min(linhas - 1);
                    self.table_state.select(Some(linha));
                    self.column = self
                        .column
                        .map(|coluna| coluna.min(colunas.saturating_sub(1)));
                } else {
                    self.table_state
                        .select(if linhas == 0 { None } else { Some(0) });
                    self.column = None;
                    self.mode = Mode::Rows;
                    self.edit = None;
                }
                self.status = Status::Ready;
            }
            Response::Updated { table } => self.load(table),
            Response::Failed(message) => self.status = Status::Failed(message),
        }
    }

    pub(super) fn load(&mut self, table: String) {
        self.status = Status::Loading(table.clone());
        self.db.request(Request::Rows { table });
    }

    /// Manda o `UPDATE` para o worker e volta ao modo coluna.
    pub(super) fn apply_edit(&mut self) {
        let Some(edit) = self.edit.take() else {
            return;
        };
        self.mode = Mode::Columns;
        let Some(table) = self.current.clone() else {
            return;
        };
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let Some(column) = data.columns.get(edit.column).cloned() else {
            return;
        };
        let key = data.key_of(edit.row);
        if key.is_empty() {
            self.status = Status::Failed(String::from("tabela sem chave primária"));
            return;
        }
        let value = if edit.input.is_empty() {
            None
        } else {
            Some(edit.input)
        };
        self.db.request(Request::Update {
            table: table.clone(),
            column,
            value,
            key,
        });
        self.status = Status::Loading(table);
    }

    pub fn row_count(&self) -> usize {
        self.data.as_ref().map_or(0, |data| data.rows.len())
    }
}
