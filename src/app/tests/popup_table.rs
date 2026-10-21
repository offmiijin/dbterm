use super::*;

#[test]
fn e_nas_tabelas_pede_as_propriedades() {
    let mut app = app_nas_tabelas();
    app.on_key(tecla('e'));

    assert!(matches!(app.status, Status::Loading(ref o) if o == "tabela users"));
    assert!(app.table_edit.is_none(), "o pop-up espera a resposta");
}

#[test]
fn resposta_com_as_propriedades_abre_o_popup_da_tabela() {
    let app = app_com_popup_de_tabela();

    let edicao = app.table_edit.as_ref().expect("pop-up aberto");
    assert_eq!(edicao.field, TableField::Name);
    assert_eq!(edicao.values.name, "users");
    assert_eq!(edicao.values.engine, "InnoDB");
    assert_eq!(edicao.values.row_format, "Dynamic");
    assert_eq!(edicao.auto_increment, "1024", "o número vem como texto");
    assert_eq!(
        edicao.fields(),
        vec![
            TableField::Name,
            TableField::Comment,
            TableField::Engine,
            TableField::RowFormat,
            TableField::AutoIncrement,
            TableField::Charset,
            TableField::Collation
        ]
    );
}

#[test]
fn tabela_sem_auto_increment_esconde_o_campo() {
    let mut app = app_nas_tabelas();
    let mut info = info_tabela();
    info.auto_increment = None;
    app.on_response(Response::TableInfo(info));

    assert!(
        !app.table_edit
            .as_ref()
            .expect("pop-up")
            .fields()
            .contains(&TableField::AutoIncrement)
    );

    // `tab` pula direto do formato de linha para o charset.
    app.table_edit.as_mut().expect("pop-up").field = TableField::RowFormat;
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").field,
        TableField::Charset
    );
}

#[test]
fn ctrl_hjkl_anda_nos_campos_do_popup_da_tabela() {
    let mut app = app_com_popup_de_tabela();
    let campo = |app: &App| app.table_edit.as_ref().expect("pop-up").field;

    app.on_key(ctrl('l'));
    assert_eq!(campo(&app), TableField::Comment);
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").values.name,
        "users",
        "`ctrl` não digita no campo de texto"
    );
    app.on_key(ctrl('j'));
    assert_eq!(campo(&app), TableField::Engine);
    app.on_key(ctrl('k'));
    assert_eq!(campo(&app), TableField::Comment);
    app.on_key(ctrl('h'));
    assert_eq!(campo(&app), TableField::Name);
}

#[test]
fn ctrl_maiusculo_anda_nos_campos_como_minusculo() {
    let mut app = app_com_popup_de_tabela();
    let campo = |app: &App| app.table_edit.as_ref().expect("pop-up").field;

    // Com o teclado estendido, o `shift` chega no maiúsculo.
    app.on_key(ctrl('L'));
    assert_eq!(campo(&app), TableField::Comment);
    app.on_key(ctrl('J'));
    assert_eq!(campo(&app), TableField::Engine);
    app.on_key(ctrl('K'));
    assert_eq!(campo(&app), TableField::Comment);
    app.on_key(ctrl('H'));
    assert_eq!(campo(&app), TableField::Name);
}

#[test]
fn digitacao_edita_o_nome_e_o_comentario() {
    let mut app = app_com_popup_de_tabela();

    // No campo de texto `s` é texto: não grava.
    app.on_key(tecla('s'));
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").values.name,
        "userss"
    );
    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").values.name,
        "users"
    );

    // `j`/`k` também são texto: quem anda nos campos é `tab` e as setas.
    app.on_key(tecla('j'));
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").field,
        TableField::Name
    );
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").values.name,
        "usersj"
    );

    app.on_key(KeyEvent::from(KeyCode::Tab));
    app.on_key(tecla('x'));
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").values.comment,
        "cadastrox"
    );
}

#[test]
fn ctrl_s_no_campo_de_texto_salva_e_fecha() {
    let mut app = app_com_popup_de_tabela();
    for _ in 0.."users".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    for caractere in "clientes".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl('s'));

    assert!(app.table_edit.is_none(), "o pop-up fecha ao salvar");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "tabela users"));
}

#[test]
fn ctrl_s_no_campo_de_lista_salva() {
    let mut app = app_com_popup_de_tabela();
    app.on_key(KeyEvent::from(KeyCode::Tab));
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.table_edit.as_ref().expect("pop-up").field,
        TableField::Engine
    );

    app.on_key(ctrl('s'));

    assert!(app.table_edit.is_none());
}

#[test]
fn enter_e_s_sozinhos_nao_salva_no_popup_da_tabela() {
    let mut app = app_com_popup_de_tabela();
    // No campo de texto `enter` não salva mais.
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.table_edit.is_some(), "enter não salva");
    // No campo de lista `s` sem `ctrl` não salva e `enter` abre a lista.
    app.table_edit.as_mut().expect("pop-up").field = TableField::Engine;
    app.on_key(tecla('s'));
    assert!(app.table_edit.is_some(), "s sozinho não salva");
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.table_edit.as_ref().expect("pop-up").picker.is_some());
}

fn ctrl_s_sem_mudanca_fecha_sem_pedir_alter() {
    let mut app = app_com_popup_de_tabela();
    app.on_key(ctrl('s'));
    assert!(app.table_edit.is_none());
    assert!(!matches!(app.status, Status::Failed(_)));
}

