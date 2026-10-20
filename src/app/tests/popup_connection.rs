use super::*;

#[test]
fn e_nas_conexoes_abre_o_popup_de_edicao_preenchido() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    let edicao = app.connection_edit.as_ref().expect("pop-up aberto");
    assert_eq!(edicao.saved, 0);
    assert_eq!(edicao.field, Field::Host);
    assert_eq!(edicao.connection.value(Field::Host), "db.local");
    assert_eq!(edicao.connection.value(Field::Port), "3306");
    assert_eq!(edicao.connection.value(Field::User), "root");
    assert_eq!(edicao.connection.value(Field::Database), "app");
}

#[test]
fn e_fora_das_conexoes_nao_abre_a_edicao() {
    let mut app = app_nas_conexoes();
    app.focus = Focus::Tables;
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    assert!(app.connection_edit.is_none());
    assert_eq!(app.mode, Mode::Rows);
}

#[test]
fn tab_anda_nos_campos_do_popup_na_ordem_da_tela() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));

    let mut ordem = vec![app.connection_edit.as_ref().expect("pop-up").field];
    for _ in 0..4 {
        app.on_key(KeyEvent::from(KeyCode::Tab));
        ordem.push(app.connection_edit.as_ref().expect("pop-up").field);
    }
    // Servidor (host, porta, banco) e depois autenticação (usuário, senha).
    assert_eq!(
        ordem,
        vec![
            Field::Host,
            Field::Port,
            Field::Database,
            Field::User,
            Field::Password
        ]
    );

    // Tab dá a volta; no pop-up, digitar texto anda com `tab` e setas.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.connection_edit.as_ref().expect("pop-up").field,
        Field::Host
    );
    app.on_key(KeyEvent::from(KeyCode::BackTab));
    assert_eq!(
        app.connection_edit.as_ref().expect("pop-up").field,
        Field::Password
    );
}

#[test]
fn ctrl_hjkl_anda_nos_campos_do_popup_da_conexao() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    let campo = |app: &App| app.connection_edit.as_ref().expect("pop-up").field;

    app.on_key(ctrl('l'));
    assert_eq!(campo(&app), Field::Port, "para a direita, a porta");
    app.on_key(ctrl('j'));
    assert_eq!(campo(&app), Field::Database);
    app.on_key(ctrl('j'));
    assert_eq!(campo(&app), Field::User);
    app.on_key(ctrl('j'));
    assert_eq!(campo(&app), Field::Password);
    app.on_key(ctrl('h'));
    assert_eq!(campo(&app), Field::User);
    app.on_key(ctrl('k'));
    assert_eq!(campo(&app), Field::Database);
    // Do banco, `ctrl+k` volta para a porta, um campo por vez.
    app.on_key(ctrl('k'));
    assert_eq!(campo(&app), Field::Port);

    // `ctrl` não digita no campo com foco.
    assert_eq!(
        app.connection_edit
            .as_ref()
            .expect("pop-up")
            .connection
            .value(Field::Host),
        "db.local"
    );
}

#[test]
fn digitar_no_popup_edita_o_campo_com_foco_sem_sair() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    // `q` e `j` são texto no pop-up.
    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.connection_edit
            .as_ref()
            .expect("pop-up")
            .connection
            .value(Field::Host),
        "db.localqj"
    );
    assert!(!app.should_quit);
    assert!(matches!(app.mode, Mode::Rows));

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(
        app.connection_edit
            .as_ref()
            .expect("pop-up")
            .connection
            .value(Field::Host),
        "db.localq"
    );
}

#[test]
fn s_e_enter_nao_salva_so_ctrl_s_salva() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    // `s` sem `ctrl` é texto no campo.
    app.on_key(KeyEvent::from(KeyCode::Char('s')));
    assert!(app.connection_edit.is_some(), "s sozinho não salva");
    assert_eq!(
        app.connection_edit
            .as_ref()
            .expect("pop-up")
            .connection
            .value(Field::Host),
        "db.locals"
    );
    // `enter` também não salva.
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.connection_edit.is_some(), "enter não salva");
    app.on_key(ctrl('s'));
    assert!(app.connection_edit.is_none(), "ctrl+s salva");
}

fn ctrl_s_grava_a_conexao_editada_no_mesmo_lugar_da_lista() {
    let mut app = app_nas_conexoes();
    app.focus = Focus::Connections;
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    for _ in 0.."db.local".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    for caractere in "novo.local".chars() {
        app.on_key(KeyEvent::from(KeyCode::Char(caractere)));
    }
    app.on_key(ctrl('s'));

    assert!(app.connection_edit.is_none(), "o pop-up fecha ao salvar");
    let hosts: Vec<&str> = app
        .history
        .all()
        .iter()
        .map(|saved| saved.host.as_str())
        .collect();
    assert_eq!(
        hosts,
        ["novo.local", "outro"],
        "a posição na lista não muda"
    );
}

#[test]
fn esc_no_popup_descarta_a_edicao() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    app.on_key(KeyEvent::from(KeyCode::Char('x')));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    assert!(app.connection_edit.is_none());
    assert!(!app.should_quit);
    assert!(app.screen == Screen::Browse);
    assert_eq!(app.history.all()[0].host, "db.local");
}

#[test]
fn porta_invalida_mantem_o_popup_aberto_com_o_motivo() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    app.on_key(KeyEvent::from(KeyCode::Tab));
    for _ in 0.."3306".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    for caractere in "abc".chars() {
        app.on_key(KeyEvent::from(KeyCode::Char(caractere)));
    }
    app.on_key(ctrl('s'));

    assert!(app.connection_edit.is_some());
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "porta inválida: abc"));
    assert_eq!(app.history.all()[0].port, 3306);
}
