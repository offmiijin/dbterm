use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::input::{PAGE, step};
use super::*;
use crate::db::ColumnKind;

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
            kinds: vec![ColumnKind::Number],
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

    // `h`/`l` trocam de painel; `tab` não mexe mais no foco.
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert!(app.focus == Focus::Data);
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert!(app.focus == Focus::Tables);

    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    assert!(app.should_quit);
}

#[test]
fn esc_nos_registros_volta_as_tabelas_e_so_sai_nelas() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;

    // Nos registros, `esc` volta às tabelas em vez de sair.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Tables);
    assert!(!app.should_quit);

    // Nas tabelas, `esc` sai da aplicação.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.should_quit);
}

#[test]
fn h_l_trocam_de_painel_e_tab_nao_faz_nada() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert!(app.focus == Focus::Data);

    // Tab foi removido da troca de foco entre os painéis.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert!(app.focus == Focus::Data);
    app.on_key(KeyEvent::from(KeyCode::BackTab));
    assert!(app.focus == Focus::Data);
}

#[test]
fn enter_entra_no_modo_coluna_e_h_l_anda_na_horizontal() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);
    assert_eq!(app.column, Some(0));

    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert_eq!(app.column, Some(1));
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.column, Some(1), "clampa no fim");
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert_eq!(app.column, Some(0));

    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Rows);
    assert_eq!(app.column, None);
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
            kinds: vec![ColumnKind::Number, ColumnKind::Text],
            rows: vec![
                vec![String::from("1"), String::from("ana")],
                vec![String::from("2"), String::from("bia")],
            ],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.table_state.selected(), Some(1));
    assert_eq!(app.column, Some(1));
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
        vistos.push(app.column.unwrap());
    }
    // 0 → coluna 1 → coluna 2 → clampa.
    assert_eq!(vistos, vec![1, 2, 2]);
}

#[test]
fn segundo_enter_na_tabela_aberta_vai_aos_registros() {
    let mut app = app();
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.status = Status::Ready;

    // Primeiro enter: começa a carregar e o foco continua na lista.
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Tables);
    assert!(matches!(app.status, Status::Loading(ref t) if t == "users"));

    // Resposta: os registros ficam prontos, mas o foco segue na lista.
    app.on_response(Response::Rows {
        table: String::from("users"),
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("1")]],
            primary_key: vec![0],
        },
    });
    assert!(app.focus == Focus::Tables);

    // Segundo enter: vai direto aos registros, sem precisar de `l`.
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Data);
    assert_eq!(app.mode, Mode::Rows);
}

#[test]
fn enter_em_outra_tabela_carrega_sem_pular_aos_registros() {
    // `users` está aberta, mas a lista aponta para `orders`: o enter deve
    // carregar a nova tabela e manter o foco na lista.
    let mut app = app_com_grade();
    app.focus = Focus::Tables;
    app.tables = vec![String::from("users"), String::from("orders")];
    app.list_state.select(Some(1));

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Tables);
    assert!(matches!(app.status, Status::Loading(ref t) if t == "orders"));
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
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("7")]],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.table_state.selected(), Some(0));
    assert_eq!(app.column, None);
    assert_eq!(app.mode, Mode::Rows);
}
