use super::*;

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
        offset: 0,
        limit: ROW_LIMIT,
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
fn recarregar_a_mesma_tabela_preserva_linha_e_coluna() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    app.table_state.select(Some(1));

    app.on_response(Response::Rows {
        table: String::from("users"),
        offset: 0,
        limit: ROW_LIMIT,
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
