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

use crate::config::{Connection, DatabaseField, Field};
use crate::db::{
    Collation, DatabaseAlter, DatabaseInfo, Db, ROW_LIMIT, Request, Response, TableData,
};
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
    pub picker: Option<Picker>,
}

/// Lista de opções aberta sobre o pop-up do banco.
#[derive(Debug)]
pub struct Picker {
    /// Campo que a lista está escolhendo.
    pub field: DatabaseField,
    pub items: Vec<String>,
    /// Item destacado.
    pub index: usize,
}

impl DatabaseEdit {
    pub fn new(info: DatabaseInfo) -> Self {
        Self {
            original: info.clone(),
            values: info,
            field: DatabaseField::Charset,
            picker: None,
        }
    }

    /// Campos do pop-up na ordem de tabulação: os que o servidor informa.
    pub fn fields(&self) -> Vec<DatabaseField> {
        DatabaseField::ALL
            .into_iter()
            .filter(|field| self.available(*field))
            .collect()
    }

    /// O servidor informa este campo? A opção de somente leitura só existe da
    /// 8.0.22 em diante.
    pub fn available(&self, field: DatabaseField) -> bool {
        match field {
            DatabaseField::Charset | DatabaseField::Collation => true,
            DatabaseField::ReadOnly => self.values.read_only.is_some(),
        }
    }

    /// Valor do campo como aparece no pop-up.
    pub fn value(&self, field: DatabaseField) -> String {
        match field {
            DatabaseField::Charset => self.values.charset.clone(),
            DatabaseField::Collation => self.values.collation.clone(),
            DatabaseField::ReadOnly => sim_nao(self.values.read_only),
        }
    }

    /// Collations que combinam com o charset escolhido.
    pub fn collations(&self) -> impl Iterator<Item = &Collation> {
        self.values
            .collations
            .iter()
            .filter(|collation| collation.charset == self.values.charset)
    }

    /// Opções do campo, na ordem em que aparecem na lista.
    pub fn options(&self, field: DatabaseField) -> Vec<String> {
        match field {
            DatabaseField::Charset => self
                .values
                .charsets
                .iter()
                .map(|charset| charset.name.clone())
                .collect(),
            DatabaseField::Collation => self
                .collations()
                .map(|collation| collation.name.clone())
                .collect(),
            DatabaseField::ReadOnly => Vec::new(),
        }
    }

    /// Espaço no campo com foco: abre a lista do campo ou, no sim/não, alterna.
    pub fn activate(&mut self) {
        match self.field {
            DatabaseField::ReadOnly => {
                self.values.read_only = self.values.read_only.map(|valor| !valor);
            }
            campo => {
                let items = self.options(campo);
                if items.is_empty() {
                    return;
                }
                // A lista abre no valor atual, que pode estar em qualquer ponto.
                let atual = self.value(campo);
                let index = items.iter().position(|item| *item == atual).unwrap_or(0);
                self.picker = Some(Picker {
                    field: campo,
                    items,
                    index,
                });
            }
        }
    }

    /// Aplica na escolha da lista o valor do campo.
    pub fn choose(&mut self, field: DatabaseField, item: String) {
        match field {
            DatabaseField::Charset => {
                self.values.charset = item;
                // A collation acompanha o charset: vai para a padrão dele.
                if let Some(charset) = self
                    .values
                    .charsets
                    .iter()
                    .find(|charset| charset.name == self.values.charset)
                {
                    self.values.collation = charset.default_collation.clone();
                }
            }
            DatabaseField::Collation => self.values.collation = item,
            DatabaseField::ReadOnly => {}
        }
    }
}

