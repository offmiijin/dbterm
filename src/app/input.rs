//! O mapa de teclas: cada [`Mode`] decide o que cada tecla faz.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, Edit, Focus, Mode, Screen, Status};
use crate::config::Field;

/// Quantas linhas PgUp/PgDn andam.
pub(super) const PAGE: isize = 20;

impl App {
    pub(super) fn on_key(&mut self, key: KeyEvent) {
        match self.screen {
            Screen::Quick => self.on_key_quick(key),
            Screen::Connect => self.on_key_connect(key),
            Screen::Browse => self.on_key_browse(key),
        }
    }

    /// Tela inicial: sem campo de texto, então `j`/`k`/`q` são atalhos.
    fn on_key_quick(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('n') => self.new_connection(),
            KeyCode::Enter => self.open_quick(),
            KeyCode::Down | KeyCode::Char('j') => self.move_quick(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_quick(-1),
            _ => {}
        }
    }

    fn on_key_browse(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Rows => self.on_key_rows(key),
            Mode::Columns => self.on_key_columns(key),
            Mode::Edit => self.on_key_edit(key),
            Mode::Confirm => self.on_key_confirm(key),
            Mode::ConfirmAgain => self.on_key_confirm_again(key),
            Mode::ConfirmQuit => self.on_key_confirm_quit(key),
        }
    }

    /// Tela de conexão: nenhuma tecla digita quando o campo é livre, então
    /// `j`/`k`/`q` são texto; a navegação é por `tab` e setas.
    fn on_key_connect(&mut self, key: KeyEvent) {
        if self.connecting {
            // Enquanto a conexão abre, só `esc` volta e abandona a tentativa.
            if key.code == KeyCode::Esc {
                self.connecting = false;
                self.back_to_quick();
            }
            return;
        }
        match key.code {
            KeyCode::Esc => self.back_to_quick(),
            KeyCode::Enter => self.submit_connection(),
            KeyCode::Tab | KeyCode::Down => self.move_field(1),
            KeyCode::BackTab | KeyCode::Up => self.move_field(-1),
            KeyCode::Backspace => {
                self.connection.value_mut(self.field).pop();
                self.connect_error = None;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.connection.value_mut(self.field).push(c);
                self.connect_error = None;
            }
            _ => {}
        }
    }

    /// Anda um campo, dando a volta nas pontas.
    fn move_field(&mut self, delta: isize) {
        let fields = Field::ALL;
        let atual = fields
            .iter()
            .position(|field| *field == self.field)
            .unwrap_or(0);
        let proximo = (atual as isize + delta).rem_euclid(fields.len() as isize) as usize;
        self.field = fields[proximo];
    }

    fn on_key_rows(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.ask_quit(),
            // Nos registros, `esc` volta às tabelas; nas tabelas, pede
            // confirmação antes de sair.
            KeyCode::Esc => match self.focus {
                Focus::Tables => self.ask_quit(),
                Focus::Data => self.focus = Focus::Tables,
            },
            // Trocar de painel é só com `enter`; `esc` volta para as tabelas.
            // `h`/`l` andam na horizontal pelas colunas da grade, que podem não
            // caber todas na tela.
            KeyCode::Left | KeyCode::Char('h') if self.focus == Focus::Data => {
                self.scroll_columns(-1)
            }
            KeyCode::Right | KeyCode::Char('l') if self.focus == Focus::Data => {
                self.scroll_columns(1)
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
            KeyCode::Char('q') => self.ask_quit(),
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

    fn on_key_confirm_quit(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.should_quit = true,
            KeyCode::Esc => self.mode = self.quit_return,
            _ => {}
        }
    }

    /// Abre a confirmação de saída, guardando o modo de onde ela partiu.
    fn ask_quit(&mut self) {
        self.quit_return = self.mode;
        self.mode = Mode::ConfirmQuit;
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
        self.column = Some(0);
    }

    fn exit_columns(&mut self) {
        self.mode = Mode::Rows;
        self.column = None;
    }

    fn move_column(&mut self, delta: isize) {
        let len = self.data.as_ref().map_or(0, |data| data.columns.len());
        if len == 0 {
            return;
        }
        let current = self.column.unwrap_or(0);
        let next = (current as isize + delta).clamp(0, len as isize - 1) as usize;
        self.column = Some(next);
    }

    /// Rola as colunas da grade na horizontal, sem mexer na seleção.
    fn scroll_columns(&mut self, delta: isize) {
        let len = self.data.as_ref().map_or(0, |data| data.columns.len());
        if len == 0 {
            return;
        }
        let atual = self.hscroll.min(len - 1);
        self.hscroll = (atual as isize + delta).clamp(0, len as isize - 1) as usize;
    }

    fn start_edit(&mut self) {
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let Some(row) = self.table_state.selected() else {
            return;
        };
        let Some(column) = self.column else {
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
                self.load_next_page();
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
                    // `G` encosta na última linha carregada: a próxima página vem.
                    self.load_next_page();
                }
            }
        }
    }

    /// Enter na lista: abre a tabela selecionada; se ela já está aberta, o
    /// segundo enter leva direto aos registros (sem precisar de `l`).
    fn open_selected_table(&mut self) {
        let Some(table) = self
            .list_state
            .selected()
            .and_then(|i| self.tables.get(i))
            .cloned()
        else {
            return;
        };
        if self.current.as_deref() == Some(table.as_str()) && self.data.is_some() {
            self.focus = Focus::Data;
            return;
        }
        self.load(table);
    }

    fn reload(&mut self) {
        if let Some(table) = self.current.clone() {
            self.load(table);
        }
    }
}

/// Move um índice clampado, tratando o caso "nada selecionado ainda".
pub(super) fn step(current: Option<usize>, delta: isize, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let Some(current) = current else {
        return Some(0);
    };
    Some((current as isize + delta).clamp(0, len as isize - 1) as usize)
}
