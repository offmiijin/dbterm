use super::*;

#[test]
fn muitas_colunas_mostram_o_nome_do_cabecalho_inteiro() {
    // 14 colunas não cabem em 100 colunas de terminal: sem a rolagem
    // horizontal, o `Table` espreme o cabeçalho e sobra só a inicial.
    let screen = screen_at(&mut app_com_muitas_colunas(), 100);
    for nome in ["id", "name", "email", "active"] {
        assert!(screen.contains(nome), "faltou {nome}: {screen}");
    }
    // As colunas que não couberam ficam para a rolagem, e o título avisa.
    assert!(!screen.contains("coluna_0"), "{screen}");
    assert!(screen.contains("colunas 1-4/14"), "{screen}");
}

#[test]
fn rolagem_horizontal_alcanca_as_ultimas_colunas() {
    let mut app = app_com_muitas_colunas();
    app.hscroll = 13;
    let screen = screen_at(&mut app, 100);
    assert!(screen.contains("coluna_5"), "{screen}");
    assert!(screen.contains("colunas 14-14/14"), "{screen}");
    assert!(!screen.contains("created_at"), "{screen}");
}

#[test]
fn coluna_selecionada_fora_da_tela_puxa_a_rolagem() {
    let mut app = app_com_muitas_colunas();
    app.focus = crate::app::Focus::Data;
    app.mode = crate::app::Mode::Columns;
    app.column = Some(13);
    let screen = screen_at(&mut app, 100);
    assert!(screen.contains("coluna_5"), "{screen}");
    assert!(screen.contains("col: coluna_5"), "{screen}");
}

#[test]
fn rodape_mostra_o_que_foi_carregado_e_o_fim_da_tabela() {
    let mut app = app_com_dados();
    let tela = screen_at(&mut app, 100);
    assert!(tela.contains("2 linhas · ↓ carrega mais"), "{tela}");

    // Página curta: a tabela acabou e o rodapé para de convidar a descer.
    app.end_reached = true;
    let tela = screen_at(&mut app, 100);
    assert!(!tela.contains("carrega mais"), "{tela}");
}

#[test]
fn janela_vertical_segue_a_selecao_e_a_rolagem() {
    // Seleção dentro da rolagem guardada: a janela não anda.
    assert_eq!(linhas_visiveis(0, Some(3), 5, 100), (0, 5));
    assert_eq!(linhas_visiveis(10, Some(12), 5, 100), (10, 15));
    // Seleção abaixo da janela: ela vira a última linha visível.
    assert_eq!(linhas_visiveis(0, Some(7), 5, 100), (3, 8));
    // Seleção acima da rolagem: ela vira a primeira linha visível.
    assert_eq!(linhas_visiveis(20, Some(2), 5, 100), (2, 7));
    // Fim da tabela: a janela encolhe em vez de passar do fim.
    assert_eq!(linhas_visiveis(95, Some(99), 5, 100), (95, 100));
    // Rolagem além do fim e casos vazios.
    assert_eq!(linhas_visiveis(50, None, 5, 10), (9, 10));
    assert_eq!(linhas_visiveis(0, None, 5, 0), (0, 0));
    assert_eq!(linhas_visiveis(0, Some(0), 0, 10), (0, 0));
}

#[test]
fn grade_desenha_so_as_linhas_da_janela() {
    let mut app = app_com_dados();
    app.data = Some(TableData {
        columns: vec![String::from("id")],
        kinds: vec![ColumnKind::Text],
        rows: (1..=100)
            .map(|n| vec![format!("registro-{n:03}")])
            .collect(),
        primary_key: vec![0],
    });
    app.larguras = crate::view::larguras(app.data.as_ref().expect("grade"));
    app.table_state.select(Some(49));

    let terminal = terminal_de(&mut app, 100, 12);
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);
    // A linha selecionada aparece, e as de fora da janela não são desenhadas.
    assert!(tela.contains("registro-050"), "{tela}");
    assert!(!tela.contains("registro-001"), "{tela}");
    // O destaque fica na linha da seleção, não na primeira da janela.
    let (_, y) = achar(buffer, "registro-050");
    let largura = buffer.area.width as usize;
    let linha: String = buffer.content()[y as usize * largura..][..largura]
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(linha.contains('❯'), "{linha}");
}
