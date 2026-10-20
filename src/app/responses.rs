//! Respostas do banco: como cada `Response` atualiza o estado.
//!
//! Todo o tratamento de `Response` mora aqui; pedir dados novos mora em `ops`.

use ratatui::widgets::ListState;

use crate::db::{Response, TableData};
use crate::history::Saved;
use crate::ui;

use super::state::{DatabaseEdit, TableEdit};
use super::App;

impl App {
    pub(super) fn on_response(&mut self, response: Response) {
        match response {
            Response::Connected { server, database } => {
                self.server = server;
                self.database = database;
                if self.screen == super::state::Screen::Connect {
                    // Só sai do formulário quando a conexão foi aceita, e a
                    // conexão usada sobe para o topo das rápidas.
                    self.connecting = false;
                    self.history.touch(Saved::from_form(&self.connection));
                    self.screen = super::state::Screen::Browse;
                    self.connections_state.select(Some(0));
                }
                // Conexão escolhida: a cascata desce para os bancos dela.
                self.focus = super::state::Focus::Databases;
                self.status = super::state::Status::Loading(String::from("bancos"));
                self.db.request(crate::db::Request::Databases);
            }
            Response::Databases(databases) => {
                self.databases = databases;
                self.status = super::state::Status::Ready;
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
                self.status = super::state::Status::Ready;
                self.database_edit = Some(DatabaseEdit::new(info));
            }
            Response::DatabaseUpdated { database } => {
                self.status =
                    super::state::Status::Notice(format!("banco {database} atualizado"));
            }
            Response::TableInfo(info) => {
                self.status = super::state::Status::Ready;
                self.table_edit = Some(TableEdit::new(info));
            }
            Response::TableUpdated { before, after } => {
                self.renamed(&before, &after);
                self.status =
                    super::state::Status::Notice(format!("tabela {after} atualizada"));
            }
            Response::Database(database) => {
                self.database = database;
                self.status = super::state::Status::Loading(String::from("tabelas"));
                self.db.request(crate::db::Request::Tables);
            }
            Response::Tables(tables) => {
                self.tables = tables;
                self.status = super::state::Status::Ready;
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
                    self.mode = super::state::Mode::Rows;
                    self.edit = None;
                }
                self.status = super::state::Status::Ready;
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
                    self.status = super::state::Status::Failed(message);
                }
            }
        }
    }

    /// Junta a página seguinte à grade, se ainda for a mesma tabela.
    pub(super) fn append_page(&mut self, table: String, limit: usize, data: TableData) {
        self.loading_more = false;
        if self.current.as_deref() != Some(table.as_str()) {
            // A tabela trocou enquanto a página vinha: ela não serve mais.
            return;
        }
        self.end_reached = data.rows.len() < limit;
        // A página nova pode trazer valores mais largos que os já vistos.
        for (largura, nova) in self.larguras.iter_mut().zip(ui::larguras(&data))
        {
            *largura = (*largura).max(nova);
        }
        if let Some(atual) = self.data.as_mut() {
            atual.rows.extend(data.rows);
        }
        self.status = super::state::Status::Ready;
    }

    /// A tabela pode ter sido renomeada: a lista e a tabela aberta acompanham.
    pub(super) fn renamed(&mut self, before: &str, after: &str) {
        if self.current.as_deref() == Some(before) {
            self.current = Some(after.to_owned());
        }
        let Some(index) = self.tables.iter().position(|table| table == before) else {
            return;
        };
        self.tables[index] = after.to_owned();
        // O servidor lista em ordem alfabética: a lista local segue igual.
        self.tables.sort();
        self.list_state
            .select(self.tables.iter().position(|table| table == after));
    }
}
