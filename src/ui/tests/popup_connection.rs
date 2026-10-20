use super::*;

#[test]
fn popup_de_conexao_mostra_servidor_e_autenticacao_agrupados() {
    let mut app = app_com_popup_de_conexao();
    let terminal = terminal_de(&mut app, 64, 12);
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);

    assert!(tela.contains("\u{f044} Editar"), "{tela}");
    assert!(tela.contains("Servidor"), "{tela}");
    assert!(tela.contains("Autenticação"), "{tela}");
    assert!(tela.contains("s salvar · esc cancelar"), "{tela}");

    // Host, porta e banco em linhas seguidas, dentro do grupo "Servidor".
    let (_, yhost) = achar(buffer, "Host");
    let (_, yporta) = achar(buffer, "Porta");
    let (_, ybanco) = achar(buffer, "Banco");
    assert_eq!(yporta, yhost + 1, "porta logo abaixo do host");
    assert_eq!(ybanco, yporta + 1, "banco abaixo da porta");

    // O grupo da autenticação vem depois, com a senha abaixo do usuário.
    let (_, yusuario) = achar(buffer, "Usuário");
    let (_, ysenha) = achar(buffer, "Senha");
    assert!(yusuario > ybanco, "autenticação abaixo do servidor");
    assert_eq!(ysenha, yusuario + 1, "senha abaixo do usuário");
}

#[test]
fn popup_de_conexao_nao_mostra_a_senha_em_claro() {
    let mut app = app_com_popup_de_conexao();
    let tela = screen(&mut app);
    assert!(!tela.contains("segredo"), "{tela}");
    assert!(tela.contains("•••••••"), "{tela}");
}

#[test]
fn popup_de_conexao_destaca_o_campo_em_edicao() {
    let mut app = app_com_popup_de_conexao();
    let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();

    // O valor do campo com foco tem fundo próprio e o cursor no fim.
    let (x, y) = achar(buffer, "db.local");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::SURFACE0);
    let linha: String = (x..x + 20)
        .map(|coluna| buffer.cell((coluna, y)).unwrap().symbol().to_string())
        .collect();
    assert!(linha.contains('▏'), "{linha}");

    // O campo sem foco não tem fundo.
    let (x, y) = achar(buffer, "app");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::BASE, "banco");
}

#[test]
fn popup_de_conexao_corta_o_valor_longo_sem_quebrar_o_layout() {
    let mut app = app_com_popup_de_conexao();
    if let Some(edit) = app.connection_edit.as_mut() {
        *edit.connection.value_mut(Field::Host) =
            String::from("db.producao.interno.empresa.com.br.exemplo.mais.longo");
    }
    let tela = screen(&mut app);
    // O valor grande não empurra as linhas: o corte fica no começo, onde o
    // cursor não está, e o pop-up mantém as ações no lugar.
    assert!(tela.contains('…'), "{tela}");
    assert!(tela.contains("s salvar · esc cancelar"), "{tela}");
    assert!(tela.contains("Senha"), "{tela}");
}
