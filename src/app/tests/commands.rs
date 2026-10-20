use super::*;

fn ctrl_enter() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL)
}

fn app_navegando() -> App {
    let mut app = app_com_grade();
    app.focus = Focus::Tables;
    app
}

#[test]
fn ctrl_p_abre_na_aba_sql_com_ponto_e_virgula() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));

    let commands = app.commands.as_ref().expect("pop-up aberto");
    assert_eq!(commands.tab, crate::app::CommandTab::Sql);
    assert_eq!(commands.input, ";");
}

#[test]
fn reabrir_volta_com_o_ponto_e_virgula() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.commands.is_none());

    app.on_key(ctrl('P'));
    assert_eq!(
        app.commands.as_ref().expect("pop-up aberto").input,
        ";",
        "toda abertura da aba SQL traz o `;`"
    );
}

#[test]
fn prefixo_e_fixo_e_troca_pelo_prefixo() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    assert_eq!(app.commands.as_ref().expect("pop-up").input, ";select 1");

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(app.commands.as_ref().expect("pop-up").input, ";select ");

    // Apagar tudo para no prefixo fixo, sem esvaziar a linha.
    for _ in 0..";select ".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    let commands = app.commands.as_ref().expect("pop-up");
    assert_eq!(commands.input, ";");
    assert_eq!(commands.tab, crate::app::CommandTab::Sql);

    // Com só o prefixo, digitar o `#` troca para a busca de tabela.
    app.on_key(tecla('#'));
    let commands = app.commands.as_ref().expect("pop-up");
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    assert_eq!(commands.input, "#");

    for caractere in "pro".chars() {
        app.on_key(tecla(caractere));
    }
    let commands = app.commands.as_ref().expect("pop-up");
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    assert_eq!(commands.input, "#pro");
}

#[test]
fn tab_troca_de_aba_levando_o_texto_junto() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }

    // Só o prefixo troca; o resto do texto acompanha a aba.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    let commands = app.commands.as_ref().expect("pop-up");
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    assert_eq!(commands.input, "#select 1");

    app.on_key(KeyEvent::from(KeyCode::BackTab));
    let commands = app.commands.as_ref().expect("pop-up");
    assert_eq!(commands.tab, crate::app::CommandTab::Sql);
    assert_eq!(commands.input, ";select 1");
}

#[test]
fn reabrir_com_hash_volta_na_aba_table() {
    let mut app = app_nas_tabelas();
    app.on_key(ctrl('p'));
    // O pop-up abre com o `;` fixo: digitar o `#` troca para a busca.
    app.on_key(KeyEvent::from(KeyCode::Backspace));
    for caractere in "#users".chars() {
        app.on_key(tecla(caractere));
    }
    let commands = app.commands.as_ref().expect("pop-up");
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    app.on_key(KeyEvent::from(KeyCode::Esc));

    app.on_key(ctrl('p'));
    let commands = app.commands.as_ref().expect("pop-up aberto");
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    assert_eq!(commands.input, "#users");
}

#[test]
fn sql_tira_o_prefixo_e_os_espacos() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "  select COUNT(*) from users  ".chars() {
        app.on_key(tecla(caractere));
    }
    assert_eq!(
        app.commands.as_ref().expect("pop-up").sql(),
        "select COUNT(*) from users"
    );
}

#[test]
fn esc_fecha_sem_executar() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    app.on_key(tecla('x'));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    assert!(app.commands.is_none());
    assert!(!app.should_quit);
    assert!(matches!(app.status, Status::Ready));
}

#[test]
fn ponto_e_virgula_puro_recarrega_a_tabela_aberta() {
    let mut app = app_navegando();
    app.current = Some(String::from("users"));
    app.on_key(ctrl('p'));
    app.on_key(ctrl_enter());

    assert!(app.commands.is_none(), "o pop-up fecha ao executar");
    assert!(matches!(app.status, Status::Loading(ref t) if t == "users"));
}

#[test]
fn ponto_e_virgula_puro_sem_tabela_avisa() {
    let mut app = app_navegando();
    app.current = None;
    app.on_key(ctrl('p'));
    app.on_key(ctrl_enter());

    assert!(app.commands.is_none());
    assert!(matches!(app.status, Status::Notice(ref aviso) if aviso == "nenhuma tabela aberta"));
}

