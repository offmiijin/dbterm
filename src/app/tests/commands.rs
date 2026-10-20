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
fn digitacao_entra_no_fim_e_backspace_preserva_o_prefixo() {
    let mut app = app_navegando();
    app.on_key(ctrl('p'));
    for caractere in "select 1".chars() {
        app.on_key(tecla(caractere));
    }
    assert_eq!(app.commands.as_ref().expect("pop-up").input, ";select 1");

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(app.commands.as_ref().expect("pop-up").input, ";select ");

    // Apagar tudo não tira o `;` inicial.
    for _ in 0.."select ".len() + 1 {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    assert_eq!(app.commands.as_ref().expect("pop-up").input, ";");
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
fn ponto_e_virgula_fora_do_popup_nao_abre_o_comando() {
    // O `;` só seleciona o comando com o pop-up aberto; fora dele é tecla morta.
    let mut app = app_navegando();
    app.on_key(tecla(';'));
    assert!(app.commands.is_none());

    let mut app = app_com_grade();
    app.on_key(tecla(';'));
    assert!(app.commands.is_none(), "nem nos registros abre por fora");
}

#[test]
fn fora_das_tabelas_o_ctrl_p_nao_abre() {
    let mut app = app_nos_bancos();
    app.on_key(ctrl('p'));
    assert!(app.commands.is_none(), "nos bancos não abre");

    let mut app = app_nas_conexoes();
    app.on_key(ctrl('p'));
    assert!(app.commands.is_none(), "nas conexões não abre");
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
fn resposta_da_consulta_vira_a_grade_sem_paginacao() {
    let mut app = app_com_grade();
    app.on_response(Response::Query {
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
