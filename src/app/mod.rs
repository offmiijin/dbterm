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

use crate::config::{Connection, Field};
use crate::db::{Db, Request, Response, TableData};
use crate::history::{History, Saved};
use crate::ui;

/// Espera máxima por um evento antes de redesenhar (e drenar o banco).
const TICK: Duration = Duration::from_millis(50);

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
    pub screen: Screen,
    /// Conexões rápidas salvas, da mais recente para a mais antiga.
    pub history: History,
    /// Índice da conexão rápida selecionada.
    pub quick: Option<usize>,
    /// Formulário da tela de conexão.
    pub connection: Connection,
    /// Campo em edição na tela de conexão.
    pub field: Field,
    /// Verdadeiro enquanto uma conexão pedida na tela está sendo aberta.
    pub connecting: bool,
    /// Falha da última tentativa de conexão pela tela.
    pub connect_error: Option<String>,
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
    /// Primeira coluna que a grade tenta mostrar: a rolagem horizontal das
    /// tabelas com mais colunas do que cabe na tela. Quem ajusta à largura e à
    /// coluna selecionada é `ui`.
    pub hscroll: usize,
    db: Db,
}

impl App {
    /// Abre o app; sem URL, começa nas conexões rápidas.
    pub fn new(url: Option<&str>) -> Self {
        let db = Db::spawn();
        let history = History::load();
        let quick = if history.recent().is_empty() {
            None
        } else {
            Some(0)
        };
        let app = Self {
            screen: if url.is_some() {
                Screen::Browse
            } else {
                Screen::Quick
            },
            history,
            quick,
            connection: Connection::default(),
            field: Field::Host,
            connecting: false,
            connect_error: None,
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
            hscroll: 0,
            db,
        };
        if let Some(url) = url {
            // Os pedidos vão na mesma fila: `Tables` só é lido depois do
            // `Connect` pelo worker.
            app.db.request(Request::Connect {
                url: url.to_owned(),
            });
            app.db.request(Request::Tables);
        }
        app
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
                if self.connecting {
                    // Só sai do formulário quando a conexão foi aceita, e a
                    // conexão usada sobe para o topo das rápidas.
                    self.connecting = false;
                    self.history.touch(Saved::from_form(&self.connection));
                    self.screen = Screen::Browse;
                    self.status = Status::Connecting;
                    self.db.request(Request::Tables);
                }
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
                    self.hscroll = 0;
                    self.mode = Mode::Rows;
                    self.edit = None;
                }
                self.status = Status::Ready;
            }
            Response::Updated { table } => self.load(table),
            Response::Failed(message) => {
                if self.connecting {
                    // Falha ao conectar volta para o formulário com o motivo.
                    self.connecting = false;
                    self.connect_error = Some(message);
                } else {
                    self.status = Status::Failed(message);
                }
            }
        }
    }

    /// Abre o formulário vazio: o `n` das conexões rápidas.
    pub(super) fn new_connection(&mut self) {
        self.connection = Connection::default();
        self.field = Field::Host;
        self.connect_error = None;
        self.screen = Screen::Connect;
    }

    /// Abre o formulário preenchido com a conexão rápida selecionada, senha
    /// inclusa; o foco começa na senha.
    pub(super) fn open_quick(&mut self) {
        let Some(saved) = self
            .quick
            .and_then(|index| self.history.recent().get(index))
            .cloned()
        else {
            return;
        };
        *self.connection.value_mut(Field::Host) = saved.host.clone();
        *self.connection.value_mut(Field::Port) = saved.port.to_string();
        *self.connection.value_mut(Field::User) = saved.user.clone();
        *self.connection.value_mut(Field::Password) = saved.password.clone().unwrap_or_default();
        *self.connection.value_mut(Field::Database) = saved.database.clone();
        self.field = Field::Password;
        self.connect_error = None;
        self.screen = Screen::Connect;
    }

    /// Volta do formulário para as conexões rápidas.
    pub(super) fn back_to_quick(&mut self) {
        self.connect_error = None;
        self.connection.value_mut(Field::Password).clear();
        self.screen = Screen::Quick;
    }

    /// Anda pela lista de conexões rápidas.
    pub(super) fn move_quick(&mut self, delta: isize) {
        self.quick = input::step(self.quick, delta, self.history.recent().len());
    }

    /// Monta a URL do formulário e pede a conexão; erros ficam na tela.
    pub(super) fn submit_connection(&mut self) {
        match self.connection.url() {
            Ok(url) => {
                self.connect_error = None;
                self.connecting = true;
                self.db.request(Request::Connect { url });
            }
            Err(message) => self.connect_error = Some(message),
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
