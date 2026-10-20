use super::*;

#[test]
fn mostra_conexao_tabelas_e_celulas() {
    let screen = screen(&mut app_com_dados());
    assert!(screen.contains("MySQL 8.0.35"), "{screen}");
    assert!(screen.contains("users"), "{screen}");
    assert!(screen.contains("ana"), "{screen}");
    assert!(screen.contains("NULL"), "{screen}");
    assert!(screen.contains("2 linhas"), "{screen}");
}

#[test]
fn sem_dados_mostra_dica() {
    let screen = screen(&mut App::new(Some("mysql://user@localhost/app")));
    assert!(screen.contains("selecione uma tabela"), "{screen}");
}

#[test]
fn popup_de_saida_aparece_sem_edicao() {
    let mut app = app_com_dados();
    app.mode = crate::app::Mode::ConfirmQuit;
    let tela = screen(&mut app);
    assert!(tela.contains("\u{f011} Sair"), "{tela}");
    assert!(tela.contains("Deseja fechar o dbterm?"), "{tela}");
    assert!(tela.contains("enter sair · esc cancelar"), "{tela}");
}

#[test]
fn colunas_tem_traco_divisorio() {
    let screen = screen(&mut app_com_dados());
    assert!(screen.contains('│'), "{screen}");
    assert!(screen.contains("id"), "{screen}");
    assert!(screen.contains("name"), "{screen}");
}

#[test]
fn cabecalho_nao_e_espremido_em_grade_estreita() {
    // Sem o cálculo de largura, o `Table` encolhe tudo e o cabeçalho vira
    // "c", "l", "n" — todas as colunas precisam aparecer por inteiro.
    let screen = screen_at(&mut app_largo(), 120);
    for nome in [
        "id",
        "name",
        "email",
        "active",
        "created_at",
        "last_login",
        "notes",
        "meta",
    ] {
        assert!(screen.contains(nome), "faltou {nome}: {screen}");
    }
}

#[test]
fn valor_que_nao_cabe_termina_em_reticencias() {
    let mut app = app_com_dados();
    app.data = Some(TableData {
        columns: vec![String::from("id"), String::from("name")],
        kinds: vec![ColumnKind::Number, ColumnKind::Text],
        rows: vec![vec![
            String::from("1"),
            String::from("um valor bem comprido que nao cabe na coluna"),
        ]],
        primary_key: vec![0],
    });
    let screen = screen(&mut app);
    assert!(screen.contains('…'), "{screen}");
    assert!(!screen.contains("cabe na coluna"), "{screen}");
}

#[test]
fn erro_aparece_no_rodape() {
    let mut app = app_com_dados();
    app.status = Status::Failed(String::from("sem permissão"));
    let screen = screen(&mut app);
    assert!(screen.contains("erro: sem permissão"), "{screen}");
}

