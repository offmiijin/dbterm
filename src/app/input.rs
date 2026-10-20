//! O mapa de teclas: cada [`Mode`] decide o que cada tecla faz.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;
use super::state::{Edit, Focus, Mode, Picker, Screen, Status};
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
        // Os pop-ups ficam por cima de tudo: enquanto um está aberto, as
        // teclas são dele.
        if self.connection_edit.is_some() {
            self.on_key_connection_edit(key);
            return;
        }
        if self.database_edit.is_some() {
            self.on_key_database_edit(key);
            return;
        }
        if self.table_edit.is_some() {
            self.on_key_table_edit(key);
            return;
        }
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
            KeyCode::Tab | KeyCode::Down => self.move_field(1, &Field::ALL),
            KeyCode::BackTab | KeyCode::Up => self.move_field(-1, &Field::ALL),
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

    /// Anda um campo da lista, dando a volta nas pontas.
    fn move_field(&mut self, delta: isize, campos: &[Field]) {
        self.field = next(self.field, delta, campos);
    }

    /// Pop-up "Editar" da conexão: `tab` anda nos campos, `ctrl+h/j/k/l`
    /// também, e `s` grava.
    fn on_key_connection_edit(&mut self, key: KeyEvent) {
        if let Some(delta) = ctrl_dir(key) {
            self.move_edit_field(delta);
            return;
        }
        match key.code {
            KeyCode::Esc => self.connection_edit = None,
            KeyCode::Char('s') => self.save_connection_edit(),
            KeyCode::Tab | KeyCode::Down => self.move_edit_field(1),
            KeyCode::BackTab | KeyCode::Up => self.move_edit_field(-1),
            KeyCode::Backspace => {
                if let Some(edit) = self.connection_edit.as_mut() {
                    let field = edit.field;
                    edit.connection.value_mut(field).pop();
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let Some(edit) = self.connection_edit.as_mut() {
                    let field = edit.field;
                    edit.connection.value_mut(field).push(c);
                }
            }
            _ => {}
        }
    }

    /// Anda entre os campos do pop-up, na ordem em que aparecem na tela.
    fn move_edit_field(&mut self, delta: isize) {
        let Some(edit) = self.connection_edit.as_mut() else {
            return;
        };
        edit.field = next(edit.field, delta, &Field::EDICAO);
    }

    /// Pop-up "Editar" do banco: `j`/`k` andam nos campos, `enter` abre a lista
    /// do campo (ou alterna o sim/não), `s` grava, `esc` descarta. `ctrl+h/j/k/l`
    /// também andam nos campos.
    fn on_key_database_edit(&mut self, key: KeyEvent) {
        // A lista aberta fica por cima do pop-up e recebe as teclas.
        if let Some(edit) = self.database_edit.as_mut()
            && edit.picker.is_some()
        {
            if let Some((campo, item)) = picker_key(&mut edit.picker, key) {
                edit.choose(campo, item);
            }
            return;
        }
        if let Some(delta) = ctrl_dir(key) {
            self.move_database_field(delta);
            return;
        }
        match key.code {
            KeyCode::Esc => self.database_edit = None,
            KeyCode::Char('s') => self.save_database_edit(),
            KeyCode::Enter => {
                if let Some(edit) = self.database_edit.as_mut() {
                    edit.activate();
                }
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => self.move_database_field(1),
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => self.move_database_field(-1),
            _ => {}
        }
    }

    /// Pop-up "Editar" da tabela: como no banco, `enter` abre a lista do campo
    /// e `s` grava — só que, nos campos digitados (`tabela`, `comentário`,
    /// `auto-increment`), `enter` grava e `s` é texto. `ctrl+h/j/k/l` andam nos
    /// campos.
    fn on_key_table_edit(&mut self, key: KeyEvent) {
        // A lista aberta fica por cima do pop-up e recebe as teclas.
        if let Some(edit) = self.table_edit.as_mut()
            && edit.picker.is_some()
        {
            if let Some((campo, item)) = picker_key(&mut edit.picker, key) {
                edit.choose(campo, item);
            }
            return;
        }
        if let Some(delta) = ctrl_dir(key) {
            self.move_table_field(delta);
            return;
        }
        let dropdown = self
            .table_edit
            .as_ref()
            .is_some_and(|edit| edit.field.dropdown());
        match key.code {
            KeyCode::Esc => self.table_edit = None,
            KeyCode::Char('s') if dropdown => self.save_table_edit(),
            KeyCode::Enter if dropdown => {
                if let Some(edit) = self.table_edit.as_mut() {
                    edit.activate();
                }
            }
            KeyCode::Enter => self.save_table_edit(),
            KeyCode::Tab | KeyCode::Down => self.move_table_field(1),
            KeyCode::BackTab | KeyCode::Up => self.move_table_field(-1),
            KeyCode::Backspace => {
                if let Some(edit) = self.table_edit.as_mut() {
                    edit.backspace();
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let Some(edit) = self.table_edit.as_mut() {
                    edit.type_char(c);
                }
            }
            _ => {}
        }
    }

    /// Anda entre os campos da tabela que o servidor informou.
    fn move_table_field(&mut self, delta: isize) {
        let Some(edit) = self.table_edit.as_mut() else {
            return;
        };
        let campos = edit.fields();
        edit.field = next(edit.field, delta, &campos);
    }

    /// Anda entre os campos do banco que o servidor informou.
    fn move_database_field(&mut self, delta: isize) {
        let Some(edit) = self.database_edit.as_mut() else {
            return;
        };
        let campos = edit.fields();
        edit.field = next(edit.field, delta, &campos);
    }

    fn on_key_rows(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.ask_quit(),
            // `esc` sobe um nível da cascata conexões → bancos → tabelas →
            // registros; na raiz (conexões), pede confirmação para sair.
            KeyCode::Esc => self.up_level(),
            // As letras dos chips levam direto ao nível.
            KeyCode::Char('c') => self.focus_level(Focus::Connections),
            KeyCode::Char('b') => self.focus_level(Focus::Databases),
            KeyCode::Char('t') => self.focus_level(Focus::Tables),
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
            // `e` abre o pop-up de edição do nível: a conexão, o banco ou a
            // tabela.
            KeyCode::Char('e') if self.focus == Focus::Connections => self.open_connection_edit(),
            KeyCode::Char('e') if self.focus == Focus::Databases => self.request_database_edit(),
            KeyCode::Char('e') if self.focus == Focus::Tables => self.request_table_edit(),
            KeyCode::Enter => match self.focus {
                Focus::Connections => self.open_selected_connection(),
                Focus::Databases => self.use_selected_database(),
                Focus::Tables => self.open_selected_table(),
                Focus::Data => self.enter_columns(),
            },
            _ => {}
        }
    }

    /// Sobe um nível da cascata; na raiz, pede confirmação para sair.
    fn up_level(&mut self) {
        match self.focus.above() {
            Some(nivel) => self.focus = nivel,
            None => self.ask_quit(),
        }
    }

    /// Leva o foco direto ao nível pedido pelo chip, saindo do modo coluna.
    fn focus_level(&mut self, focus: Focus) {
        if self.focus == Focus::Data {
            self.column = None;
            self.mode = Mode::Rows;
        }
        self.focus = focus;
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
            Focus::Connections => {
                let next = step(
                    self.connections_state.selected(),
                    delta,
                    self.history.all().len(),
                );
                self.connections_state.select(next);
            }
            Focus::Databases => {
                let next = step(self.databases_state.selected(), delta, self.databases.len());
                self.databases_state.select(next);
            }
            Focus::Tables => {
                let len = self.tables.len();
                let next = step(self.list_state.selected(), delta, len);
                self.list_state.select(next);
            }
            Focus::Data => {
                let len = self.row_count();
                let atual = self.table_state.selected();
                let next = step(atual, delta, len);
                self.table_state.select(next);
                // Segurar a tecla no fim da página não pode pedir a página de
                // novo a cada repetição: só quando a seleção andou de verdade.
                if next != atual {
                    self.load_next_page();
                }
            }
        }
    }

    fn select_edge(&mut self, last: bool) {
        match self.focus {
            Focus::Connections => {
                let len = self.history.all().len();
                if len > 0 {
                    self.connections_state
                        .select(Some(if last { len - 1 } else { 0 }));
                }
            }
            Focus::Databases => {
                let len = self.databases.len();
                if len > 0 {
                    self.databases_state
                        .select(Some(if last { len - 1 } else { 0 }));
                }
            }
            Focus::Tables => {
                if !self.tables.is_empty() {
                    self.list_state
                        .select(Some(if last { self.tables.len() - 1 } else { 0 }));
                }
            }
            Focus::Data => {
                let len = self.row_count();
                if len > 0 {
                    let destino = if last { len - 1 } else { 0 };
                    let andou = self.table_state.selected() != Some(destino);
                    self.table_state.select(Some(destino));
                    if andou {
                        self.load_next_page();
                    }
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

/// Próximo item da lista, dando a volta nas pontas; o primeiro quando o atual
/// não está nela.
pub(super) fn next<T: Copy + PartialEq>(atual: T, delta: isize, itens: &[T]) -> T {
    let indice = itens.iter().position(|item| *item == atual).unwrap_or(0);
    itens[(indice as isize + delta).rem_euclid(itens.len() as isize) as usize]
}

/// Direção pedida por `ctrl+h/j/k/l` nos pop-ups de edição: `h`/`k` voltam um
/// campo e `l`/`j` avançam, como as setas. `None` quando a tecla não é uma
/// delas.
fn ctrl_dir(key: KeyEvent) -> Option<isize> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    match key.code {
        KeyCode::Char('h') | KeyCode::Char('k') => Some(-1),
        KeyCode::Char('j') | KeyCode::Char('l') => Some(1),
        _ => None,
    }
}

/// Lista de opções aberta sobre um pop-up: `j`/`k` andam, `enter` escolhe e
/// `esc` volta. Devolve o campo e o item quando a escolha foi confirmada.
fn picker_key<F: Copy>(picker: &mut Option<Picker<F>>, key: KeyEvent) -> Option<(F, String)> {
    // `ctrl+h/j/k/l` anda na lista como `k`/`j`.
    if let Some(delta) = ctrl_dir(key) {
        if let Some(picker) = picker.as_mut() {
            picker.index = step(Some(picker.index), delta, picker.items.len()).unwrap_or(0);
        }
        return None;
    }
    match key.code {
        KeyCode::Esc => {
            *picker = None;
            None
        }
        KeyCode::Enter => picker.take().and_then(|picker| {
            picker
                .items
                .get(picker.index)
                .cloned()
                .map(|item| (picker.field, item))
        }),
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(picker) = picker.as_mut() {
                picker.index = step(Some(picker.index), 1, picker.items.len()).unwrap_or(0);
            }
            None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(picker) = picker.as_mut() {
                picker.index = step(Some(picker.index), -1, picker.items.len()).unwrap_or(0);
            }
            None
        }
        _ => None,
    }
}
