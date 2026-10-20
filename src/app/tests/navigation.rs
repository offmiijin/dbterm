use super::*;

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
fn teclas_navegam_selecionam_e_saem() {
    let mut app = app();
    app.focus = Focus::Tables;
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

    // `h`/`l` não trocam mais de painel: nas tabelas eles não fazem nada.
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert!(app.focus == Focus::Tables);

    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.should_quit);
}

#[test]
fn esc_sobe_um_nivel_por_vez_ate_pedir_confirmacao() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;

    // Nos registros, `esc` volta às tabelas em vez de sair.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Tables);
    assert_eq!(app.mode, Mode::Rows);
    assert!(!app.should_quit);

    // Nas tabelas, sobe para os bancos; e depois para as conexões.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Databases);
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Connections);

    // Na raiz da cascata, `esc` abre a confirmação em vez de sair na hora.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);

    // `esc` na confirmação cancela e volta às conexões.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Rows);
    assert!(app.focus == Focus::Connections);
    assert!(!app.should_quit);

    // Confirmar com `enter` sai da aplicação.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.should_quit);
}

#[test]
fn teclas_c_b_t_levam_ao_nivel() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    app.column = Some(0);

    // Sair dos registros pelos chips fecha o modo coluna.
    app.on_key(KeyEvent::from(KeyCode::Char('c')));
    assert!(app.focus == Focus::Connections);
    assert_eq!(app.mode, Mode::Rows);
    assert_eq!(app.column, None);

    app.on_key(KeyEvent::from(KeyCode::Char('b')));
    assert!(app.focus == Focus::Databases);
    app.on_key(KeyEvent::from(KeyCode::Char('t')));
    assert!(app.focus == Focus::Tables);
}

#[test]
fn j_k_andam_nas_conexoes_e_nos_bancos() {
    let mut app = app_com_rapidas(&["a", "b"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.connections_state.selected(), Some(1));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.connections_state.selected(), Some(1), "clampa no fim");
    app.on_key(KeyEvent::from(KeyCode::Char('g')));
    assert_eq!(app.connections_state.selected(), Some(0));

    app.focus = Focus::Databases;
    app.databases = vec![String::from("app"), String::from("outro")];
    app.databases_state.select(Some(0));
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert_eq!(app.databases_state.selected(), Some(1));
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(app.databases_state.selected(), Some(0));
}

#[test]
fn enter_nas_conexoes_abre_a_conexao_selecionada() {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    app.databases = vec![String::from("app")];
    app.tables = vec![String::from("users")];

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(matches!(app.status, Status::Connecting));
    assert!(app.databases.is_empty(), "bancos da conexão anterior somem");
    assert!(app.tables.is_empty(), "tabelas da conexão anterior somem");
}

#[test]
fn conectado_pede_os_bancos_e_desce_para_eles() {
    let mut app = app_form();
    app.connecting = true;
    app.on_response(Response::Connected {
        server: String::from("MySQL 8.0.35"),
        database: String::from("app"),
    });
    assert!(app.focus == Focus::Databases);
    assert!(matches!(app.status, Status::Loading(ref o) if o == "bancos"));
}

#[test]
fn bancos_selecionam_o_schema_atual_e_trocam_por_ele() {
    let mut app = app();
    app.database = String::from("outro");
    app.on_response(Response::Databases(vec![
        String::from("app"),
        String::from("outro"),
    ]));
    assert_eq!(app.databases_state.selected(), Some(1));
    assert!(matches!(app.status, Status::Loading(ref o) if o == "outro"));
}

#[test]
fn sem_schema_atual_o_primeiro_banco_e_escolhido() {
    let mut app = app();
    app.database.clear();
    app.on_response(Response::Databases(vec![
        String::from("clientes"),
        String::from("outro"),
    ]));
    assert_eq!(app.databases_state.selected(), Some(0));
    assert!(matches!(app.status, Status::Loading(ref o) if o == "clientes"));
}

#[test]
fn sem_bancos_a_lista_fica_vazia_e_sem_selecao() {
    let mut app = app();
    app.on_response(Response::Databases(Vec::new()));
    assert_eq!(app.databases_state.selected(), None);
    assert!(matches!(app.status, Status::Ready));
}

#[test]
fn trocar_de_banco_pede_as_tabelas_do_novo() {
    let mut app = app_com_grade();
    app.on_response(Response::Database(String::from("outro")));
    assert_eq!(app.database, "outro");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "tabelas"));
}

#[test]
fn enter_no_banco_atual_desce_para_as_tabelas() {
    let mut app = app_com_grade();
    app.database = String::from("app");
    app.focus = Focus::Databases;
    app.databases = vec![String::from("app")];
    app.databases_state.select(Some(0));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Tables);
}

#[test]
fn q_em_colunas_confirma_e_cancelar_volta_ao_modo_coluna() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);

    // `q` pede confirmação em vez de sair direto.
    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);

    // Cancelar restaura o modo de origem, preservando a coluna selecionada.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Columns);
    assert_eq!(app.column, Some(0));
    assert!(!app.should_quit);
}

#[test]
fn enter_troca_de_painel_e_h_l_nao() {
    let mut app = app_com_grade();
    app.focus = Focus::Tables;

    // `h`/`l` não trocam mais de painel; quem leva aos dados é o `enter`.
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Data);

    // De volta às tabelas, só com `esc`.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Tables);

    // Tab foi removido da troca de foco entre os painéis.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::BackTab));
    assert!(app.focus == Focus::Tables);
}

#[test]
fn h_l_nos_registros_rolam_as_colunas() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    // `h`/`l` andam na horizontal pelas colunas, do mesmo jeito que as setas.
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert_eq!(app.hscroll, 1);
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert_eq!(app.hscroll, 1, "clampa na última coluna");
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert_eq!(app.hscroll, 0);
    app.on_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(app.hscroll, 0, "clampa na primeira coluna");
    // Rolar não mexe na seleção nem no modo.
    assert_eq!(app.column, None);
    assert_eq!(app.mode, Mode::Rows);
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
    app.focus = Focus::Tables;
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
        offset: 0,
        limit: ROW_LIMIT,
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
        offset: 0,
        limit: ROW_LIMIT,
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