#[test]
fn confirmacao_dupla_aparece_na_tela() {
    let mut app = app_com_dados();
    app.mode = crate::app::Mode::Confirm;
    app.edit = Some(crate::app::Edit {
        row: 0,
        column: 1,
        original: String::from("ana"),
        input: String::from("bia"),
    });
    let tela = screen(&mut app);
    assert!(tela.contains("\u{f04b} Execução"), "{tela}");
    assert!(tela.contains("TABLE users COLUMN name"), "{tela}");
    assert!(tela.contains("UPDATE users"), "{tela}");
    assert!(tela.contains("SET name = 'bia'"), "{tela}");
    assert!(tela.contains("WHERE id = 1;"), "{tela}");
    assert!(!tela.contains("Deseja realmente alterar"), "{tela}");
    assert!(!tela.contains("De: ana"), "{tela}");
    assert!(tela.contains("enter executar · esc cancelar"), "{tela}");

    app.mode = crate::app::Mode::ConfirmAgain;
    let terminal = terminal_de(&mut app, 64, 14);
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);
    assert!(tela.contains("\u{f00c} Confirmar"), "{tela}");
    assert!(tela.contains("TABLE users COLUMN name"), "{tela}");
    assert!(tela.contains("SET name = 'bia'"), "{tela}");
    assert!(tela.contains("WHERE id = 1;"), "{tela}");
    assert!(
        tela.contains("1 registro será alterado na execução do UPDATE."),
        "{tela}"
    );
    assert!(tela.contains("ESSA AÇÃO É IRREVERSÍVEL."), "{tela}");
    assert!(tela.contains("Você confirma?"), "{tela}");
    assert!(!tela.contains("Tem certeza?"), "{tela}");

    // O aviso é texto padrão, menos `UPDATE` (cor da query); a advertência
    // é vermelha; e a pergunta vem depois de uma linha em branco.
    let (xa, ya) = achar(buffer, "1 registro será alterado");
    assert_eq!(buffer.cell((xa, ya)).unwrap().fg, crate::theme::TEXT, "1");
    assert_eq!(
        buffer.cell((xa + 40, ya)).unwrap().fg,
        crate::theme::MAUVE,
        "UPDATE"
    );
    let (xd, yd) = achar(buffer, "ESSA AÇÃO É IRREVERSÍVEL.");
    assert_eq!(buffer.cell((xd, yd)).unwrap().fg, crate::theme::RED, "ESSA");
    let (xp, yp) = achar(buffer, "Você confirma?");
    assert_eq!(
        buffer.cell((xp, yp - 1)).unwrap().symbol(),
        " ",
        "linha vazia"
    );
    let (_, yc) = achar(buffer, "enter confirmo · esc cancelar");
    assert_eq!(yc, yp + 2, "ação logo abaixo da régua");
    assert_eq!(
        buffer.cell((xp, yp + 1)).unwrap().symbol(),
        "─",
        "régua abaixo da pergunta"
    );
}

#[test]
fn query_do_popup_de_execucao_tem_cor_do_titulo_e_reguas() {
    let mut app = app_com_dados();
    app.mode = crate::app::Mode::Confirm;
    app.edit = Some(crate::app::Edit {
        row: 0,
        column: 1,
        original: String::from("ana"),
        input: String::from("bia"),
    });
    let terminal = terminal_de(&mut app, 64, 12);
    let buffer = terminal.backend().buffer();
    let (x, y) = achar(buffer, "UPDATE users");

    // Declaração na cor do título, identificador na padrão e literal pelo
    // tipo: `name` é texto (verde) e a chave `id` é número (pêssego).
    assert_eq!(
        buffer.cell((x, y)).unwrap().fg,
        crate::theme::MAUVE,
        "UPDATE"
    );
    assert_eq!(
        buffer.cell((x + 7, y)).unwrap().fg,
        crate::theme::TEXT,
        "users"
    );
    assert_eq!(
        buffer.cell((x, y + 1)).unwrap().fg,
        crate::theme::MAUVE,
        "SET"
    );
    assert_eq!(
        buffer.cell((x + 12, y + 1)).unwrap().fg,
        crate::theme::GREEN,
        "'bia'"
    );
    assert_eq!(
        buffer.cell((x, y + 2)).unwrap().fg,
        crate::theme::MAUVE,
        "WHERE"
    );
    assert_eq!(
        buffer.cell((x + 11, y + 2)).unwrap().fg,
        crate::theme::PEACH,
        "1"
    );

    // Régua entre a identificação e a query, e entre a query e a ação.
    assert_eq!(buffer.cell((x, y - 1)).unwrap().symbol(), "─");
    assert_eq!(buffer.cell((x, y + 3)).unwrap().symbol(), "─");
}

