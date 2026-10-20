use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::input::{PAGE, step};
use super::*;
use crate::config::{DatabaseField, Field, TableField};
use crate::db::{Charset, Collation, ColumnKind, DatabaseInfo, ROW_LIMIT, Response, TableInfo};
use crate::history::{History, Saved};

mod cell_edit;
mod commands;
mod connect;
mod data;
mod navigation;
mod pagination;
mod popup_connection;
mod popup_database;
mod popup_table;

fn app() -> App {
    let mut app = App::new(Some("mysql://user@localhost/app"));
    app.history = History::in_memory();
    app
}

/// App na tela inicial, sem histórico em disco.
fn app_quick() -> App {
    let mut app = App::new(None);
    app.history = History::in_memory();
    app.quick = None;
    app
}

/// App com conexões rápidas salvas, na ordem dada (a primeira é a mais recente).
fn app_com_rapidas(hosts: &[&str]) -> App {
    let mut app = app_quick();
    let total = hosts.len();
    let entries = hosts
        .iter()
        .enumerate()
        .map(|(i, host)| Saved {
            engine: String::from(Saved::MYSQL),
            host: String::from(*host),
            port: 3306,
            user: String::from("root"),
            database: String::from("app"),
            password: None,
            last_used: (total - i) as u64,
        })
        .collect();
    app.history = History::with(entries);
    app.quick = Some(0);
    app
}

/// App com o formulário aberto via `n`.
fn app_form() -> App {
    let mut app = app_quick();
    app.on_key(KeyEvent::from(KeyCode::Char('n')));
    app
}

fn app_com_grade() -> App {
    let mut app = app();
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.current = Some(String::from("users"));
    app.data = Some(TableData {
        columns: vec![String::from("id"), String::from("name")],
        kinds: vec![ColumnKind::Number, ColumnKind::Text],
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

/// Grade com `linhas` registros de uma coluna só, já focada nos dados.
fn grade_com(table: &str, linhas: usize) -> App {
    let mut app = app();
    app.tables = vec![table.to_string()];
    app.list_state.select(Some(0));
    app.current = Some(table.to_string());
    app.data = Some(TableData {
        columns: vec![String::from("id")],
        kinds: vec![ColumnKind::Number],
        rows: (1..=linhas).map(|n| vec![n.to_string()]).collect(),
        primary_key: vec![0],
    });
    app.focus = Focus::Data;
    app.table_state.select(Some(0));
    app.status = Status::Ready;
    app
}

/// Uma página de `linhas` registros a partir de `primeira`.
fn pagina(tabela: &str, primeira: usize, linhas: usize) -> Response {
    Response::Rows {
        table: tabela.to_string(),
        offset: primeira - 1,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: (primeira..primeira + linhas)
                .map(|n| vec![n.to_string()])
                .collect(),
            primary_key: vec![0],
        },
    }
}

/// App na tela de trabalho, com uma conexão salva selecionada.
fn app_nas_conexoes() -> App {
    let mut app = app_com_rapidas(&["db.local", "outro"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    app
}

/// App na tela de trabalho, com o banco `app` selecionado.
fn app_nos_bancos() -> App {
    let mut app = app();
    app.databases = vec![String::from("app"), String::from("outro")];
    app.databases_state.select(Some(0));
    app.focus = Focus::Databases;
    app
}

/// Propriedades de banco como o worker devolveria, com o catálogo do servidor.
fn info_banco(read_only: Option<bool>) -> DatabaseInfo {
    DatabaseInfo {
        name: String::from("app"),
        charset: String::from("utf8mb4"),
        collation: String::from("utf8mb4_0900_ai_ci"),
        read_only,
        size_bytes: 12 * 1024 * 1024 * 1024,
        charsets: vec![
            Charset {
                name: String::from("latin1"),
                default_collation: String::from("latin1_swedish_ci"),
            },
            Charset {
                name: String::from("utf8mb4"),
                default_collation: String::from("utf8mb4_0900_ai_ci"),
            },
        ],
        collations: vec![
            Collation {
                name: String::from("latin1_swedish_ci"),
                charset: String::from("latin1"),
            },
            Collation {
                name: String::from("utf8mb4_0900_ai_ci"),
                charset: String::from("utf8mb4"),
            },
            Collation {
                name: String::from("utf8mb4_bin"),
                charset: String::from("utf8mb4"),
            },
        ],
    }
}

/// App na tela de trabalho, com a tabela `users` selecionada.
fn app_nas_tabelas() -> App {
    let mut app = app();
    app.databases = vec![String::from("app")];
    app.tables = vec![String::from("orders"), String::from("users")];
    app.list_state.select(Some(1));
    app.focus = Focus::Tables;
    app
}

/// Propriedades de tabela como o worker devolveria, com o catálogo do servidor.
fn info_tabela() -> TableInfo {
    TableInfo {
        name: String::from("users"),
        comment: String::from("cadastro"),
        engine: String::from("InnoDB"),
        row_format: String::from("Dynamic"),
        auto_increment: Some(1024),
        charset: String::from("utf8mb4"),
        collation: String::from("utf8mb4_0900_ai_ci"),
        size_bytes: 12 * 1024 * 1024 * 1024,
        engines: vec![String::from("InnoDB"), String::from("MyISAM")],
        charsets: vec![
            Charset {
                name: String::from("latin1"),
                default_collation: String::from("latin1_swedish_ci"),
            },
            Charset {
                name: String::from("utf8mb4"),
                default_collation: String::from("utf8mb4_0900_ai_ci"),
            },
        ],
        collations: vec![
            Collation {
                name: String::from("latin1_swedish_ci"),
                charset: String::from("latin1"),
            },
            Collation {
                name: String::from("utf8mb4_0900_ai_ci"),
                charset: String::from("utf8mb4"),
            },
            Collation {
                name: String::from("utf8mb4_bin"),
                charset: String::from("utf8mb4"),
            },
        ],
    }
}

/// App com o pop-up da tabela aberto, como se a resposta tivesse chegado.
fn app_com_popup_de_tabela() -> App {
    let mut app = app_nas_tabelas();
    app.on_response(Response::TableInfo(info_tabela()));
    app
}

/// Tecla de texto, sem modificadores.
fn tecla(caractere: char) -> KeyEvent {
    KeyEvent::from(KeyCode::Char(caractere))
}

/// Tecla com Ctrl, como o terminal a entrega em modo bruto.
fn ctrl(caractere: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(caractere), KeyModifiers::CONTROL)
}
