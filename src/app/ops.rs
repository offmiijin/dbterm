//! Operações do `App`: carregar dados, trocar de banco e salvar edições.
//!
//! Pedidos ao banco moram aqui; a aplicação das respostas mora em `responses`.

use crate::config::{Connection, Field};
use crate::db::{DatabaseAlter, ROW_LIMIT, Request};
use crate::history::Saved;

use super::App;
use super::state::ConnectionEdit;

impl App {
    /// `ctrl+p`: abre o comando master com o último comando no buffer (ou o
    /// `;` inicial, na primeira vez).
    pub(super) fn open_commands(&mut self) {
        self.reopen_commands();
    }

    /// `;`/`#`: abre o comando master direto na aba do prefixo, só com o
    /// prefixo fixo.
    pub(super) fn open_commands_with(&mut self, tab: super::state::CommandTab) {
        self.commands = Some(super::state::Commands::with_tab(tab));
    }

    /// Mostra o pop-up de volta com o texto do buffer, na aba do prefixo.
    fn reopen_commands(&mut self) {
        self.commands = Some(super::state::Commands::with_input(
            self.commands_buffer.clone(),
        ));
    }

    /// `enter` no comando master: executa a aba ativa e guarda o texto no
    /// buffer. Com o `;` puro, recarrega a tabela aberta, como o `r`.
    pub(super) fn execute_commands(&mut self) {
        let Some(commands) = self.commands.take() else {
            return;
        };
        self.commands_buffer = commands.input.clone();
        match commands.tab {
            super::state::CommandTab::Sql => self.execute_sql(commands.sql()),
            super::state::CommandTab::Table => self.execute_table(commands.table_query()),
        }
    }

    /// Executa o SQL: vazio recarrega a tabela aberta, senão vai ao servidor.
    /// Sem declaração (`cod_empresa = 1064`), vira `SELECT` com `WHERE` na
    /// tabela aberta; a primeira página vem com 200 linhas.
    fn execute_sql(&mut self, sql: String) {
        if sql.is_empty() {
            if let Some(table) = self.current.clone() {
                self.load(table);
            } else {
                self.status = super::state::Status::Notice(String::from("nenhuma tabela aberta"));
            }
            return;
        }
        let base = match expandir_sql(&sql, self.current.as_deref()) {
            Ok(base) => base,
            Err(message) => {
                self.status = super::state::Status::Failed(message);
                return;
            }
        };
        self.query_sql = None;
        self.status = super::state::Status::Loading(String::from("consulta"));
        self.db.request(Request::Query {
            sql: base,
            offset: 0,
            limit: ROW_LIMIT,
        });
    }

    /// `enter` na busca de tabela: abre a exata ou a primeira sugestão da
    /// caixa de completar. Sem ela, o pop-up volta para a correção.
    fn execute_table(&mut self, busca: String) {
        if busca.is_empty() {
            self.reopen_commands();
            self.status = super::state::Status::Failed(String::from("informe a tabela"));
            return;
        }
        let Some(index) = self.find_table(&busca) else {
            self.reopen_commands();
            self.status = super::state::Status::Failed(format!("tabela não encontrada: {busca}"));
            return;
        };
        let Some(table) = self.tables.get(index).cloned() else {
            return;
        };
        self.list_state.select(Some(index));
        // Como o segundo `enter` na lista: cai direto nos registros.
        self.focus = super::state::Focus::Data;
        if self.current.as_deref() != Some(table.as_str()) || self.data.is_none() {
            self.load(table);
        }
    }

    /// Índice da tabela buscada: a exata (sem olhar maiúsculas) ou a primeira
    /// sugestão da caixa de completar.
    fn find_table(&self, busca: &str) -> Option<usize> {
        if busca.is_empty() {
            return None;
        }
        self.tables
            .iter()
            .position(|tabela| tabela.eq_ignore_ascii_case(busca))
            .or_else(|| {
                let busca = busca.to_lowercase();
                self.tables
                    .iter()
                    .position(|tabela| tabela.to_lowercase().contains(&busca))
            })
    }

