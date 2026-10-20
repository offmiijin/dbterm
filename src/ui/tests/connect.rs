use super::*;

#[test]
fn tela_de_conexao_mostra_a_marca_e_o_formulario() {
    let mut app = app_form();
    let terminal = terminal_de(&mut app, 80, 24);
    let screen = texto(terminal.backend().buffer());
    assert!(screen.contains("██████╗"), "{screen}");
    assert!(screen.contains("127.0.0.1"), "{screen}");
    assert!(screen.contains("host"), "{screen}");
    assert!(screen.contains("usuário"), "{screen}");
    assert!(screen.contains("schema"), "{screen}");
    assert!(screen.contains("enter conectar"), "{screen}");
}

#[test]
fn senha_nao_aparece_na_tela_de_conexao() {
    let mut app = app_form();
    app.connection
        .value_mut(Field::Password)
        .push_str("segredo");
    let terminal = terminal_de(&mut app, 80, 24);
    let screen = texto(terminal.backend().buffer());
    assert!(!screen.contains("segredo"), "{screen}");
    assert!(screen.contains("•••••••"), "{screen}");
}

#[test]
fn terminal_estreito_troca_a_marca_pelo_nome() {
    let mut app = app_form();
    let terminal = terminal_de(&mut app, 30, 24);
    let screen = texto(terminal.backend().buffer());
    assert!(screen.contains("dbterm"), "{screen}");
    assert!(!screen.contains("██████"), "{screen}");
}

#[test]
fn tela_inicial_mostra_as_conexoes_rapidas() {
    let mut app = app_com_rapidas(&["db.local", "outro"]);
    let terminal = terminal_de(&mut app, 80, 30);
    let screen = texto(terminal.backend().buffer());
    assert!(screen.contains("Conexões Rápidas"), "{screen}");
    assert!(screen.contains("MySQL · db.local:3306"), "{screen}");
    assert!(screen.contains("root @ app"), "{screen}");
    assert!(screen.contains("outro:3306"), "{screen}");
    assert!(screen.contains("enter abrir"), "{screen}");
}

#[test]
fn tela_inicial_mostra_no_maximo_tres_conexoes() {
    let mut app = app_com_rapidas(&["a", "b", "c", "d"]);
    let terminal = terminal_de(&mut app, 80, 30);
    let screen = texto(terminal.backend().buffer());
    assert!(screen.contains("a:3306"), "{screen}");
    assert!(screen.contains("b:3306"), "{screen}");
    assert!(screen.contains("c:3306"), "{screen}");
    assert!(!screen.contains("d:3306"), "{screen}");
}

#[test]
fn tela_inicial_vazia_pede_nova_conexao() {
    let mut app = app_quick();
    let terminal = terminal_de(&mut app, 80, 24);
    let screen = texto(terminal.backend().buffer());
    assert!(screen.contains("nenhuma conexão salva"), "{screen}");
    assert!(screen.contains("n nova conexão"), "{screen}");
}

#[test]
fn chips_mostram_os_niveis_na_ordem_com_o_ativo_expandido() {
    let mut app = app_com_dados();
    app.databases = vec![String::from("app")];
    app.focus = Focus::Databases;
    let tela = screen(&mut app);
    let conexoes = tela.find("[c]").expect("chip de conexões");
    let bancos = tela.find("bancos (1)").expect("bancos ativo");
    let tabelas = tela.find("[t]").expect("chip de tabelas");
    assert!(conexoes < bancos && bancos < tabelas, "{tela}");
    assert!(!tela.contains("[b]"), "o nível ativo não é um chip: {tela}");
}

#[test]
fn com_as_tabelas_ativas_os_bancos_viram_chip() {
    let mut app = app_com_dados();
    app.databases = vec![String::from("app")];
    app.focus = Focus::Tables;
    let tela = screen(&mut app);
    assert!(tela.contains("tabelas (1)"), "{tela}");
    assert!(tela.contains("[c]"), "{tela}");
    assert!(tela.contains("[b]"), "{tela}");
}

#[test]
fn chips_ficam_centralizados_sobre_o_painel() {
    let mut app = app_com_dados();
    app.databases = vec![String::from("app")];
    app.focus = Focus::Databases;
    let terminal = terminal_de(&mut app, 80, 12);
    let buffer = terminal.backend().buffer();

    // "[c] bancos (1) [t]" tem 18 colunas dentro das 28 do painel.
    let (x, _) = achar(buffer, "[c]");
    assert_eq!(x, (28 - 18) / 2);
}

#[test]
fn nomes_dos_niveis_usam_o_roxo_da_marca() {
    let mut app = app_com_dados();
    app.databases = vec![String::from("app")];
    app.focus = Focus::Databases;
    let terminal = terminal_de(&mut app, 80, 12);
    let buffer = terminal.backend().buffer();

    let (x, y) = achar(buffer, "bancos (1)");
    assert_eq!(buffer.cell((x, y)).unwrap().fg, crate::theme::MAUVE);

    // As letras dos chips recolhidos têm a mesma cor do nome ativo.
    let (x, y) = achar(buffer, "[c]");
    assert_eq!(buffer.cell((x + 1, y)).unwrap().fg, crate::theme::MAUVE);
}

#[test]
fn nivel_de_conexoes_lista_os_enderecos_salvos() {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.databases = vec![String::from("app")];
    app.tables = vec![String::from("users")];
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    let tela = screen(&mut app);
    assert!(tela.contains("db.local:3306"), "{tela}");
    assert!(tela.contains("conexões (1)"), "{tela}");
}

#[test]
fn rodape_mostra_a_tecla_de_edicao_so_nos_niveis_que_editam() {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.databases = vec![String::from("app")];
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));

    // O `e` não é chip: aparece no rodapé enquanto as conexões estão ativas.
    let tela = screen_at(&mut app, 100);
    assert!(tela.contains("conexões (1)"), "{tela}");
    assert!(!tela.contains("[e]"), "{tela}");
    assert!(tela.contains("e editar"), "{tela}");

    // Nos bancos, a mesma dica: `e` edita o banco selecionado.
    app.focus = Focus::Databases;
    app.databases_state.select(Some(0));
    let tela = screen_at(&mut app, 100);
    assert!(tela.contains("bancos (1)"), "{tela}");
    assert!(tela.contains("e editar"), "{tela}");

    // Nas tabelas também: `e` edita a tabela selecionada.
    app.focus = Focus::Tables;
    let tela = screen_at(&mut app, 100);
    assert!(tela.contains("e editar"), "{tela}");

    // Nos registros não: a grade edita célula a célula.
    app.focus = Focus::Data;
    let tela = screen_at(&mut app, 100);
    assert!(!tela.contains("e editar"), "{tela}");

    // Com o pop-up aberto, o rodapé é o do pop-up.
    app.focus = Focus::Connections;
    app.connection_edit = Some(ConnectionEdit {
        saved: 0,
        connection: Connection::default(),
        field: Field::Host,
    });
    let tela = screen_at(&mut app, 100);
    assert!(!tela.contains("e editar"), "{tela}");
}