#[test]
fn enter_no_campo_de_lista_abre_a_lista_no_valor_atual() {
    let mut app = app_com_popup_de_tabela();
    app.table_edit.as_mut().expect("pop-up").field = TableField::Engine;

    app.on_key(KeyEvent::from(KeyCode::Enter));

    let lista = app
        .table_edit
        .as_ref()
        .expect("pop-up")
        .picker
        .as_ref()
        .expect("lista aberta");
    assert_eq!(lista.field, TableField::Engine);
    assert_eq!(lista.items, ["InnoDB", "MyISAM"]);
    assert_eq!(lista.index, 0, "a lista abre no valor atual");
}

#[test]
fn escolher_engine_e_formato_de_linha_muda_so_o_campo() {
    let mut app = app_com_popup_de_tabela();
    app.table_edit.as_mut().expect("pop-up").field = TableField::Engine;
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(tecla('j'));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let edicao = app.table_edit.as_ref().expect("pop-up");
    assert!(edicao.picker.is_none(), "a lista fecha ao escolher");
    assert_eq!(edicao.values.engine, "MyISAM");
    assert_eq!(edicao.values.row_format, "Dynamic", "o resto não muda");
}

#[test]
fn escolher_charset_da_tabela_leva_a_collation_padrao() {
    let mut app = app_com_popup_de_tabela();
    app.table_edit.as_mut().expect("pop-up").field = TableField::Charset;
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(tecla('k'));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let edicao = app.table_edit.as_ref().expect("pop-up");
    assert_eq!(edicao.values.charset, "latin1");
    assert_eq!(edicao.values.collation, "latin1_swedish_ci");
}

#[test]
fn lista_de_collation_da_tabela_filtra_pelo_charset() {
    let mut app = app_com_popup_de_tabela();
    app.table_edit.as_mut().expect("pop-up").field = TableField::Collation;
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let lista = app
        .table_edit
        .as_ref()
        .expect("pop-up")
        .picker
        .as_ref()
        .expect("lista aberta");
    assert_eq!(lista.items, ["utf8mb4_0900_ai_ci", "utf8mb4_bin"]);
}

#[test]
fn esc_na_lista_da_tabela_volta_ao_popup() {
    let mut app = app_com_popup_de_tabela();
    app.table_edit.as_mut().expect("pop-up").field = TableField::Charset;
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(tecla('k'));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    let edicao = app.table_edit.as_ref().expect("pop-up continua aberto");
    assert!(edicao.picker.is_none());
    assert_eq!(edicao.values.charset, "utf8mb4");
}

#[test]
fn esc_no_popup_da_tabela_descarta() {
    let mut app = app_com_popup_de_tabela();
    app.on_key(tecla('x'));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    assert!(app.table_edit.is_none());
    assert!(!app.should_quit);
    assert!(app.screen == Screen::Browse);
    assert_eq!(app.tables[1], "users");
}

#[test]
fn auto_increment_invalido_mantem_o_popup_aberto() {
    let mut app = app_com_popup_de_tabela();
    app.table_edit.as_mut().expect("pop-up").field = TableField::AutoIncrement;
    for _ in 0.."1024".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    for caractere in "abc".chars() {
        app.on_key(tecla(caractere));
    }
    app.on_key(ctrl('s'));

    assert!(app.table_edit.is_some(), "o pop-up continua aberto");
    assert!(
        matches!(app.status, Status::Failed(ref erro) if erro == "auto-increment inválido: abc")
    );
}

#[test]
fn auto_increment_apagado_nao_muda_o_valor() {
    let mut app = app_com_popup_de_tabela();
    let mut edicao = app.table_edit.take().expect("pop-up");
    edicao.auto_increment.clear();
    assert!(edicao.alter().expect("sem erro").is_empty());

    edicao.auto_increment = String::from("2048");
    assert_eq!(edicao.alter().expect("sem erro").auto_increment, Some(2048));

    // Um nome novo entra como rename, e só ele.
    edicao.values.name = String::from("clientes");
    let alter = edicao.alter().expect("sem erro");
    assert_eq!(alter.rename.as_deref(), Some("clientes"));
    assert_eq!(alter.auto_increment, Some(2048));
}

#[test]
fn tabela_renomeada_atualiza_a_lista_e_a_tabela_aberta() {
    let mut app = app_nas_tabelas();
    app.current = Some(String::from("users"));

    app.on_response(Response::TableUpdated {
        before: String::from("users"),
        after: String::from("clientes"),
    });

    assert_eq!(app.tables, ["clientes", "orders"], "a lista segue ordenada");
    assert_eq!(app.list_state.selected(), Some(0), "a seleção acompanha");
    assert_eq!(app.current.as_deref(), Some("clientes"));
    assert!(
        matches!(app.status, Status::Notice(ref aviso) if aviso == "tabela clientes atualizada")
    );
}

#[test]
fn nome_vazio_mantem_o_popup_aberto() {
    let mut app = app_com_popup_de_tabela();
    for _ in 0.."users".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    app.on_key(ctrl('s'));

    assert!(app.table_edit.is_some(), "o pop-up continua aberto");
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "informe o nome da tabela"));
    assert_eq!(app.tables[1], "users", "nada foi renomeado");
}