#[test]
fn comando_executado_fica_no_buffer_para_a_proxima_abertura() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());
    assert!(app.commands.is_none());

    app.on_key(ctrl('p'));
    assert_eq!(
        app.commands.as_ref().expect("pop-up aberto").input,
        ";select 1"
    );
}

#[test]
fn esc_tambem_guarda_o_texto_no_buffer() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "select 2".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.commands.is_none());

    app.on_key(ctrl('p'));
    assert_eq!(
        app.commands.as_ref().expect("pop-up aberto").input,
        ";select 2"
    );
}

#[test]
fn ctrl_enter_com_sql_fecha_e_pede_a_consulta() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "select COUNT(*) from users".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(app.commands.is_none(), "o pop-up fecha ao executar");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "consulta"));
}

#[test]
fn prefixos_fora_do_popup_abrem_o_comando() {
    // O `;` abre direto no SQL e o `#`, na Tabela.
    let mut app = app_navegando();
    app.on_key(tecla(';'));
    let commands = app.commands.as_ref().expect("pop-up aberto");
    assert_eq!(commands.tab, crate::app::CommandTab::Sql);
    assert_eq!(commands.input, ";");
    app.on_key(KeyEvent::from(KeyCode::Esc));

    let mut app = app_navegando();
    app.on_key(tecla('#'));
    let commands = app.commands.as_ref().expect("pop-up aberto");
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    assert_eq!(commands.input, "#");
    app.on_key(KeyEvent::from(KeyCode::Esc));

    // Nos registros também abre por fora.
    let mut app = app_com_grade();
    app.on_key(tecla(';'));
    assert_eq!(
        app.commands.as_ref().expect("pop-up aberto").input,
        ";",
        "nos registros abre por fora"
    );
}

#[test]
fn fora_das_tabelas_o_ctrl_p_nao_abre() {
    let mut app = app_nos_bancos();
    app.on_key(ctrl('p'));
    assert!(app.commands.is_none(), "nos bancos não abre");

    let mut app = app_nas_conexoes();
    app.on_key(ctrl('p'));
    assert!(app.commands.is_none(), "nas conexões não abre");

    // Os prefixos obedecem a mesma regra do `ctrl+p`.
    let mut app = app_nos_bancos();
    app.on_key(tecla(';'));
    assert!(app.commands.is_none(), "`;` nos bancos não abre");
    app.on_key(tecla('#'));
    assert!(app.commands.is_none(), "`#` nos bancos não abre");
}

#[test]
fn enter_sozinho_executa_como_o_ctrl_enter_do_terminal() {
    // Sem o protocolo estendido de teclado, o terminal entrega o `ctrl+enter`
    // como um `enter` comum: o pop-up executa nos dois.
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(KeyEvent::from(KeyCode::Enter));

    assert!(app.commands.is_none(), "o pop-up fecha ao executar");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "consulta"));
}

#[test]
fn sugestoes_completam_pelo_nome_da_tabela() {
    let tabelas = vec![
        String::from("orders"),
        String::from("products"),
        String::from("profiles"),
    ];
    let commands = crate::app::Commands::with_input(String::from("#pro"));
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    assert_eq!(commands.table_query(), "pro");
    assert_eq!(commands.suggestions(&tabelas), vec![1, 2]);
}

#[test]
fn sugestoes_vazias_na_aba_sql_e_sem_prefixo_mostra_a_lista() {
    let tabelas = vec![String::from("orders"), String::from("users")];
    let sql = crate::app::Commands::with_input(String::from(";select 1"));
    assert!(sql.suggestions(&tabelas).is_empty());

    let busca = crate::app::Commands::with_input(String::from("#"));
    assert_eq!(busca.suggestions(&tabelas), vec![0, 1]);
}

#[test]
fn enter_na_busca_exata_abre_a_tabela_nos_registros() {
    let mut app = app_nas_tabelas();
    app.on_key(ctrl('p'));
    app.on_key(KeyEvent::from(KeyCode::Backspace));
    for caractere in "#users".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(app.commands.is_none(), "o pop-up fecha ao executar");
    assert!(app.focus == Focus::Data, "cai direto nos registros");
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(app.status, Status::Loading(ref t) if t == "users"));
}

#[test]
fn enter_na_busca_parcial_abre_a_primeira_sugestao() {
    let mut app = app_nas_tabelas();
    app.on_key(ctrl('p'));
    app.on_key(KeyEvent::from(KeyCode::Backspace));
    for caractere in "#use".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(app.commands.is_none());
    assert!(app.focus == Focus::Data);
    assert!(matches!(app.status, Status::Loading(ref t) if t == "users"));
}

