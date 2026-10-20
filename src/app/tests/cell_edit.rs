use super::*;

#[test]
fn edicao_passa_por_duas_confirmacoes() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Edit);
    let edit = app.edit.as_ref().expect("edição aberta");
    assert_eq!(edit.original, "1");
    assert_eq!(edit.input, "1");

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    app.on_key(KeyEvent::from(KeyCode::Char('9')));
    assert_eq!(app.edit.as_ref().unwrap().input, "9");

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Confirm);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::ConfirmAgain);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);
    assert!(app.edit.is_none());
}

#[test]
fn esc_na_confirmacao_volta_a_editar() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Confirm);

    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Edit);
    assert!(app.edit.is_some());
}

#[test]
fn tabela_sem_chave_primaria_nao_abre_edicao() {
    let mut app = app_com_grade();
    if let Some(data) = app.data.as_mut() {
        data.primary_key.clear();
    }
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);
    assert!(app.edit.is_none());
    assert!(matches!(app.status, Status::Notice(_)));
}
