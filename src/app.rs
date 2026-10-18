//! Estado da aplicação e o loop de eventos.
//!
//! Regra do MVP: o estado só muda aqui; o desenho (`ui::draw`) é leitura pura.

use std::io;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::{ListState, TableState};

use crate::db::{Db, Request, Response, TableData};
use crate::ui;

/// Espera máxima por um evento antes de redesenhar (e drenar o banco).
const TICK: Duration = Duration::from_millis(50);
/// Quantas linhas PgUp/PgDn andam.
const PAGE: isize = 20;

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
    Failed(String),
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
    pub should_quit: bool,
    pub list_state: ListState,
    pub table_state: TableState,
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
            should_quit: false,
            list_state: ListState::default(),
            table_state: TableState::default(),
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
                self.current = Some(table);
                self.data = Some(data);
                self.table_state.select(Some(0));
                self.status = Status::Ready;
            }
            Response::Failed(message) => self.status = Status::Failed(message),
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Tables => Focus::Data,
                    Focus::Data => Focus::Tables,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(PAGE),
            KeyCode::PageUp => self.move_selection(-PAGE),
            KeyCode::Char('g') => self.select_edge(false),
            KeyCode::Char('G') => self.select_edge(true),
            KeyCode::Char('r') => self.reload(),
            KeyCode::Enter => self.open_selected_table(),
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Tables => {
                let len = self.tables.len();
                let next = step(self.list_state.selected(), delta, len);
                self.list_state.select(next);
            }
            Focus::Data => {
                let len = self.row_count();
                let next = step(self.table_state.selected(), delta, len);
                self.table_state.select(next);
            }
        }
    }

    fn select_edge(&mut self, last: bool) {
        match self.focus {
            Focus::Tables => {
                if !self.tables.is_empty() {
                    self.list_state
                        .select(Some(if last { self.tables.len() - 1 } else { 0 }));
                }
            }
            Focus::Data => {
                let len = self.row_count();
                if len > 0 {
                    self.table_state
                        .select(Some(if last { len - 1 } else { 0 }));
                }
            }
        }
    }

    fn open_selected_table(&mut self) {
        if let Some(table) = self.list_state.selected().and_then(|i| self.tables.get(i)) {
            self.load(table.clone());
        }
    }

    fn reload(&mut self) {
        if let Some(table) = self.current.clone() {
            self.load(table);
        }
    }

    fn load(&mut self, table: String) {
        self.status = Status::Loading(table.clone());
        self.db.request(Request::Rows { table });
    }

    pub fn row_count(&self) -> usize {
        self.data.as_ref().map_or(0, |data| data.rows.len())
    }
}

/// Move um índice clampado, tratando o caso "nada selecionado ainda".
fn step(current: Option<usize>, delta: isize, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let Some(current) = current else {
        return Some(0);
    };
    Some((current as isize + delta).clamp(0, len as isize - 1) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new("mysql://user@localhost/app")
    }

    #[test]
    fn step_clampa_nos_limites() {
        assert_eq!(step(None, 1, 0), None);
        assert_eq!(step(None, -1, 3), Some(0));
        assert_eq!(step(Some(0), -1, 3), Some(0));
        assert_eq!(step(Some(2), 1, 3), Some(2));
        assert_eq!(step(Some(1), PAGE, 3), Some(2));
        assert_eq!(step(Some(1), 1, 3), Some(2));
    }

    #[test]
    fn conectado_mostra_servidor_e_schema() {
        let mut app = app();
        app.on_response(Response::Connected {
            server: String::from("MySQL 8.0.35"),
            database: String::from("app"),
        });
        assert_eq!(app.server, "MySQL 8.0.35");
        assert_eq!(app.database, "app");
    }

    #[test]
    fn lista_de_tabelas_seleciona_a_primeira() {
        let mut app = app();
        app.on_response(Response::Tables(vec![String::from("users")]));
        assert_eq!(app.list_state.selected(), Some(0));
        assert!(matches!(app.status, Status::Loading(_)));
    }

    #[test]
    fn resposta_de_linhas_preenche_a_grade() {
        let mut app = app();
        app.on_response(Response::Rows {
            table: String::from("users"),
            data: TableData {
                columns: vec![String::from("id")],
                rows: vec![vec![String::from("1")]],
            },
        });
        assert_eq!(app.current.as_deref(), Some("users"));
        assert_eq!(app.row_count(), 1);
        assert_eq!(app.table_state.selected(), Some(0));
        assert!(matches!(app.status, Status::Ready));
    }

    #[test]
    fn falha_vira_status_de_erro() {
        let mut app = app();
        app.on_response(Response::Failed(String::from("sem permissão")));
        assert!(matches!(app.status, Status::Failed(message) if message == "sem permissão"));
    }

    #[test]
    fn teclas_navegam_selecionam_e_saem() {
        let mut app = app();
        app.tables = vec![String::from("a"), String::from("b")];
        app.list_state.select(Some(0));

        app.on_key(KeyEvent::from(KeyCode::Char('j')));
        assert_eq!(app.list_state.selected(), Some(1));
        app.on_key(KeyEvent::from(KeyCode::Down));
        assert_eq!(app.list_state.selected(), Some(1), "clampa no fim");
        app.on_key(KeyEvent::from(KeyCode::Char('g')));
        assert_eq!(app.list_state.selected(), Some(0));
        app.on_key(KeyEvent::from(KeyCode::Char('G')));
        assert_eq!(app.list_state.selected(), Some(1));

        app.on_key(KeyEvent::from(KeyCode::Tab));
        assert!(app.focus == Focus::Data);

        app.on_key(KeyEvent::from(KeyCode::Char('q')));
        assert!(app.should_quit);
    }
}
