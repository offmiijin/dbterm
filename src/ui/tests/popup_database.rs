use super::*;

#[test]
fn popup_do_banco_mostra_identificacao_padroes_e_opcoes() {
    let mut app = app_com_popup_de_banco(Some(false));
    let terminal = terminal_de(&mut app, 64, 12);
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);

    assert!(tela.contains("\u{f044} Editar"), "{tela}");
    assert!(tela.contains("Identificação"), "{tela}");
    assert!(tela.contains("Banco  app"), "{tela}");
    assert!(tela.contains("Tamanho  12 GB"), "{tela}");
    assert!(tela.contains("▾ utf8mb4"), "{tela}");
    assert!(tela.contains("Padrões"), "{tela}");
    assert!(tela.contains("utf8mb4_0900_ai_ci"), "{tela}");
    assert!(tela.contains("Opções"), "{tela}");
    assert!(tela.contains("Somente leitura  não"), "{tela}");
    assert!(tela.contains("s salvar · esc cancelar"), "{tela}");

    // A identificação vem antes dos padrões, e as opções por último.
    let banco = tela.find("Banco").expect("banco");
    let charset = tela.find("Charset").expect("charset");
    let opcoes = tela.find("Opções").expect("opções");
    assert!(banco < charset && charset < opcoes, "{tela}");
}

#[test]
fn popup_do_banco_esconde_as_opcoes_que_o_servidor_nao_informa() {
    let mut app = app_com_popup_de_banco(None);
    let tela = screen(&mut app);
    assert!(!tela.contains("Somente leitura"), "{tela}");
    assert!(!tela.contains("Opções"), "{tela}");
    assert!(tela.contains("Padrões"), "{tela}");
}

#[test]
fn popup_do_banco_destaca_o_campo_em_edicao() {
    let mut app = app_com_popup_de_banco(Some(true));
    let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();

    // O nome e o tamanho saem com a cor de valor preenchido; só o rótulo fica
    // apagado.
    for valor in ["app", "12 GB"] {
        let (x, y) = achar(buffer, valor);
        assert_eq!(
            buffer.cell((x, y)).unwrap().fg,
            crate::theme::TEXT,
            "{valor}"
        );
    }
    let (x, y) = achar(buffer, "Tamanho");
    assert_eq!(
        buffer.cell((x, y)).unwrap().fg,
        crate::theme::OVERLAY0,
        "rótulo"
    );

    // O charset tem o foco: fundo do campo e o marcador de lista.
    let (x, y) = achar(buffer, "▾ utf8mb4");
    assert_eq!(buffer.cell((x + 2, y)).unwrap().bg, crate::theme::SURFACE0);

    // O campo sem foco não tem fundo próprio.
    let (x, y) = achar(buffer, "utf8mb4_0900_ai_ci");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::BASE);

    // A opção ligada aparece como "sim", sem fundo de campo.
    let (x, y) = achar(buffer, "sim");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::BASE);
}

#[test]
fn lista_do_charset_mostra_as_opcoes_e_o_valor_atual_destacado() {
    let mut app = app_com_popup_de_banco(Some(false));
    if let Some(edicao) = app.database_edit.as_mut() {
        edicao.activate();
    }
    let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);

    // O pop-up dá lugar à lista, com o título do campo e as ações dela.
    assert!(tela.contains("\u{f0d7} Charset"), "{tela}");
    assert!(tela.contains("latin1"), "{tela}");
    assert!(tela.contains("utf8mb4"), "{tela}");
    assert!(tela.contains("enter escolher · esc voltar"), "{tela}");
    assert!(!tela.contains("Somente leitura"), "{tela}");

    // O item atual (utf8mb4) vem com o marcador e o fundo da seleção.
    let (x, y) = achar(buffer, "❯ utf8mb4");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::SURFACE0);
    // O outro item não tem destaque.
    let (x, y) = achar(buffer, "latin1");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::BASE);
}

#[test]
fn tamanho_do_banco_sai_em_unidade_legivel() {
    let tamanho = super::super::popup::tamanho;
    assert_eq!(tamanho(0), "0 B");
    assert_eq!(tamanho(512), "512 B");
    assert_eq!(tamanho(1536), "1.5 KB");
    assert_eq!(tamanho(1024 * 1024), "1.0 MB");
    assert_eq!(tamanho(742 * 1024 * 1024), "742 MB");
    assert_eq!(tamanho(12 * 1024 * 1024 * 1024), "12 GB");
}

#[test]
fn janela_da_lista_acompanha_o_item_destacado() {
    let janela = super::super::popup::janela;
    // Começo da lista: a janela fica presa no topo.
    assert_eq!(janela(0, 5, 100), 0);
    assert_eq!(janela(3, 5, 100), 1);
    // Seleção no meio: ela fica no meio da janela.
    assert_eq!(janela(50, 5, 100), 48);
    // Fim da lista: a janela encolhe para caber.
    assert_eq!(janela(99, 5, 100), 95);
    // Lista menor que a janela: começa do topo.
    assert_eq!(janela(1, 5, 3), 0);
}
