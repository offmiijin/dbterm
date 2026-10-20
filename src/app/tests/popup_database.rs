use super::*;

#[test]
fn resposta_com_as_propriedades_abre_o_popup_do_banco() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    let edicao = app.database_edit.as_ref().expect("pop-up aberto");
    assert_eq!(edicao.field, DatabaseField::Charset);
    assert_eq!(edicao.values.name, "app");
    assert_eq!(edicao.values.charset, "utf8mb4");
    assert_eq!(edicao.values.collation, "utf8mb4_0900_ai_ci");
    assert_eq!(
        edicao.fields(),
        vec![
            DatabaseField::Charset,
            DatabaseField::Collation,
            DatabaseField::ReadOnly
        ]
    );
}

#[test]
fn servidor_sem_opcao_de_leitura_esconde_o_campo() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(None)));

    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").fields(),
        vec![DatabaseField::Charset, DatabaseField::Collation]
    );

    // Tab dá a volta só nos campos que o servidor informa.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Collation
    );
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Charset
    );
}

#[test]
fn enter_abre_a_lista_do_charset_no_valor_atual() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    app.on_key(KeyEvent::from(KeyCode::Enter));

    let lista = app
        .database_edit
        .as_ref()
        .expect("pop-up")
        .picker
        .as_ref()
        .expect("lista aberta");
    assert_eq!(lista.field, DatabaseField::Charset);
    assert_eq!(lista.items, ["latin1", "utf8mb4"]);
    assert_eq!(lista.index, 1, "a lista abre no valor atual");
}

#[test]
fn escolher_charset_leva_a_collation_padrao_dele() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let edicao = app.database_edit.as_ref().expect("pop-up");
    assert!(edicao.picker.is_none(), "a lista fecha ao escolher");
    assert_eq!(edicao.values.charset, "latin1");
    assert_eq!(edicao.values.collation, "latin1_swedish_ci");
}

#[test]
fn lista_de_collation_traz_so_o_charset_escolhido() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let lista = app
        .database_edit
        .as_ref()
        .expect("pop-up")
        .picker
        .as_ref()
        .expect("lista aberta");
    assert_eq!(lista.field, DatabaseField::Collation);
    assert_eq!(lista.items, ["utf8mb4_0900_ai_ci", "utf8mb4_bin"]);
    assert_eq!(lista.index, 0);

    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").values.collation,
        "utf8mb4_bin"
    );
}

#[test]
fn j_k_na_lista_param_nos_limites() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit
            .as_ref()
            .expect("pop-up")
            .picker
            .as_ref()
            .expect("lista")
            .index,
        1,
        "clampa no fim"
    );
    for _ in 0..3 {
        app.on_key(KeyEvent::from(KeyCode::Char('k')));
    }
    assert_eq!(
        app.database_edit
            .as_ref()
            .expect("pop-up")
            .picker
            .as_ref()
            .expect("lista")
            .index,
        0,
        "clampa no começo"
    );
}

#[test]
fn ctrl_hjkl_anda_na_lista_do_popup() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    let indice = |app: &App| {
        app.database_edit
            .as_ref()
            .expect("pop-up")
            .picker
            .as_ref()
            .expect("lista")
            .index
    };

    app.on_key(ctrl('k'));
    assert_eq!(indice(&app), 0, "a lista abre no valor atual");
    app.on_key(ctrl('l'));
    assert_eq!(indice(&app), 1);
    app.on_key(ctrl('h'));
    assert_eq!(indice(&app), 0);
    app.on_key(ctrl('j'));
    assert_eq!(indice(&app), 1);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").values.charset,
        "utf8mb4"
    );
}

#[test]
fn esc_na_lista_volta_ao_popup_sem_escolher() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    let edicao = app.database_edit.as_ref().expect("pop-up continua aberto");
    assert!(edicao.picker.is_none());
    assert_eq!(edicao.values.charset, "utf8mb4");
}

#[test]
fn enter_na_opcao_de_somente_leitura_alterna() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    // `j` anda nos campos como no resto do app.
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::ReadOnly
    );

    app.on_key(KeyEvent::from(KeyCode::Enter));
    let edicao = app.database_edit.as_ref().expect("pop-up");
    assert!(edicao.picker.is_none(), "sim/não não abre lista");
    assert_eq!(edicao.values.read_only, Some(true));

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").values.read_only,
        Some(false)
    );
}

#[test]
fn j_k_andam_nos_campos_do_popup_do_banco() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Collation
    );
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Charset
    );

    // `k` no primeiro campo dá a volta e cai no último.
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::ReadOnly
    );
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Charset
    );
}

#[test]
fn s_sozinho_nao_salva_no_popup_do_banco() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Char('s')));
    assert!(app.database_edit.is_some(), "s sozinho não salva");
    // `enter` abre a lista em vez de salvar.
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.database_edit.is_some(), "enter não salva");
    assert!(app.database_edit.as_ref().expect("pop-up").picker.is_some());
}

fn ctrl_s_no_popup_do_banco_fecha_sem_mudanca() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    app.on_key(ctrl('s'));

    assert!(app.database_edit.is_none());
    assert!(!matches!(app.status, Status::Failed(_)));
}

#[test]
fn esc_no_popup_do_banco_descarta() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Char('x')));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    assert!(app.database_edit.is_none());
    assert!(!app.should_quit);
    assert!(app.screen == Screen::Browse);
    assert!(app.databases_state.selected() == Some(0));
}

#[test]
fn banco_atualizado_vira_aviso_no_rodape() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseUpdated {
        database: String::from("app"),
    });
    assert!(matches!(app.status, Status::Notice(ref aviso) if aviso == "banco app atualizado"));
}

#[test]
fn ctrl_hjkl_anda_nos_campos_do_popup_do_banco() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    let campo = |app: &App| app.database_edit.as_ref().expect("pop-up").field;

    app.on_key(ctrl('j'));
    assert_eq!(campo(&app), DatabaseField::Collation);
    app.on_key(ctrl('l'));
    assert_eq!(campo(&app), DatabaseField::ReadOnly);
    app.on_key(ctrl('k'));
    assert_eq!(campo(&app), DatabaseField::Collation);
    app.on_key(ctrl('h'));
    assert_eq!(campo(&app), DatabaseField::Charset);
    // `ctrl+h` no primeiro campo dá a volta e cai no último.
    app.on_key(ctrl('h'));
    assert_eq!(campo(&app), DatabaseField::ReadOnly);
}
