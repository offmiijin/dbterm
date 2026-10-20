use super::*;

#[test]
fn popup_da_tabela_mostra_identificacao_armazenamento_e_padroes() {
    let mut app = app_com_popup_de_tabela();
    let terminal = terminal_de(&mut app, 64, 20);
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);

    assert!(tela.contains("\u{f044} Editar"), "{tela}");
    assert!(tela.contains("Identificação"), "{tela}");
    assert!(tela.contains("Tabela  users▏"), "{tela}");
    assert!(tela.contains("Tamanho  12 GB"), "{tela}");
    // O comentário fica na frente do rótulo, com uma linha extra abaixo.
    assert!(tela.contains("Comentário  cadastro"), "{tela}");
    assert!(tela.contains("Armazenamento"), "{tela}");
    assert!(tela.contains("▾ Dynamic"), "{tela}");
    assert!(tela.contains("Auto-increment  1024"), "{tela}");
    assert!(tela.contains("Padrões de texto"), "{tela}");
    assert!(tela.contains("▾ utf8mb4"), "{tela}");
    assert!(tela.contains("utf8mb4_0900_ai_ci"), "{tela}");
    // No campo de texto com foco, `enter` salva.
    assert!(tela.contains("enter salvar · esc cancelar"), "{tela}");

    // A identificação vem antes do armazenamento, e os padrões por último.
    let identificacao = tela.find("Identificação").expect("identificação");
    let armazenamento = tela.find("Armazenamento").expect("armazenamento");
    let padroes = tela.find("Padrões de texto").expect("padrões");
    assert!(
        identificacao < armazenamento && armazenamento < padroes,
        "{tela}"
    );

    // O nome ocupa a linha inteira e o tamanho vem logo abaixo.
    let (_, ynome) = achar(buffer, "Tabela");
    let (_, ytamanho) = achar(buffer, "Tamanho");
    assert_eq!(ytamanho, ynome + 1, "tamanho abaixo do nome");

    // O texto fica na linha do rótulo e a caixa desce só uma linha.
    let (_, ycomentario) = achar(buffer, "Comentário");
    let (_, ycadastro) = achar(buffer, "cadastro");
    let (_, yarmazenamento) = achar(buffer, "Armazenamento");
    assert_eq!(ycadastro, ycomentario, "texto na frente do rótulo");
    assert_eq!(
        yarmazenamento,
        ycomentario + 3,
        "duas linhas de caixa abaixo do rótulo"
    );
}

