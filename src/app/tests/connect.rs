use super::*;

#[test]
fn sem_url_o_app_abre_nas_conexoes_rapidas() {
    let app = app_quick();
    assert!(app.screen == Screen::Quick);
    assert!(app.quick.is_none(), "sem histórico não há seleção");
}

#[test]
fn com_url_o_app_vai_direto_para_a_tela_de_trabalho() {
    assert!(app().screen == Screen::Browse);
}

#[test]
fn rapidas_selecionam_a_mais_recente() {
    let app = app_com_rapidas(&["a", "b"]);
    assert_eq!(app.quick, Some(0));
    assert_eq!(app.history.recent()[0].host, "a");
}

#[test]
fn n_abre_o_formulario_vazio() {
    let mut app = app_com_rapidas(&["a"]);
    app.on_key(KeyEvent::from(KeyCode::Char('n')));
    assert!(app.screen == Screen::Connect);
    assert_eq!(app.field, Field::Host);
    assert_eq!(app.connection.value(Field::Host), "127.0.0.1");
    assert_eq!(app.connection.value(Field::Database), "");
}

#[test]
fn enter_abre_a_rapida_preenchida_com_foco_na_senha() {
    let mut app = app_com_rapidas(&["db.local"]);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.screen == Screen::Connect);
    assert_eq!(app.field, Field::Password);
    assert_eq!(app.connection.value(Field::Host), "db.local");
    assert_eq!(app.connection.value(Field::Port), "3306");
    assert_eq!(app.connection.value(Field::User), "root");
    assert_eq!(app.connection.value(Field::Database), "app");
    assert_eq!(app.connection.value(Field::Password), "");
}

#[test]
fn j_k_anda_pelas_rapidas() {
    let mut app = app_com_rapidas(&["a", "b"]);
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.quick, Some(1));
    app.on_key(KeyEvent::from(KeyCode::Down));
    assert_eq!(app.quick, Some(1), "clampa no fim");
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(app.quick, Some(0));
}

#[test]
fn esc_nas_rapidas_sai() {
    let mut app = app_quick();
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.should_quit);
}

#[test]
fn esc_no_formulario_volta_para_as_rapidas() {
    let mut app = app_form();
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.screen == Screen::Quick);
    assert!(!app.should_quit);
}

#[test]
fn tab_anda_entre_os_campos_dando_a_volta() {
    let mut app = app_form();
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(app.field, Field::Port);
    app.on_key(KeyEvent::from(KeyCode::Down));
    assert_eq!(app.field, Field::User);
    app.on_key(KeyEvent::from(KeyCode::BackTab));
    assert_eq!(app.field, Field::Port);

    // Voltar do primeiro campo cai no último.
    app.field = Field::Host;
    app.on_key(KeyEvent::from(KeyCode::Up));
    assert_eq!(app.field, Field::Database);
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(app.field, Field::Host);
}

#[test]
fn digitar_edita_o_campo_com_foco_sem_sair() {
    let mut app = app_form();
    app.field = Field::Database;
    // `q` e `j` são texto na tela de conexão.
    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.connection.value(Field::Database), "qj");
    assert!(!app.should_quit);

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(app.connection.value(Field::Database), "q");
}

#[test]
fn formulario_incompleto_mostra_o_erro_e_nao_conecta() {
    let mut app = app_form();
    app.connection.value_mut(Field::User).clear();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.connect_error.as_deref(), Some("informe o usuário"));
    assert!(app.screen == Screen::Connect);
    assert!(!app.connecting);
}

#[test]
fn formulario_sem_schema_conecta_e_escolhe_o_banco_depois() {
    let mut app = app_form();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.connect_error.is_none());
    assert!(app.connecting);
}

#[test]
fn formulario_valido_pede_a_conexao() {
    let mut app = app_form();
    app.field = Field::Database;
    app.on_key(KeyEvent::from(KeyCode::Char('a')));
    app.on_key(KeyEvent::from(KeyCode::Char('p')));
    app.on_key(KeyEvent::from(KeyCode::Char('p')));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.connect_error.is_none());
    assert!(app.connecting);
}

#[test]
fn enquanto_conecta_o_teclado_nao_edita() {
    let mut app = app_form();
    app.connecting = true;
    app.field = Field::Host;
    app.on_key(KeyEvent::from(KeyCode::Char('x')));
    assert_eq!(app.connection.value(Field::Host), "127.0.0.1");
}

#[test]
fn esc_durante_a_conexao_abandona_e_volta() {
    let mut app = app_form();
    app.connecting = true;
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.screen == Screen::Quick);
    assert!(!app.connecting);
}

#[test]
fn conexao_aceita_vai_para_a_grade_e_entra_nas_rapidas() {
    let mut app = app_form();
    app.connecting = true;
    app.on_response(Response::Connected {
        server: String::from("MySQL 8.0.35"),
        database: String::from("app"),
    });
    assert!(app.screen == Screen::Browse);
    assert!(!app.connecting);
    assert_eq!(app.server, "MySQL 8.0.35");
    assert_eq!(app.database, "app");
    assert_eq!(app.history.recent().len(), 1);
    assert_eq!(app.history.recent()[0].host, "127.0.0.1");
}

#[test]
fn conexao_recusada_volta_ao_formulario_com_o_motivo() {
    let mut app = app_form();
    app.connecting = true;
    app.on_response(Response::Failed(String::from("falha ao conectar")));
    assert!(app.screen == Screen::Connect);
    assert!(!app.connecting);
    assert_eq!(app.connect_error.as_deref(), Some("falha ao conectar"));
    assert!(app.history.recent().is_empty(), "falha não salva conexão");
}

#[test]
fn enter_abre_a_rapida_com_a_senha_salva() {
    let mut app = app_quick();
    app.history = History::with(vec![Saved {
        engine: String::from(Saved::MYSQL),
        host: String::from("db.local"),
        port: 3306,
        user: String::from("root"),
        database: String::from("app"),
        password: Some(String::from("segredo")),
        last_used: 1,
    }]);
    app.quick = Some(0);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.connection.value(Field::Password), "segredo");
}

#[test]
fn setas_horizontais_rolam_as_colunas_da_grade() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.hscroll, 1);
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.hscroll, 1, "clampa na última coluna");
    app.on_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(app.hscroll, 0);
    app.on_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(app.hscroll, 0, "clampa na primeira coluna");

    // Na lista de tabelas as setas horizontais não rolam a grade.
    app.focus = Focus::Tables;
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.hscroll, 0);
}

#[test]
fn tabela_nova_volta_a_rolagem_horizontal_ao_inicio() {
    let mut app = app_com_grade();
    app.hscroll = 1;
    app.on_response(Response::Rows {
        table: String::from("outra"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id"), String::from("name")],
            kinds: vec![ColumnKind::Number, ColumnKind::Text],
            rows: vec![vec![String::from("1"), String::from("ana")]],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.hscroll, 0);
}
