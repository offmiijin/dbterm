use super::super::ops::tabela_do_sql;
use super::*;

fn abre_editor_com(app: &mut App, sql: &str) {
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in sql.chars() {
        app.on_key(tecla(caractere));
    }
}

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
fn ctrl_enter_executa_sem_fechar_o_editor() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(app.is_query_editor(), "o editor continua aberto");
    assert_eq!(app.active_query, Some(0));
    let result = app.query_result.as_ref().expect("painel criado");
    assert_eq!(result.sql, "select 1");
    assert!(result.data.is_none(), "registros ainda vindo");
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
fn executar_abre_os_registros_no_painel_inferior() {
    let mut app = app_com_grade();
    app.on_key(tecla('u'));
    app.on_key(tecla('n'));
    for caractere in "select * from users".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());
    assert!(app.is_query_editor());

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
    let result = app.query_result.as_ref().expect("painel preenchido");
    assert_eq!(result.len(), 1, "os registros aparecem no painel");
    assert_eq!(result.tabela.as_deref(), Some("users"));
    assert!(app.query_sql.is_none(), "a grade principal fica intacta");
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

#[test]
fn tabela_do_sql_mostra_a_tabela_do_from() {
    assert_eq!(
        tabela_do_sql("SELECT * FROM users WHERE id = 1"),
        Some(String::from("users"))
    );
    assert_eq!(
        tabela_do_sql("select * from `app`.`users`"),
        Some(String::from("users"))
    );
    assert_eq!(
        tabela_do_sql("WITH c AS (SELECT 1) SELECT * FROM t"),
        Some(String::from("t"))
    );
    assert_eq!(tabela_do_sql("SELECT 1"), None);
    assert_eq!(tabela_do_sql("SELECT * FROM (SELECT 1)"), None);
}

/// Executa no editor e devolve o app com a primeira página cheia no painel.
fn app_com_resultado(base: &str) -> App {
    let mut app = app_com_grade();
    abre_editor_com(&mut app, base);
    app.on_key(ctrl_enter());
    app.on_response(Response::Query {
        sql: base.to_string(),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: (1..=ROW_LIMIT).map(|n| vec![n.to_string()]).collect(),
            primary_key: Vec::new(),
        },
    });
    app
}

#[test]
fn ctrl_r_foca_o_resultado_e_esc_volta_ao_editor() {
    let mut app = app_com_resultado("select * from users");
    assert!(app.focus == Focus::Data);

    app.on_key(ctrl('r'));
    assert!(app.focus == Focus::Resultado);
    assert_eq!(
        app.query_result.as_ref().and_then(|r| r.state.selected()),
        Some(0)
    );

    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Data);
    assert!(app.is_query_editor());
}

#[test]
fn ctrl_shift_j_foca_o_resultado() {
    let mut app = app_com_resultado("select * from users");
    // Com o teclado estendido, o `shift` chega no `J` maiúsculo.
    app.on_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::CONTROL));
    assert!(app.focus == Focus::Resultado);
}

#[test]
fn ir_ao_resultado_sem_execucao_nao_sai_do_editor() {
    let mut app = app_com_grade();
    abre_editor_com(&mut app, "select 1");
    app.on_key(ctrl('r'));
    assert!(app.focus == Focus::Data);
    app.on_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::CONTROL));
    assert!(app.focus == Focus::Data);
}

#[test]
fn j_no_resultado_anda_e_puxa_a_pagina_seguinte() {
    let mut app = app_com_resultado("select * from users");
    app.on_key(ctrl('r'));

    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    let result = app.query_result.as_ref().expect("painel");
    assert_eq!(result.state.selected(), Some(ROW_LIMIT - 1));
    assert!(result.loading_more, "encostou no fim: pede mais 200");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "consulta"));

    app.on_response(Response::Query {
        sql: String::from("select * from users"),
        offset: ROW_LIMIT,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("201")]],
            primary_key: Vec::new(),
        },
    });
    let result = app.query_result.as_ref().expect("painel");
    assert_eq!(result.len(), ROW_LIMIT + 1);
    assert!(result.end_reached, "página curta fecha a consulta");
    assert!(!result.loading_more);
}

#[test]
fn r_no_resultado_reexecuta_a_query() {
    let mut app = app_com_resultado("select * from users");
    app.on_key(ctrl('r'));
    app.on_key(tecla('r'));

    let result = app.query_result.as_ref().expect("painel");
    assert!(result.data.is_none(), "o painel limpa para a nova busca");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "consulta"));
}