/// "sim"/"não" das opções; "—" quando o servidor não informa.
fn sim_nao(valor: Option<bool>) -> String {
    match valor {
        Some(true) => String::from("sim"),
        Some(false) => String::from("não"),
        None => String::from("—"),
    }
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
    /// Bancos do servidor conectado.
    pub databases: Vec<String>,
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
    /// Edição da conexão selecionada, aberta pelo `e` nas conexões.
    pub connection_edit: Option<ConnectionEdit>,
    /// Edição do banco selecionado, aberta pelo `e` nos bancos.
    pub database_edit: Option<DatabaseEdit>,
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
            data: None,
            focus: Focus::Databases,
            status: Status::Connecting,
            mode: Mode::Rows,
            quit_return: Mode::Rows,
            edit: None,
            connection_edit: None,
            database_edit: None,
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

    fn on_response(&mut self, response: Response) {
        match response {
            Response::Connected { server, database } => {
                self.server = server;
                self.database = database;
                if self.screen == Screen::Connect {
                    // Só sai do formulário quando a conexão foi aceita, e a
                    // conexão usada sobe para o topo das rápidas.
                    self.connecting = false;
                    self.history.touch(Saved::from_form(&self.connection));
                    self.screen = Screen::Browse;
                    self.connections_state.select(Some(0));
                }
                // Conexão escolhida: a cascata desce para os bancos dela.
                self.focus = Focus::Databases;
                self.status = Status::Loading(String::from("bancos"));
                self.db.request(Request::Databases);
            }
            Response::Databases(databases) => {
                self.databases = databases;
                self.status = Status::Ready;
                // Abre o banco que já estava em uso ou, sem ele, o primeiro.
                let escolhido = self
                    .databases
                    .iter()
                    .position(|database| *database == self.database)
                    .or_else(|| (!self.databases.is_empty()).then_some(0));
                self.databases_state.select(escolhido);
                if let Some(database) = escolhido.and_then(|i| self.databases.get(i)).cloned() {
                    self.use_database(database);
                }
            }
            Response::DatabaseInfo(info) => {
                self.status = Status::Ready;
                self.database_edit = Some(DatabaseEdit::new(info));
            }
            Response::DatabaseUpdated { database } => {
                self.status = Status::Notice(format!("banco {database} atualizado"));
            }
            Response::Database(database) => {
                self.database = database;
                self.status = Status::Loading(String::from("tabelas"));
                self.db.request(Request::Tables);
            }
            Response::Tables(tables) => {
                self.tables = tables;
                self.status = Status::Ready;
                if self.tables.is_empty() {
                    self.list_state = ListState::default();
                } else {
                    self.list_state.select(Some(0));
                    // Abrir a primeira tabela deixa a tela útil de imediato.
                    self.load(self.tables[0].clone());
                }
            }
            Response::Rows {
                table,
                offset,
                limit,
                data,
            } => {
                if offset > 0 {
                    // Página seguinte: só junta ao que já está na tela.
                    self.append_page(table, limit, data);
                    return;
                }
                let mesma_tabela = self.current.as_deref() == Some(table.as_str());
                let linhas = data.rows.len();
                let colunas = data.columns.len();
                self.current = Some(table);
                self.larguras = ui::larguras(&data);
                self.data = Some(data);
                self.end_reached = linhas < limit;
                self.loading_more = false;
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
                // Uma página que falhou não pode deixar a próxima travada.
                self.loading_more = false;
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

    /// Junta a página seguinte à grade, se ainda for a mesma tabela.
    fn append_page(&mut self, table: String, limit: usize, data: TableData) {
        self.loading_more = false;
        if self.current.as_deref() != Some(table.as_str()) {
            // A tabela trocou enquanto a página vinha: ela não serve mais.
            return;
        }
        self.end_reached = data.rows.len() < limit;
        // A página nova pode trazer valores mais largos que os já vistos.
        for (largura, nova) in self.larguras.iter_mut().zip(ui::larguras(&data)) {
            *largura = (*largura).max(nova);
        }
        if let Some(atual) = self.data.as_mut() {
            atual.rows.extend(data.rows);
        }
        self.status = Status::Ready;
    }

    /// Pede a página seguinte quando a seleção encosta na última linha carregada.
    pub(super) fn load_next_page(&mut self) {
        if self.loading_more || self.end_reached {
            return;
        }
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let Some(selecionada) = self.table_state.selected() else {
            return;
        };
        if selecionada + 1 < data.rows.len() {
            return;
        }
        let Some(table) = self.current.clone() else {
            return;
        };
        self.loading_more = true;
        self.status = Status::Loading(table.clone());
        self.db.request(Request::Rows {
            table,
            offset: data.rows.len(),
            limit: ROW_LIMIT,
        });
    }

    /// Abre a conexão selecionada na lista de conexões salvas.
    pub(super) fn open_selected_connection(&mut self) {
        let Some(saved) = self
            .connections_state
            .selected()
            .and_then(|index| self.history.all().get(index))
            .cloned()
        else {
            return;
        };
        let url = match saved.url() {
            Ok(url) => url,
            Err(message) => {
                self.status = Status::Failed(message);
                return;
            }
        };
        self.reset_levels();
        self.status = Status::Connecting;
        self.db.request(Request::Connect { url });
    }

    /// Enter na lista de bancos: troca o schema atual; no banco que já está
    /// aberto, desce para as tabelas.
    pub(super) fn use_selected_database(&mut self) {
        let Some(database) = self
            .databases_state
            .selected()
            .and_then(|index| self.databases.get(index))
            .cloned()
        else {
            return;
        };
        if database == self.database && self.data.is_some() {
            self.focus = Focus::Tables;
            return;
        }
        self.use_database(database);
    }

    /// Troca o banco atual pelo `USE`, na mesma conexão.
    fn use_database(&mut self, database: String) {
        self.clear_levels();
        self.status = Status::Loading(database.clone());
        self.db.request(Request::UseDatabase { database });
    }

    /// Esquece tabelas, grade e seleções de uma conexão ou banco que saiu.
    fn reset_levels(&mut self) {
        self.databases.clear();
        self.databases_state = ListState::default();
        self.clear_levels();
    }

    /// Esquece tabelas, grade e seleções, preservando a lista de bancos.
    fn clear_levels(&mut self) {
        self.tables.clear();
        self.list_state = ListState::default();
        self.current = None;
        self.data = None;
        self.table_state = TableState::default();
        self.column = None;
        self.hscroll = 0;
        self.larguras.clear();
        self.end_reached = false;
        self.loading_more = false;
        self.mode = Mode::Rows;
        self.edit = None;
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
        else {
            return;
        };
        self.connection = saved.form();
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

    /// Abre o pop-up "Editar" com a conexão selecionada, senha inclusa.
    pub(super) fn open_connection_edit(&mut self) {
        let Some(index) = self.connections_state.selected() else {
            return;
        };
        let Some(saved) = self.history.all().get(index) else {
            return;
        };
        self.connection_edit = Some(ConnectionEdit {
            saved: index,
            connection: saved.form(),
            field: Field::Host,
        });
    }

    /// Grava a conexão editada no histórico, no mesmo lugar da lista. Um erro
    /// do formulário mantém o pop-up aberto para a correção.
    pub(super) fn save_connection_edit(&mut self) {
        let Some(edit) = self.connection_edit.as_ref() else {
            return;
        };
        if let Err(message) = edit.connection.url() {
            self.status = Status::Failed(message);
            return;
        }
        let saved = Saved::from_form(&edit.connection);
        let index = edit.saved;
        self.connection_edit = None;
        self.history.replace(index, saved);
    }

    /// `e` nos bancos: pede ao servidor as propriedades do banco selecionado;
    /// o pop-up abre quando a resposta chega.
    pub(super) fn request_database_edit(&mut self) {
        let Some(database) = self
            .databases_state
            .selected()
            .and_then(|index| self.databases.get(index))
            .cloned()
        else {
            return;
        };
        self.status = Status::Loading(format!("banco {database}"));
        self.db.request(Request::DatabaseInfo { database });
    }

    /// Aplica no servidor só o que mudou no banco.
    pub(super) fn save_database_edit(&mut self) {
        let Some(edit) = self.database_edit.as_ref() else {
            return;
        };
        let alter = DatabaseAlter::between(&edit.original, &edit.values);
        let database = edit.values.name.clone();
        self.database_edit = None;
        if alter.is_empty() {
            return;
        }
        self.status = Status::Loading(format!("banco {database}"));
        self.db.request(Request::AlterDatabase { database, alter });
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

    /// Pede a tabela do começo; recarregar a mesma mantém as páginas já
    /// carregadas, que só se perdem ao abrir outra tabela.
    pub(super) fn load(&mut self, table: String) {
        self.status = Status::Loading(table.clone());
        self.loading_more = false;
        self.end_reached = false;
        let limit = self.page_limit(&table);
        self.db.request(Request::Rows {
            table,
            offset: 0,
            limit,
        });
    }

    /// Quantas linhas pedir ao carregar `table`: a tabela que já está na tela
    /// mantém o tanto que já veio; uma tabela nova começa com uma página.
    fn page_limit(&self, table: &str) -> usize {
        if self.current.as_deref() == Some(table) {
            self.row_count().max(ROW_LIMIT)
        } else {
            ROW_LIMIT
        }
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
