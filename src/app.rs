//! Estado da aplicação e o loop de eventos.
//!
//! Regra do MVP: o estado só muda aqui; o desenho (`ui::draw`) é leitura pura.

use std::io;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
    pub edit: Option<Edit>,
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
            mode: Mode::Rows,
            edit: None,
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
                    if let Some(coluna) = self.table_state.selected_column() {
                        let coluna = ui::data_column(coluna).min(colunas.saturating_sub(1));
                        self.table_state
                            .select_column(Some(ui::table_column(coluna)));
                    }
                } else {
                    self.table_state
                        .select(if linhas == 0 { None } else { Some(0) });
                    self.table_state.select_column(None);
                    self.mode = Mode::Rows;
                    self.edit = None;
                }
                self.status = Status::Ready;
            }
            Response::Updated { table } => self.load(table),
            Response::Failed(message) => self.status = Status::Failed(message),
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Rows => self.on_key_rows(key),
            Mode::Columns => self.on_key_columns(key),
            Mode::Edit => self.on_key_edit(key),
            Mode::Confirm => self.on_key_confirm(key),
            Mode::ConfirmAgain => self.on_key_confirm_again(key),
        }
    }

    fn on_key_rows(&mut self, key: KeyEvent) {
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
            KeyCode::Enter => match self.focus {
                Focus::Tables => self.open_selected_table(),
                Focus::Data => self.enter_columns(),
            },
            _ => {}
        }
    }

    fn on_key_columns(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => self.exit_columns(),
            KeyCode::Left | KeyCode::Char('h') => self.move_column(-1),
            KeyCode::Right | KeyCode::Char('l') => self.move_column(1),
            KeyCode::Enter => self.start_edit(),
            _ => {}
        }
    }

    fn on_key_edit(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.edit = None;
                self.mode = Mode::Columns;
            }
            KeyCode::Enter => self.mode = Mode::Confirm,
            KeyCode::Backspace => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.input.pop();
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let Some(edit) = self.edit.as_mut() {
                    edit.input.push(c);
                }
            }
            _ => {}
        }
    }

    fn on_key_confirm(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.mode = Mode::ConfirmAgain,
            KeyCode::Esc => self.mode = Mode::Edit,
            _ => {}
        }
    }

    fn on_key_confirm_again(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.apply_edit(),
            KeyCode::Esc => self.mode = Mode::Edit,
            _ => {}
        }
    }

    /// Entra no modo horizontal, na primeira coluna do registro selecionado.
    fn enter_columns(&mut self) {
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let Some(row) = self.table_state.selected() else {
            return;
        };
        if data.columns.is_empty() || row >= data.rows.len() {
            return;
        }
        self.mode = Mode::Columns;
        self.table_state.select_cell(Some((row, 0)));
    }

    fn exit_columns(&mut self) {
        self.mode = Mode::Rows;
        self.table_state.select_column(None);
    }

    fn move_column(&mut self, delta: isize) {
        let len = self.data.as_ref().map_or(0, |data| data.columns.len());
        if len == 0 {
            return;
        }
        let current = self
            .table_state
            .selected_column()
            .map_or(0, ui::data_column);
        let next = (current as isize + delta).clamp(0, len as isize - 1) as usize;
        self.table_state.select_column(Some(ui::table_column(next)));
    }

    fn start_edit(&mut self) {
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let Some(row) = self.table_state.selected() else {
            return;
        };
        let Some(column) = self.table_state.selected_column().map(ui::data_column) else {
            return;
        };
        if data.primary_key.is_empty() {
            self.status =
                Status::Notice(String::from("tabela sem chave primária: somente leitura"));
            return;
        }
        let Some(original) = data.rows.get(row).and_then(|row| row.get(column)) else {
            return;
        };
        self.edit = Some(Edit {
            row,
            column,
            original: original.clone(),
            input: original.clone(),
        });
        self.mode = Mode::Edit;
    }

    /// Manda o `UPDATE` para o worker e volta ao modo coluna.
    fn apply_edit(&mut self) {
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
        let Some(row) = data.rows.get(edit.row) else {
            return;
        };
        let key: Vec<(String, String)> = data
            .primary_key
            .iter()
            .filter_map(|&index| Some((data.columns.get(index)?.clone(), row.get(index)?.clone())))
            .collect();
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

    fn app_com_grade() -> App {
        let mut app = app();
        app.tables = vec![String::from("users")];
        app.list_state.select(Some(0));
        app.current = Some(String::from("users"));
        app.data = Some(TableData {
            columns: vec![String::from("id"), String::from("name")],
            rows: vec![
                vec![String::from("1"), String::from("ana")],
                vec![String::from("2"), String::from("bia")],
            ],
            primary_key: vec![0],
        });
        app.focus = Focus::Data;
        app.table_state.select(Some(0));
        app.status = Status::Ready;
        app
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
                primary_key: vec![0],
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

    #[test]
    fn enter_entra_no_modo_coluna_e_h_l_anda_na_horizontal() {
        let mut app = app_com_grade();
        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Columns);
        assert_eq!(app.table_state.selected_column(), Some(0));

        app.on_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.table_state.selected_column(), Some(2));
        app.on_key(KeyEvent::from(KeyCode::Right));
        assert_eq!(app.table_state.selected_column(), Some(2), "clampa no fim");
        app.on_key(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.table_state.selected_column(), Some(0));

        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Rows);
        assert_eq!(app.table_state.selected_column(), None);
    }

    #[test]
    fn edicao_passa_por_duas_confirmacoes() {
        let mut app = app_com_grade();
        app.on_key(KeyEvent::from(KeyCode::Enter));
        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Edit);
        let edit = app.edit.as_ref().expect("edição aberta");
        assert_eq!(edit.original, "1");
        assert_eq!(edit.input, "1");

        app.on_key(KeyEvent::from(KeyCode::Backspace));
        app.on_key(KeyEvent::from(KeyCode::Char('9')));
        assert_eq!(app.edit.as_ref().unwrap().input, "9");

        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Confirm);
        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::ConfirmAgain);
        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Columns);
        assert!(app.edit.is_none());
    }

    #[test]
    fn esc_na_confirmacao_volta_a_editar() {
        let mut app = app_com_grade();
        app.on_key(KeyEvent::from(KeyCode::Enter));
        app.on_key(KeyEvent::from(KeyCode::Enter));
        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Confirm);

        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Edit);
        assert!(app.edit.is_some());
    }

    #[test]
    fn tabela_sem_chave_primaria_nao_abre_edicao() {
        let mut app = app_com_grade();
        if let Some(data) = app.data.as_mut() {
            data.primary_key.clear();
        }
        app.on_key(KeyEvent::from(KeyCode::Enter));
        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Columns);
        assert!(app.edit.is_none());
        assert!(matches!(app.status, Status::Notice(_)));
    }

    #[test]
    fn recarregar_a_mesma_tabela_preserva_linha_e_coluna() {
        let mut app = app_com_grade();
        app.on_key(KeyEvent::from(KeyCode::Enter));
        app.on_key(KeyEvent::from(KeyCode::Char('l')));
        app.table_state.select(Some(1));

        app.on_response(Response::Rows {
            table: String::from("users"),
            data: TableData {
                columns: vec![String::from("id"), String::from("name")],
                rows: vec![
                    vec![String::from("1"), String::from("ana")],
                    vec![String::from("2"), String::from("bia")],
                ],
                primary_key: vec![0],
            },
        });
        assert_eq!(app.table_state.selected(), Some(1));
        assert_eq!(app.table_state.selected_column(), Some(2));
        assert_eq!(app.mode, Mode::Columns);
    }

    #[test]
    fn navegacao_horizontal_nao_para_nas_divisorias() {
        let mut app = app_com_grade();
        if let Some(data) = app.data.as_mut() {
            data.columns = ["id", "name", "email"]
                .iter()
                .map(|nome| nome.to_string())
                .collect();
            data.rows = vec![
                ["1", "ana", "ana@exemplo.com"]
                    .iter()
                    .map(|valor| valor.to_string())
                    .collect(),
            ];
        }
        app.on_key(KeyEvent::from(KeyCode::Enter));

        let mut vistos = Vec::new();
        for _ in 0..3 {
            app.on_key(KeyEvent::from(KeyCode::Char('l')));
            vistos.push(app.table_state.selected_column().unwrap());
        }
        // 0 → coluna de dados 1 → coluna de dados 2 → clampa; nunca um ímpar.
        assert_eq!(vistos, vec![2, 4, 4]);
        assert_eq!(
            app.table_state
                .selected_column()
                .map(crate::ui::data_column),
            Some(2)
        );
    }

    #[test]
    fn abrir_outra_tabela_reseta_a_selecao_e_o_modo() {
        let mut app = app_com_grade();
        app.on_key(KeyEvent::from(KeyCode::Enter));
        app.on_key(KeyEvent::from(KeyCode::Char('l')));

        app.on_response(Response::Rows {
            table: String::from("orders"),
            data: TableData {
                columns: vec![String::from("id")],
                rows: vec![vec![String::from("7")]],
                primary_key: vec![0],
            },
        });
        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(app.table_state.selected_column(), None);
        assert_eq!(app.mode, Mode::Rows);
    }
}
