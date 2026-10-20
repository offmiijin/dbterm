use super::*;

#[test]
fn u_cria_a_primeira_query_e_abre_o_editor() {
    let mut app = app_com_grade();
    assert!(app.queries.is_empty());

    // `u` é o atalho das qUeries (`q` sai da aplicação).
    app.on_key(tecla('u'));
    assert_eq!(app.queries.len(), 1);
    assert_eq!(app.queries[0].nome, "q1");
    assert_eq!(app.active_query, Some(0));
    assert!(app.focus == Focus::Data);
    assert!(app.is_query_editor());
}

#[test]
fn n_na_lista_cria_instancia_vazia_e_digitar_preenche() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Queries);

    app.on_key(tecla('n'));
    assert_eq!(app.queries.len(), 2);
    assert_eq!(app.active_query, Some(1));
    assert!(app.focus == Focus::Data);

    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    assert_eq!(app.queries[1].sql, "select 1");
}

#[test]
fn enter_quebra_linha_e_esc_volta_as_queries() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('a'));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(tecla('b'));
    assert_eq!(app.queries[0].sql, "a\nb");

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(app.queries[0].sql, "a\n");

    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Queries);
    // O editor continua aberto no centro, só o foco saiu dele.
    assert_eq!(app.active_query, Some(0));
}

#[test]
fn enter_na_lista_abre_o_editor_da_selecionada() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(KeyEvent::from(KeyCode::Esc));
    app.on_key(tecla('n'));
    app.on_key(KeyEvent::from(KeyCode::Esc));
    // Duas instâncias; volta ao topo e abre a primeira.
    app.on_key(tecla('g'));
    assert_eq!(app.queries_state.selected(), Some(0));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.active_query, Some(0));
    assert!(app.focus == Focus::Data);
}

#[test]
fn d_apaga_e_volta_a_grade_quando_era_a_aberta() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(KeyEvent::from(KeyCode::Esc));
    app.on_key(tecla('d'));
    assert!(app.queries.is_empty());
    assert_eq!(app.active_query, None);
    assert!(!app.is_query_editor());
}

#[test]
fn abrir_tabela_fecha_o_editor_e_volta_a_grade() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    assert!(app.is_query_editor());

    app.focus = Focus::Tables;
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.active_query, None);
    assert!(!app.is_query_editor());
}

#[test]
fn ponto_e_virgula_no_editor_e_texto_e_nao_popup() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla(';'));
    assert!(app.commands.is_none());
    assert_eq!(app.queries[0].sql, ";");
}
