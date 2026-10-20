use super::*;

#[test]
fn u_vai_para_as_queries_sem_criar_nada() {
    let mut app = app_com_grade();
    assert!(app.queries.is_empty());

    // `u` é o atalho das qUeries (`q` sai da aplicação): só move o foco.
    app.on_key(tecla('u'));
    assert!(app.focus == Focus::Queries);
    assert!(app.queries.is_empty());
    assert_eq!(app.active_query, None);
    assert!(!app.is_query_editor());
}

#[test]
fn n_na_lista_cria_instancia_vazia_e_digitar_preenche() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    assert!(app.focus == Focus::Queries);

    app.on_key(tecla('n'));
    assert_eq!(app.queries.len(), 1);
    assert_eq!(app.queries[0].nome, "q1");
    assert_eq!(app.active_query, Some(0));
    assert!(app.focus == Focus::Data);

    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    assert_eq!(app.queries[0].sql, "select 1");
}

#[test]
fn enter_quebra_linha_e_esc_volta_as_queries() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
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
    app.on_key(tecla('n'));
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
    app.on_key(tecla('n'));
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
    app.on_key(tecla('n'));
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
    app.on_key(tecla('n'));
    app.on_key(tecla(';'));
    assert!(app.commands.is_none());
    assert_eq!(app.queries[0].sql, ";");
}

/// `ctrl+enter` como o terminal entrega em modo bruto.
fn ctrl_enter() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL)
}

#[test]
fn tab_indenta_com_dois_espacos() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(app.queries[0].sql, "  ");
}

#[test]
fn ctrl_enter_executa_e_mostra_a_grade() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(!app.is_query_editor(), "o editor fecha para a grade");
    assert_eq!(app.active_query, None);
    assert_eq!(app.queries[0].sql, "select 1", "texto preservado");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "consulta"));
}

#[test]
fn ctrl_enter_vazio_avisa_sem_fechar_o_editor() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    app.on_key(ctrl_enter());

    assert!(matches!(app.status, Status::Notice(ref aviso) if aviso == "consulta vazia"));
    assert_eq!(app.active_query, Some(0));
}

#[test]
fn ctrl_enter_fora_do_select_falha_sem_fechar_o_editor() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "SHOW TABLES".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "somente SELECT é permitido"));
    assert_eq!(app.active_query, Some(0), "editor continua aberto");
}

#[test]
fn executar_abre_os_registros_na_tela_de_dados() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "select * from users".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());
    assert!(!app.is_query_editor());

    app.on_response(Response::Query {
        sql: String::from("select * from users"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("1")]],
            primary_key: Vec::new(),
        },
    });
    assert!(app.focus == Focus::Data);
    assert!(app.data.is_some(), "os registros aparecem na grade");
    assert_eq!(app.query_sql.as_deref(), Some("select * from users"));
    assert!(matches!(app.status, Status::Ready));
}

#[test]
fn enter_puro_quebra_linha_em_vez_de_executar() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.queries[0].sql, "\n");
    assert!(matches!(app.status, Status::Ready));
}

#[test]
fn setas_movem_e_digitar_insere_no_cursor() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "ac".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(app.queries[0].cursor, 1);
    app.on_key(tecla('b'));
    assert_eq!(app.queries[0].sql, "abc");
    assert_eq!(app.queries[0].cursor, 2);

    // Nas pontas, o cursor prende sem sair do texto.
    app.on_key(KeyEvent::from(KeyCode::Right));
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.queries[0].cursor, 3);
    for _ in 0..5 {
        app.on_key(KeyEvent::from(KeyCode::Left));
    }
    assert_eq!(app.queries[0].cursor, 0);
}

#[test]
fn backspace_apaga_antes_do_cursor() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "ac".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(KeyEvent::from(KeyCode::Left));
    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(app.queries[0].sql, "c");
    assert_eq!(app.queries[0].cursor, 0);
}

#[test]
fn ctrl_h_j_k_l_movem_como_as_setas() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "ab".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl('h'));
    assert_eq!(app.queries[0].cursor, 1);
    app.on_key(ctrl('l'));
    assert_eq!(app.queries[0].cursor, 2);

    // Sem `ctrl`, a letra é texto em vez de movimento.
    app.on_key(tecla('j'));
    assert_eq!(app.queries[0].sql, "abj");
}

#[test]
fn cima_baixo_mantem_a_coluna_entre_linhas() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    app.queries[0].sql = String::from("ab\ncdef");
    app.queries[0].cursor = 7;
    assert_eq!(app.queries[0].linha_col(), (1, 4));

    app.on_key(KeyEvent::from(KeyCode::Up));
    assert_eq!(app.queries[0].cursor, 2, "prende no fim da linha curta");
    app.on_key(KeyEvent::from(KeyCode::Down));
    assert_eq!(app.queries[0].cursor, 5);
    app.on_key(ctrl('k'));
    assert_eq!(app.queries[0].cursor, 2);
    app.on_key(ctrl('j'));
    assert_eq!(app.queries[0].cursor, 5);
}

#[test]
fn enter_no_meio_quebra_a_linha_no_cursor() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "ac".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(KeyEvent::from(KeyCode::Left));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.queries[0].sql, "a\nc");
    assert_eq!(app.queries[0].cursor, 2);
}