#[test]
fn popup_de_edicao_usa_titulo_identificadores_e_respiro() {
    let mut app = app_com_dados();
    app.mode = crate::app::Mode::Edit;
    app.edit = Some(crate::app::Edit {
        row: 0,
        column: 1,
        original: String::from("ana"),
        input: String::from("bia"),
    });
    let tela = screen(&mut app);
    assert!(tela.contains("\u{f040} Editar"), "{tela}");
    assert!(tela.contains("TABLE users COLUMN name"), "{tela}");
    // Uma coluna de respiro entre a borda esquerda e cada texto.
    assert!(tela.contains("│ TABLE"), "{tela}");
    assert!(!tela.contains("│TABLE"), "{tela}");
    assert!(tela.contains("│ bia"), "{tela}");

    // `enter`/`esc` na cor do título; as ações, na cor padrão.
    let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let buffer = terminal.backend().buffer();
    let largura = buffer.area.width as usize;
    let celulas: Vec<_> = buffer.content().iter().collect();
    let inicio = celulas
        .windows("enter continuar".len())
        .position(|janela| {
            janela.iter().map(|cell| cell.symbol()).collect::<String>() == "enter continuar"
        })
        .expect("linha de ação");
    let (x, y) = ((inicio % largura) as u16, (inicio / largura) as u16);
    assert_eq!(
        buffer.cell((x, y)).unwrap().fg,
        crate::theme::MAUVE,
        "enter"
    );
    assert_eq!(
        buffer.cell((x + 6, y)).unwrap().fg,
        crate::theme::TEXT,
        "continuar"
    );

    // Réguas acima e abaixo do valor, e fundo próprio do campo.
    let valor = celulas
        .windows("bia".len())
        .position(|janela| janela.iter().map(|cell| cell.symbol()).collect::<String>() == "bia")
        .expect("valor em edição");
    let (xv, yv) = ((valor % largura) as u16, (valor / largura) as u16);
    assert_eq!(buffer.cell((xv, yv - 1)).unwrap().symbol(), "─");
    assert_eq!(buffer.cell((xv, yv + 1)).unwrap().symbol(), "─");
    assert_eq!(
        buffer.cell((xv + 40, yv)).unwrap().bg,
        crate::theme::SURFACE0,
        "fundo do campo"
    );
}

#[test]
fn selecao_preserva_a_cor_da_divisoria() {
    let mut app = app_com_dados();
    app.table_state.select(Some(0));
    app.column = Some(0);
    let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    // A divisória da linha selecionada tem o fundo da seleção, mas mantém o
    // cinza de `muted` em vez de herdar o `fg` do destaque.
    let divisoria = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|cell| cell.symbol() == RULE && cell.bg == crate::theme::SURFACE0)
        .expect("esperava uma divisória na linha selecionada");
    assert_eq!(divisoria.fg, crate::theme::OVERLAY0);
}

#[test]
fn nome_da_coluna_comeca_colado_na_divisoria() {
    let screen = screen(&mut app_com_dados());
    assert!(screen.contains("│name"), "{screen}");
    assert!(!screen.contains("│ name"), "{screen}");
}

#[test]
fn destaque_da_coluna_cobre_a_largura_inteira() {
    let mut app = app_com_dados();
    app.table_state.select(Some(0));
    app.column = Some(0);
    app.mode = crate::app::Mode::Columns;
    let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    let destaque = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .filter(|cell| cell.bg == crate::theme::SURFACE1)
        .count();
    // `id` tem largura 2; somando os 2 espaços até a divisória, dá 4.
    assert_eq!(destaque, 4);
}

#[test]
fn aplica_paleta_catppuccin_macchiato() {
    let mut app = app_com_dados();
    let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
    terminal
        .draw(|frame| draw(frame, &mut app))
        .expect("desenho");
    // O primeiro caractere é o espaço de " dbterm ": fundo `base` e accent `mauve`.
    let inicio = &terminal.backend().buffer().content()[0];
    assert_eq!(inicio.bg, crate::theme::BASE);
    assert_eq!(inicio.fg, crate::theme::MAUVE);
}
