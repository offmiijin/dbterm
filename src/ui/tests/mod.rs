use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use super::draw;
use super::grid::{RULE, linhas_visiveis};
use crate::app::{App, ConnectionEdit, DatabaseEdit, Focus, Screen, Status, TableEdit};
use crate::config::{Connection, Field};
use crate::db::{Charset, Collation, ColumnKind, DatabaseInfo, TableData, TableInfo};
use crate::history::{History, Saved};

mod browse;
mod connect;
mod grid;
mod popup_connection;
mod popup_database;
mod popup_table;

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

/// App com o formulário de conexão aberto.
fn app_form() -> App {
    let mut app = app_quick();
    app.screen = Screen::Connect;
    app.connection = Connection::default();
    app.field = Field::Host;
    app.connect_error = None;
    app
}

fn screen(app: &mut App) -> String {
    screen_at(app, 64)
}

fn screen_at(app: &mut App, largura: u16) -> String {
    texto(terminal_de(app, largura, 12).backend().buffer())
}

/// Terminal de teste já desenhado, para conferir texto e cores.
fn terminal_de(app: &mut App, largura: u16, altura: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(largura, altura)).expect("terminal de teste");
    terminal.draw(|frame| draw(frame, app)).expect("desenho");
    terminal
}

/// O buffer inteiro como texto, sem quebras de linha.
fn texto(buffer: &Buffer) -> String {
    buffer.content().iter().map(|cell| cell.symbol()).collect()
}

/// `(x, y)` da primeira célula do `texto` desenhado.
fn achar(buffer: &Buffer, texto: &str) -> (u16, u16) {
    let largura = buffer.area.width as usize;
    let simbolos: Vec<&str> = buffer.content().iter().map(|cell| cell.symbol()).collect();
    for linha in simbolos.chunks(largura) {
        let com_valor: Vec<_> = linha.iter().filter(|s| s.contains("reg")).collect();
        if !com_valor.is_empty() {
            println!("{com_valor:?}");
        }
    }
    let alvo: Vec<String> = texto
        .chars()
        .map(|caractere| caractere.to_string())
        .collect();
    let posicao = simbolos
        .windows(alvo.len())
        .position(|janela| {
            janela
                .iter()
                .zip(&alvo)
                .all(|(celula, alvo)| *celula == alvo.as_str())
        })
        .unwrap_or_else(|| panic!("não achei {texto:?} na tela"));
    ((posicao % largura) as u16, (posicao / largura) as u16)
}

fn app_com_dados() -> App {
    let mut app = App::new(Some("mysql://user@localhost/app"));
    app.server = String::from("MySQL 8.0.35");
    app.database = String::from("app");
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.current = Some(String::from("users"));
    app.data = Some(TableData {
        columns: vec![String::from("id"), String::from("name")],
        kinds: vec![ColumnKind::Number, ColumnKind::Text],
        rows: vec![
            vec![String::from("1"), String::from("ana")],
            vec![String::from("2"), String::from("NULL")],
        ],
        primary_key: vec![0],
    });
    app.status = Status::Ready;
    app
}

fn app_largo() -> App {
    let mut app = App::new(Some("mysql://user@localhost/app"));
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.current = Some(String::from("users"));
    app.data = Some(TableData {
        columns: [
            "id",
            "name",
            "email",
            "active",
            "created_at",
            "last_login",
            "notes",
            "meta",
        ]
        .iter()
        .map(|nome| nome.to_string())
        .collect(),
        kinds: [
            ColumnKind::Number,
            ColumnKind::Text,
            ColumnKind::Text,
            ColumnKind::Number,
            ColumnKind::Text,
            ColumnKind::Text,
            ColumnKind::Text,
            ColumnKind::Text,
        ]
        .to_vec(),
        rows: vec![vec![
            String::from("1"),
            String::from("Ana Prado"),
            String::from("ana@exemplo.com"),
            String::from("1"),
            String::from("2024-01-05 09:12:00"),
            String::from("2026-10-17 08:00:00"),
            String::from("cliente desde o lançamento"),
            String::from("{\"plano\": \"pro\"}"),
        ]],
        primary_key: vec![0],
    });
    app.status = Status::Ready;
    app
}

/// App com o pop-up "Editar" da conexão aberto sobre a tela de trabalho.
fn app_com_popup_de_conexao() -> App {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    let mut connection = Connection::default();
    *connection.value_mut(Field::Host) = String::from("db.local");
    *connection.value_mut(Field::Database) = String::from("app");
    *connection.value_mut(Field::Password) = String::from("segredo");
    app.connection_edit = Some(ConnectionEdit {
        saved: 0,
        connection,
        field: Field::Host,
    });
    app
}

/// Tabela com 14 colunas, mais do que cabe num terminal estreito.
fn app_com_muitas_colunas() -> App {
    let mut app = app_largo();
    let data = app.data.as_mut().expect("grade");
    for index in 0..6 {
        data.columns.push(format!("coluna_{index}"));
        data.kinds.push(ColumnKind::Text);
    }
    for row in data.rows.iter_mut() {
        for index in 0..6 {
            row.push(format!("valor {index}"));
        }
    }
    app
}

/// App com o pop-up "Editar" do banco aberto sobre a tela de trabalho.
fn app_com_popup_de_banco(read_only: Option<bool>) -> App {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Databases;
    app.databases = vec![String::from("app")];
    app.databases_state.select(Some(0));
    app.database_edit = Some(DatabaseEdit::new(DatabaseInfo {
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
                name: String::from("utf8mb4_0900_ai_ci"),
                charset: String::from("utf8mb4"),
            },
            Collation {
                name: String::from("utf8mb4_bin"),
                charset: String::from("utf8mb4"),
            },
        ],
    }));
    app
}

/// App com o pop-up "Editar" da tabela aberto sobre a tela de trabalho.
fn app_com_popup_de_tabela() -> App {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Tables;
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.table_edit = Some(TableEdit::new(TableInfo {
        name: String::from("users"),
        comment: String::from("cadastro"),
        engine: String::from("InnoDB"),
        row_format: String::from("Dynamic"),
        auto_increment: Some(1024),
        charset: String::from("utf8mb4"),
        collation: String::from("utf8mb4_0900_ai_ci"),
        size_bytes: 12 * 1024 * 1024 * 1024,
        engines: vec![String::from("InnoDB"), String::from("MyISAM")],
        charsets: vec![Charset {
            name: String::from("utf8mb4"),
            default_collation: String::from("utf8mb4_0900_ai_ci"),
        }],
        collations: vec![Collation {
            name: String::from("utf8mb4_0900_ai_ci"),
            charset: String::from("utf8mb4"),
        }],
    }));
    app
}

/// Texto de uma linha do buffer de teste.
fn linha_do_buffer(buffer: &Buffer, indice: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer.cell((x, indice)).unwrap().symbol().to_string())
        .collect()
}

/// O texto do comentário nas três linhas da caixa, sem o rótulo nem o
/// preenchimento.
fn comentario_da_caixa(buffer: &Buffer, y: u16) -> Vec<String> {
    let (bx, _) = achar(buffer, "Identificação");
    let inicio = bx + 15 + 2;
    let fim = bx + 57;
    (0..3)
        .map(|offset| {
            (inicio..fim)
                .map(|x| buffer.cell((x, y + offset)).unwrap().symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}