#[test]
fn enter_sem_tabela_mantem_o_popup_para_a_correcao() {
    let mut app = app_nas_tabelas();
    app.on_key(ctrl('p'));
    app.on_key(KeyEvent::from(KeyCode::Backspace));
    for caractere in "#xyz".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(app.commands.is_some(), "o pop-up continua aberto");
    assert_eq!(app.commands.as_ref().expect("pop-up").input, "#xyz");
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "tabela não encontrada: xyz"));
}

#[test]
fn enter_com_hash_puro_pede_a_tabela() {
    let mut app = app_nas_tabelas();
    app.on_key(ctrl('p'));
    app.on_key(KeyEvent::from(KeyCode::Backspace));
    app.on_key(tecla('#'));
    app.on_key(ctrl_enter());

    assert!(app.commands.is_some(), "o pop-up continua aberto");
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "informe a tabela"));
}

#[test]
fn resposta_sem_limit_pagina_de_200_em_200() {
    let mut app = app_com_grade();
    let base = String::from("select * from objetos where cod_empresa = 1064");
    let pagina_cheia = |primeira: usize| Response::Query {
        sql: base.clone(),
        offset: primeira - 1,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: (primeira..primeira + ROW_LIMIT)
                .map(|n| vec![n.to_string()])
                .collect(),
            primary_key: Vec::new(),
        },
    };
    app.on_response(pagina_cheia(1));

    assert_eq!(app.row_count(), ROW_LIMIT);
    assert!(!app.end_reached, "página cheia pede a próxima");
    assert_eq!(app.query_sql.as_deref(), Some(base.as_str()));
    assert!(app.focus == Focus::Data);
    assert!(matches!(app.status, Status::Ready));

    // Encostar na última linha pede a próxima página da consulta.
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert!(app.loading_more);
    assert!(matches!(app.status, Status::Loading(ref o) if o == "consulta"));

    // Página curta fecha a consulta.
    app.on_response(Response::Query {
        sql: base.clone(),
        offset: ROW_LIMIT,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("201")]],
            primary_key: Vec::new(),
        },
    });
    assert_eq!(app.row_count(), ROW_LIMIT + 1);
    assert!(app.end_reached);
    assert!(!app.loading_more);
}

#[test]
fn resposta_da_consulta_vira_a_grade_sem_paginacao() {
    // Com `LIMIT` próprio, a consulta executa de uma vez e não pagina.
    let mut app = app_com_grade();
    app.on_response(Response::Query {
        sql: String::from("SELECT * FROM users LIMIT 1"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("COUNT(*)")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("2")]],
            primary_key: Vec::new(),
        },
    });

    let data = app.data.as_ref().expect("grade com o retorno");
    assert_eq!(data.columns, ["COUNT(*)"]);
    assert_eq!(data.rows, [["2"]]);
    assert!(app.end_reached, "consulta não pagina");
    assert!(app.query_sql.is_none());
    assert!(!app.loading_more);
    assert_eq!(app.table_state.selected(), Some(0));
    assert!(app.focus == Focus::Data);
    assert!(matches!(app.status, Status::Ready));
}

#[test]
fn resposta_sem_colunas_mantem_a_grade_com_aviso() {
    let mut app = app_com_grade();
    let linhas = app.row_count();
    app.on_response(Response::Query {
        sql: String::from("UPDATE users SET name = 'ana' WHERE id = 1"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: Vec::new(),
            kinds: Vec::new(),
            rows: Vec::new(),
            primary_key: Vec::new(),
        },
    });

    assert_eq!(app.row_count(), linhas, "a grade fica como está");
    assert!(
        matches!(app.status, Status::Notice(ref aviso) if aviso == "consulta executada (sem retorno)")
    );
}

#[test]
fn atalho_condicao_vira_select_na_tabela_aberta() {
    let expandir = super::super::ops::expandir_sql;
    assert_eq!(
        expandir("cod_empresa = 1064", Some("objetos")),
        Ok(String::from(
            "SELECT * FROM `objetos` WHERE cod_empresa = 1064"
        ))
    );
    // O `WHERE` na frente é opcional.
    assert_eq!(
        expandir("where cod_empresa = 1064", Some("objetos")),
        Ok(String::from(
            "SELECT * FROM `objetos` WHERE cod_empresa = 1064"
        ))
    );
    // Tabela com crase escapa dobrando.
    assert_eq!(
        expandir("id = 1", Some("we`ird")),
        Ok(String::from("SELECT * FROM `we``ird` WHERE id = 1"))
    );
}

