use super::*;

#[test]
fn encostar_na_ultima_linha_pede_e_anexa_a_pagina_seguinte() {
    let mut app = grade_com("events", ROW_LIMIT);
    // Enquanto a seleção não chega no fim, nada é pedido.
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(!app.loading_more);
    assert!(!app.end_reached);

    // Chegou no registro 200: pede mais 200.
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert!(app.loading_more);
    assert!(matches!(app.status, Status::Loading(ref tabela) if tabela == "events"));

    app.on_response(pagina("events", ROW_LIMIT + 1, ROW_LIMIT));
    assert_eq!(app.row_count(), 2 * ROW_LIMIT);
    assert!(!app.loading_more);
    assert!(!app.end_reached);
    assert!(matches!(app.status, Status::Ready));
    // A seleção fica onde estava: a página entra no fim, não no lugar do cursor.
    assert_eq!(app.table_state.selected(), Some(ROW_LIMIT - 1));

    // Chegou no registro 400: mais 200.
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert!(app.loading_more);
    app.on_response(pagina("events", 2 * ROW_LIMIT + 1, ROW_LIMIT));
    assert_eq!(app.row_count(), 3 * ROW_LIMIT);
}

#[test]
fn pagina_curta_marca_o_fim_da_tabela() {
    let mut app = grade_com("events", ROW_LIMIT);
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert!(app.loading_more);

    app.on_response(pagina("events", ROW_LIMIT + 1, 12));
    assert!(app.end_reached);
    assert_eq!(app.row_count(), ROW_LIMIT + 12);

    // No fim da tabela não há mais o que pedir.
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert_eq!(app.table_state.selected(), Some(ROW_LIMIT + 11));
    assert!(!app.loading_more);
}

#[test]
fn pagina_que_chega_depois_de_trocar_de_tabela_e_descartada() {
    let mut app = grade_com("events", 10);
    app.table_state.select(Some(8));
    app.on_key(KeyEvent::from(KeyCode::Char('j'))); // chegou no fim: pede a próxima
    assert!(app.loading_more);

    // A resposta da primeira página de outra tabela chega antes.
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

    // A página atrasada de `events` não pode entrar na grade de `users`.
    app.on_response(pagina("events", 11, 5));
    assert_eq!(app.current.as_deref(), Some("users"));
    assert_eq!(app.row_count(), 1);
}

#[test]
fn recarregar_a_mesma_tabela_pede_as_linhas_ja_carregadas() {
    let app = grade_com("events", 2 * ROW_LIMIT);
    assert_eq!(app.page_limit("events"), 2 * ROW_LIMIT);
    // Outra tabela começa do zero.
    assert_eq!(app.page_limit("users"), ROW_LIMIT);
    // Uma tabela com menos de uma página também pede ao menos uma página.
    let app = grade_com("events", 3);
    assert_eq!(app.page_limit("events"), ROW_LIMIT);
}

#[test]
fn falha_na_pagina_libera_a_proxima_tentativa() {
    let mut app = grade_com("events", 10);
    app.table_state.select(Some(8));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(app.loading_more);

    app.on_response(Response::Failed(String::from("conexão caiu")));
    assert!(!app.loading_more);
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "conexão caiu"));

    // A tabela não acabou: subir e descer de novo ao fim tenta outra página.
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert!(!app.loading_more);
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(app.loading_more);
}

#[test]
fn segurar_a_tecla_no_fim_da_pagina_nao_pede_de_novo() {
    let mut app = grade_com("events", 10);
    app.table_state.select(Some(8));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(app.loading_more);
    // A resposta chega e a página nova entra: a seleção não se moveu.
    app.on_response(pagina("events", 11, ROW_LIMIT));
    assert!(!app.loading_more);
    assert_eq!(app.row_count(), 10 + ROW_LIMIT);

    // No fim da página carregada, repetir `j` não pede nada.
    app.table_state.select(Some(10 + ROW_LIMIT - 1));
    for _ in 0..5 {
        app.on_key(KeyEvent::from(KeyCode::Char('j')));
    }
    assert_eq!(app.table_state.selected(), Some(10 + ROW_LIMIT - 1));
    assert!(
        !app.loading_more,
        "tecla repetida no fim não pode pedir página"
    );

    // `G` já no fim também não.
    for _ in 0..5 {
        app.on_key(KeyEvent::from(KeyCode::Char('G')));
    }
    assert!(!app.loading_more);
}