#[test]
fn caixa_do_comentario_vai_ate_a_borda_direita() {
    let mut app = app_com_popup_de_tabela();
    if let Some(edicao) = app.table_edit.as_mut() {
        edicao.field = crate::config::TableField::Comment;
    }
    let mut terminal = Terminal::new(TestBackend::new(64, 20)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();

    let (x, y) = achar(buffer, "Comentário");
    let (bx, _) = achar(buffer, "Identificação");
    // A caixa começa depois do rótulo, na mesma coluna nas duas linhas, e vai
    // até a coluna vizinha da borda direita.
    let inicio = bx + 15 + 2;
    assert_eq!(
        buffer.cell((x, y)).unwrap().bg,
        crate::theme::BASE,
        "rótulo"
    );
    for linha in [y, y + 1, y + 2] {
        assert_eq!(
            buffer.cell((inicio, linha)).unwrap().bg,
            crate::theme::SURFACE0,
            "começo da caixa"
        );
        assert_eq!(
            buffer.cell((bx + 56, linha)).unwrap().bg,
            crate::theme::SURFACE0,
            "fundo até a borda"
        );
        assert_eq!(
            buffer.cell((bx + 57, linha)).unwrap().symbol(),
            "│",
            "borda direita do pop-up"
        );
    }
    // As linhas extra têm o mesmo tamanho: nada de caixa antes da coluna do
    // valor.
    for linha in [y + 1, y + 2] {
        assert_eq!(buffer.cell((bx, linha)).unwrap().bg, crate::theme::BASE);
    }
}

#[test]
fn popup_da_tabela_sem_auto_increment_esconde_o_campo() {
    let mut app = app_com_popup_de_tabela();
    if let Some(edicao) = app.table_edit.as_mut() {
        edicao.original.auto_increment = None;
    }
    let terminal = terminal_de(&mut app, 64, 20);
    let tela = texto(terminal.backend().buffer());
    assert!(!tela.contains("Auto-increment"), "{tela}");
    assert!(tela.contains("Padrões de texto"), "{tela}");
}

#[test]
fn popup_da_tabela_destaca_o_campo_em_edicao() {
    let mut app = app_com_popup_de_tabela();
    let mut terminal = Terminal::new(TestBackend::new(64, 20)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();

    // O nome tem o foco: fundo do campo e cursor no fim.
    let (x, y) = achar(buffer, "users");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::SURFACE0);
    let linha: String = (x..x + 10)
        .map(|coluna| buffer.cell((coluna, y)).unwrap().symbol().to_string())
        .collect();
    assert!(linha.contains('▏'), "{linha}");

    // O comentário, sem foco, não tem fundo próprio.
    let (x, y) = achar(buffer, "cadastro");
    assert_eq!(buffer.cell((x, y)).unwrap().bg, crate::theme::BASE);
}

#[test]
fn popup_da_tabela_mostra_as_teclas_da_lista_no_campo_de_lista() {
    let mut app = app_com_popup_de_tabela();
    if let Some(edicao) = app.table_edit.as_mut() {
        edicao.field = crate::config::TableField::Engine;
    }
    let terminal = terminal_de(&mut app, 64, 20);
    let tela = texto(terminal.backend().buffer());
    assert!(
        tela.contains("enter lista · s salvar · esc cancelar"),
        "{tela}"
    );
    assert!(!tela.contains("enter salvar · esc cancelar"), "{tela}");
}

#[test]
fn lista_da_engine_mostra_as_opcoes_do_servidor() {
    let mut app = app_com_popup_de_tabela();
    if let Some(edicao) = app.table_edit.as_mut() {
        edicao.field = crate::config::TableField::Engine;
        edicao.activate();
    }
    let terminal = terminal_de(&mut app, 64, 20);
    let tela = texto(terminal.backend().buffer());

    assert!(tela.contains("\u{f0d7} Engine"), "{tela}");
    assert!(tela.contains("InnoDB"), "{tela}");
    assert!(tela.contains("MyISAM"), "{tela}");
    assert!(tela.contains("enter escolher · esc voltar"), "{tela}");
    assert!(!tela.contains("Padrões de texto"), "{tela}");
}

#[test]
fn comentario_longo_usa_tres_linhas_da_caixa() {
    let mut app = app_com_popup_de_tabela();
    if let Some(edicao) = app.table_edit.as_mut() {
        // Uma palavra só não tem onde descer: ela é partida em 40 + 40.
        edicao.values.comment = "a".repeat(80);
    }
    let mut terminal = Terminal::new(TestBackend::new(64, 20)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();
    let (_, y) = achar(buffer, "Comentário");

    assert_eq!(linha_do_buffer(buffer, y).matches('a').count(), 40);
    assert_eq!(linha_do_buffer(buffer, y + 1).matches('a').count(), 40);
    assert_eq!(linha_do_buffer(buffer, y + 2).matches('a').count(), 0);
}

#[test]
fn comentario_nao_corta_palavras_na_quebra() {
    let comentario = "um comentario com palavras que vao sendo empurradas para a linha de baixo";
    let mut app = app_com_popup_de_tabela();
    if let Some(edicao) = app.table_edit.as_mut() {
        edicao.values.comment = comentario.to_owned();
    }
    let mut terminal = Terminal::new(TestBackend::new(64, 20)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();
    let (_, y) = achar(buffer, "Comentário");

    // Juntando as linhas da caixa, o texto é exatamente o comentário: nenhuma
    // palavra foi cortada no meio.
    let linhas = comentario_da_caixa(buffer, y);
    assert_eq!(linhas.join(" ").trim_end(), comentario);
    // "sendo" é a última palavra que cabe na primeira linha (40 colunas).
    assert_eq!(linhas[0].len(), 40, "{}", linhas[0]);
    assert!(linhas[0].ends_with("sendo"), "{}", linhas[0]);
    assert!(linhas[1].starts_with("empurradas"), "{}", linhas[1]);
}

#[test]
fn comentario_maior_que_a_caixa_mostra_as_ultimas_linhas() {
    let palavras: Vec<String> = (1..=12)
        .map(|indice| format!("palavra{indice:03}"))
        .collect();
    let comentario = palavras.join(" ");

    let mut app = app_com_popup_de_tabela();
    if let Some(edicao) = app.table_edit.as_mut() {
        edicao.values.comment = comentario;
    }
    let mut terminal = Terminal::new(TestBackend::new(64, 20)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();
    let (_, y) = achar(buffer, "Comentário");

    // Quatro linhas de três palavras: a caixa mostra as três últimas, onde
    // está o cursor, sem repetir nada.
    let linhas = comentario_da_caixa(buffer, y);
    assert!(linhas[0].starts_with("palavra004"), "{}", linhas[0]);
    assert!(linhas[2].ends_with("palavra012"), "{}", linhas[2]);
    assert!(!linhas.iter().any(|linha| linha.contains("palavra003")));
}
