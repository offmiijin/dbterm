//! Estado da aplicação e o loop de eventos.
//!
//! Regra do MVP: o estado só muda aqui; o desenho (`ui::draw`) é leitura pura.
//! O mapa de teclas fica em `input`, os tipos em `state`, o comportamento dos
//! pop-ups em `edits`, os pedidos ao banco em `ops` e as respostas em
//! `responses`.
//!
//! Exceção: o `TableState` guarda a rolagem e a coluna do widget, e o `Table`
//! as atualiza ao desenhar.

mod edits;
mod input;
mod ops;
mod responses;
mod state;
#[cfg(test)]
mod tests;

pub use state::{
    CommandTab, Commands, ConnectionEdit, DatabaseEdit, Edit, Focus, Mode, Picker, QueryInstance,
    RightTab, Screen, Status, TableEdit,
};

use std::io;
use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::widgets::{ListState, TableState};

use crate::config::{Connection, Field};
use crate::db::{Db, Request, TableData};
use crate::history::History;
use crate::ui;

/// Espera máxima por um evento antes de redesenhar (e drenar o banco).
const TICK: Duration = Duration::from_millis(50);

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
    /// Bancos do servidor conectado.
    pub databases: Vec<String>,
    pub tables: Vec<String>,
    /// Tabela cujos dados estão em `data`.
    pub current: Option<String>,
    /// Base do SQL paginado que gerou `data`, quando a grade veio de `;`.
    /// `None` na navegação por tabela; sair da consulta é abrir uma tabela.
    pub query_sql: Option<String>,
    pub data: Option<TableData>,
    pub focus: Focus,
    pub status: Status,
    pub mode: Mode,
    /// Modo para o qual a confirmação de saída volta se cancelada.
    pub quit_return: Mode,
    pub edit: Option<Edit>,
    /// Edição da conexão selecionada, aberta pelo `e` nas conexões.
    pub connection_edit: Option<ConnectionEdit>,
    /// Edição do banco selecionado, aberta pelo `e` nos bancos.
    pub database_edit: Option<DatabaseEdit>,
    /// Edição da tabela selecionada, aberta pelo `e` nas tabelas.
    pub table_edit: Option<TableEdit>,
    /// Comando master, aberto com `ctrl+p`. A primeira aba é a SQL.
    pub commands: Option<Commands>,
    /// Último comando do pop-up: reabrir mostra o texto de volta, na aba do
    /// prefixo (`;` é SQL, `#` é busca de tabela).
    pub commands_buffer: String,
    /// Aba ativa da lateral direita (hoje só queries).
    pub right_tab: RightTab,
    /// Instâncias de query criadas na aba da direita.
    pub queries: Vec<QueryInstance>,
    /// Seleção na lista de queries.
    pub queries_state: ListState,
    /// Próximo contador de instância (`q1`, `q2`, ...).
    pub next_query: usize,
    /// Instância aberta no centro: o editor substitui a grade enquanto
    /// houver uma aqui. Abrir uma tabela limpa e volta à grade.
    pub active_query: Option<usize>,
    pub should_quit: bool,
    /// Seleção nas listas de conexões, bancos e tabelas. Só a do nível ativo
    /// aparece, mas cada uma guarda onde o usuário parou.
    pub connections_state: ListState,
    pub databases_state: ListState,
    pub list_state: ListState,
    pub table_state: TableState,
    /// Índice da coluna de dados selecionada na grade. O widget intercala
    /// divisórias entre as colunas; esse detalhe fica em `ui`.
    pub column: Option<usize>,
    /// Primeira coluna que a grade tenta mostrar: a rolagem horizontal das
    /// tabelas com mais colunas do que cabe na tela. Quem ajusta à largura e à
    /// coluna selecionada é `ui`.
    pub hscroll: usize,
    /// Largura natural de cada coluna de `data`, medida quando os dados mudam:
    /// medir a cada quadro varreria todas as linhas carregadas.
    pub larguras: Vec<usize>,
    /// A tabela acabou: a última página veio com menos linhas que o pedido.
    pub end_reached: bool,
    /// Há uma página a caminho: não vale pedir de novo a cada tecla.
    pub loading_more: bool,
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
            databases: Vec::new(),
            tables: Vec::new(),
            current: None,
            query_sql: None,
            data: None,
            focus: Focus::Databases,
            status: Status::Connecting,
            mode: Mode::Rows,
            quit_return: Mode::Rows,
            edit: None,
            connection_edit: None,
            database_edit: None,
            table_edit: None,
            commands: None,
            commands_buffer: String::new(),
            right_tab: RightTab::Queries,
            queries: Vec::new(),
            queries_state: ListState::default(),
            next_query: 1,
            active_query: None,
            should_quit: false,
            connections_state: ListState::default(),
            databases_state: ListState::default(),
            list_state: ListState::default(),
            table_state: TableState::default(),
            column: None,
            hscroll: 0,
            larguras: Vec::new(),
            end_reached: false,
            loading_more: false,
            db,
        };
        if let Some(url) = url {
            // Os bancos são pedidos quando o `Connected` chega; o worker atende
            // na ordem da fila.
            app.db.request(Request::Connect {
                url: url.to_owned(),
            });
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
}