#[test]
fn atalho_sql_completo_passa_direto() {
    let expandir = super::super::ops::expandir_sql;
    let sql = "select * from objetos where cod_empresa = 1064";
    assert_eq!(expandir(sql, Some("objetos")), Ok(String::from(sql)));
    assert_eq!(
        expandir("WITH c AS (SELECT 1) SELECT * FROM c", None),
        Ok(String::from("WITH c AS (SELECT 1) SELECT * FROM c"))
    );
}

#[test]
fn atalho_fora_do_select_e_recusado() {
    let expandir = super::super::ops::expandir_sql;
    assert_eq!(
        expandir("SHOW TABLES", None),
        Err(String::from("somente SELECT é permitido"))
    );
    assert_eq!(
        expandir("UPDATE objetos SET nome = 'x' WHERE id = 1", None),
        Err(String::from("somente SELECT é permitido"))
    );
    assert_eq!(
        expandir("DELETE FROM objetos WHERE id = 1", Some("objetos")),
        Err(String::from("somente SELECT é permitido"))
    );
}

#[test]
fn atalho_sem_tabela_ou_sem_condicao_falha() {
    let expandir = super::super::ops::expandir_sql;
    assert_eq!(
        expandir("cod_empresa = 1064", None),
        Err(String::from("nenhuma tabela aberta"))
    );
    assert_eq!(
        expandir("where", Some("objetos")),
        Err(String::from("informe a condição"))
    );
}

#[test]
fn abrir_o_popup_fixa_a_tabela_aberta() {
    let mut app = app_navegando();
    app.current = Some(String::from("objetos"));
    app.on_key(ctrl('p'));
    assert_eq!(
        app.commands.as_ref().expect("pop-up aberto").tabela,
        Some(String::from("objetos"))
    );
}

#[test]
fn sql_fora_do_select_mantem_o_popup_para_a_correcao() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "SHOW TABLES".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert_eq!(
        app.commands.as_ref().expect("pop-up aberto").input,
        ";SHOW TABLES"
    );
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "somente SELECT é permitido"));
}

#[test]
fn busca_nova_esvazia_a_grade_na_hora() {
    let mut app = app_com_grade();
    assert!(app.data.is_some());
    app.on_key(ctrl('p'));
    for caractere in "select * from users".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    assert!(app.commands.is_none(), "o pop-up fecha ao executar");
    assert!(app.data.is_none(), "o buffer anterior sai na hora");
    assert_eq!(app.row_count(), 0);
    assert!(app.query_sql.is_none());
    assert!(matches!(app.status, Status::Loading(ref o) if o == "consulta"));
}

#[test]
fn reset_com_ponto_e_virgula_limpa_e_pede_200() {
    // Mesmo com mais de uma página carregada, o reset volta ao padrão.
    let mut app = grade_com("events", 2 * ROW_LIMIT);
    app.on_key(ctrl('p'));
    app.on_key(ctrl_enter());

    assert!(app.commands.is_none(), "o pop-up fecha ao executar");
    assert!(app.data.is_none(), "o buffer anterior sai na hora");
    assert!(app.query_sql.is_none());
    assert!(!app.end_reached);
    assert!(matches!(app.status, Status::Loading(ref t) if t == "events"));
}

#[test]
fn prefixo_sql_restaura_a_ultima_busca() {
    let mut app = app_com_grade();
    app.on_key(ctrl('p'));
    for caractere in "id = 10".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());
    assert!(app.commands.is_none());

    app.on_key(tecla(';'));
    assert_eq!(
        app.commands.as_ref().expect("pop-up aberto").input,
        ";id = 10"
    );
}

#[test]
fn prefixo_de_outra_aba_abre_limpo() {
    let mut app = app_com_grade();
    app.on_key(ctrl('p'));
    for caractere in "id = 10".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl_enter());

    app.on_key(tecla('#'));
    let commands = app.commands.as_ref().expect("pop-up aberto");
    assert_eq!(commands.tab, crate::app::CommandTab::Table);
    assert_eq!(commands.input, "#");
}