    /// Pede a página seguinte quando a seleção encosta na última linha carregada.
    /// Na grade de consulta (`;`), a próxima página é do SQL; na tabela, da tabela.
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
        if let Some(base) = self.query_sql.clone() {
            self.loading_more = true;
            self.status = super::state::Status::Loading(String::from("consulta"));
            self.db.request(Request::Query {
                sql: base,
                offset: data.rows.len(),
                limit: ROW_LIMIT,
            });
            return;
        }
        let Some(table) = self.current.clone() else {
            return;
        };
        self.loading_more = true;
        self.status = super::state::Status::Loading(table.clone());
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
                self.status = super::state::Status::Failed(message);
                return;
            }
        };
        self.reset_levels();
        self.status = super::state::Status::Connecting;
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
            self.focus = super::state::Focus::Tables;
            return;
        }
        self.use_database(database);
    }

    /// Troca o banco atual pelo `USE`, na mesma conexão.
    pub(super) fn use_database(&mut self, database: String) {
        self.clear_levels();
        self.status = super::state::Status::Loading(database.clone());
        self.db.request(Request::UseDatabase { database });
    }

    /// Esquece tabelas, grade e seleções de uma conexão ou banco que saiu.
    pub(super) fn reset_levels(&mut self) {
        self.databases.clear();
        self.databases_state = ratatui::widgets::ListState::default();
        self.clear_levels();
    }

    /// Esquece tabelas, grade e seleções, preservando a lista de bancos.
    pub(super) fn clear_levels(&mut self) {
        self.tables.clear();
        self.list_state = ratatui::widgets::ListState::default();
        self.current = None;
        self.query_sql = None;
        self.data = None;
        self.table_state = ratatui::widgets::TableState::default();
        self.column = None;
        self.hscroll = 0;
        self.larguras.clear();
        self.end_reached = false;
        self.loading_more = false;
        self.mode = super::state::Mode::Rows;
        self.edit = None;
    }

    /// Abre o formulário vazio: o `n` das conexões rápidas.
    pub(super) fn new_connection(&mut self) {
        self.connection = Connection::default();
        self.field = Field::Host;
        self.connect_error = None;
        self.screen = super::state::Screen::Connect;
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
        self.screen = super::state::Screen::Connect;
    }

    /// Volta do formulário para as conexões rápidas.
    pub(super) fn back_to_quick(&mut self) {
        self.connect_error = None;
        self.connection.value_mut(Field::Password).clear();
        self.screen = super::state::Screen::Quick;
    }

    /// Anda pela lista de conexões rápidas.
    pub(super) fn move_quick(&mut self, delta: isize) {
        self.quick = super::input::step(self.quick, delta, self.history.recent().len());
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
            self.status = super::state::Status::Failed(message);
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
        self.status = super::state::Status::Loading(format!("banco {database}"));
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
        self.status = super::state::Status::Loading(format!("banco {database}"));
        self.db.request(Request::AlterDatabase { database, alter });
    }

    /// `e` nas tabelas: pede ao servidor as propriedades da tabela selecionada;
    /// o pop-up abre quando a resposta chega.
    pub(super) fn request_table_edit(&mut self) {
        let Some(table) = self
            .list_state
            .selected()
            .and_then(|index| self.tables.get(index))
            .cloned()
        else {
            return;
        };
        self.status = super::state::Status::Loading(format!("tabela {table}"));
        self.db.request(Request::TableInfo { table });
    }

    /// Aplica no servidor só o que mudou na tabela. Um valor inválido mantém o
    /// pop-up aberto para a correção.
    pub(super) fn save_table_edit(&mut self) {
        let Some(edit) = self.table_edit.as_ref() else {
            return;
        };
        let alter = match edit.alter() {
            Ok(alter) => alter,
            Err(message) => {
                self.status = super::state::Status::Failed(message);
                return;
            }
        };
        let table = edit.original.name.clone();
        self.table_edit = None;
        if alter.is_empty() {
            return;
        }
        self.status = super::state::Status::Loading(format!("tabela {table}"));
        self.db.request(Request::AlterTable { table, alter });
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
    /// carregadas, que só se perdem ao abrir outra tabela. Sair da consulta
    /// (`;`) é abrir a tabela: a grade volta ao modo de tabela.
    pub(super) fn load(&mut self, table: String) {
        self.status = super::state::Status::Loading(table.clone());
        self.loading_more = false;
        self.end_reached = false;
        self.query_sql = None;
        let limit = self.page_limit(&table);
        self.db.request(Request::Rows {
            table,
            offset: 0,
            limit,
        });
    }

    /// Quantas linhas pedir ao carregar `table`: a tabela que já está na tela
    /// mantém o tanto que já veio; uma tabela nova começa com uma página.
    pub(super) fn page_limit(&self, table: &str) -> usize {
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
        self.mode = super::state::Mode::Columns;
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
            self.status = super::state::Status::Failed(String::from("tabela sem chave primária"));
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
        self.status = super::state::Status::Loading(table);
    }

    pub fn row_count(&self) -> usize {
        self.data.as_ref().map_or(0, |data| data.rows.len())
    }
}

/// SQL completo, que vai ao servidor como digitado: `SELECT`, `WITH`, `SHOW`,
/// escrita (`INSERT`/`UPDATE`/`DELETE`...), DDL e os utilitários (`USE`/`SET`).
const SQL_COMPLETO: [&str; 17] = [
    "SELECT", "WITH", "SHOW", "DESCRIBE", "DESC", "EXPLAIN", "INSERT", "UPDATE", "DELETE",
    "REPLACE", "CREATE", "ALTER", "DROP", "TRUNCATE", "USE", "SET", "CALL",
];

/// Sem declaração vira `SELECT * FROM <tabela aberta> WHERE <condição>`.
/// Um `WHERE` na frente é opcional e sai da condição. Sem tabela aberta, ou
/// sem condição depois do `WHERE`, volta o motivo para a UI mostrar.
pub(super) fn expandir_sql(sql: &str, tabela: Option<&str>) -> Result<String, String> {
    let limpo = sql.trim();
    if SQL_COMPLETO
        .iter()
        .any(|palavra| comeca_com(limpo, palavra))
    {
        return Ok(limpo.to_owned());
    }
    let Some(tabela) = tabela else {
        return Err(String::from("nenhuma tabela aberta"));
    };
    let condicao = match comeca_com(limpo, "WHERE") {
        true => limpo["WHERE".len()..].trim(),
        false => limpo,
    };
    if condicao.is_empty() {
        return Err(String::from("informe a condição"));
    }
    Ok(format!(
        "SELECT * FROM `{}` WHERE {condicao}",
        tabela.replace('`', "``")
    ))
}

/// Igual ao do `db`: a palavra abre o texto com fronteira depois dela.
fn comeca_com(sql: &str, palavra: &str) -> bool {
    let texto = sql.trim_start();
    if texto.len() < palavra.len() {
        return false;
    }
    if !texto[..palavra.len()].eq_ignore_ascii_case(palavra) {
        return false;
    }
    match texto[palavra.len()..].chars().next() {
        None => true,
        Some(c) => !(c.is_ascii_alphanumeric() || c == '_'),
    }
}
