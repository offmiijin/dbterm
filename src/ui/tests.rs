use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use super::draw;
use super::grid::{RULE, linhas_visiveis};
use crate::app::{App, ConnectionEdit, Focus, Screen, Status};
use crate::config::{Connection, Field};
use crate::db::{ColumnKind, TableData};
use crate::history::{History, Saved};

/// App na tela inicial, sem histórico em disco.
fn app_quick() -> App {
    let mut app = App::new(None);
    app.history = History::in_memory();
    app.quick = None;
    app
}

/// App com conexões rápidas salvas, na ordem dada (a primeira é a mais recente).
fn app_com_rapidas(hosts: &[&str]) -> App {
    let mut app = app_quick();
    let total = hosts.len();
    let entries = hosts
        .iter()
        .enumerate()
        .map(|(i, host)| Saved {
            engine: String::from(Saved::MYSQL),
            host: String::from(*host),
            port: 3306,
            user: String::from("root"),
            database: String::from("app"),
            password: None,
            last_used: (total - i) as u64,
        })
        .collect();
    app.history = History::with(entries);
    app.quick = Some(0);
    app
}

/// App com o formulário de conexão aberto.
fn app_form() -> App {
    let mut app = app_quick();
    app.screen = Screen::Connect;
    app.connection = Connection::default();
    app.field = Field::Host;
    app.connect_error = None;
    app
}

fn screen(app: &mut App) -> String {
    screen_at(app, 64)
}

fn screen_at(app: &mut App, largura: u16) -> String {
    texto(terminal_de(app, largura, 12).backend().buffer())
}

/// Terminal de teste já desenhado, para conferir texto e cores.
fn terminal_de(app: &mut App, largura: u16, altura: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(largura, altura)).expect("terminal de teste");
    terminal.draw(|frame| draw(frame, app)).expect("desenho");
    terminal
}

/// O buffer inteiro como texto, sem quebras de linha.
fn texto(buffer: &Buffer) -> String {
    buffer.content().iter().map(|cell| cell.symbol()).collect()
}

/// `(x, y)` da primeira célula do `texto` desenhado.
fn achar(buffer: &Buffer, texto: &str) -> (u16, u16) {
    let largura = buffer.area.width as usize;
    let simbolos: Vec<&str> = buffer.content().iter().map(|cell| cell.symbol()).collect();
    for linha in simbolos.chunks(largura) {
        let com_valor: Vec<_> = linha.iter().filter(|s| s.contains("reg")).collect();
        if !com_valor.is_empty() {
            println!("{com_valor:?}");
        }
    }
    let alvo: Vec<String> = texto
        .chars()
        .map(|caractere| caractere.to_string())
        .collect();
    let posicao = simbolos
        .windows(alvo.len())
        .position(|janela| {
            janela
                .iter()
                .zip(&alvo)
                .all(|(celula, alvo)| *celula == alvo.as_str())
        })
        .unwrap_or_else(|| panic!("não achei {texto:?} na tela"));
    ((posicao % largura) as u16, (posicao / largura) as u16)
}

fn app_com_dados() -> App {
    let mut app = App::new(Some("mysql://user@localhost/app"));
    app.server = String::from("MySQL 8.0.35");
    app.database = String::from("app");
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.current = Some(String::from("users"));
    app.data = Some(TableData {
        columns: vec![String::from("id"), String::from("name")],
        kinds: vec![ColumnKind::Number, ColumnKind::Text],
        rows: vec![
            vec![String::from("1"), String::from("ana")],
            vec![String::from("2"), String::from("NULL")],
        ],
        primary_key: vec![0],
    });
    app.status = Status::Ready;
    app
}

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

fn app_largo() -> App {
    let mut app = App::new(Some("mysql://user@localhost/app"));
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.current = Some(String::from("users"));
    app.data = Some(TableData {
        columns: [
            "id",
            "name",
            "email",
            "active",
            "created_at",
            "last_login",
            "notes",
            "meta",
        ]
        .iter()
        .map(|nome| nome.to_string())
        .collect(),
        kinds: [
            ColumnKind::Number,
            ColumnKind::Text,
            ColumnKind::Text,
            ColumnKind::Number,
            ColumnKind::Text,
            ColumnKind::Text,
            ColumnKind::Text,
            ColumnKind::Text,
        ]
        .to_vec(),
        rows: vec![vec![
            String::from("1"),
            String::from("Ana Prado"),
            String::from("ana@exemplo.com"),
            String::from("1"),
            String::from("2024-01-05 09:12:00"),
            String::from("2026-10-17 08:00:00"),
            String::from("cliente desde o lançamento"),
            String::from("{\"plano\": \"pro\"}"),
        ]],
        primary_key: vec![0],
    });
    app.status = Status::Ready;
    app
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
fn rodape_mostra_a_tecla_de_edicao_so_nas_conexoes() {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));

    // O `e` não é chip: aparece no rodapé enquanto as conexões estão ativas.
    let tela = screen_at(&mut app, 100);
    assert!(tela.contains("conexões (1)"), "{tela}");
    assert!(!tela.contains("[e]"), "{tela}");
    assert!(tela.contains("e editar"), "{tela}");

    // Fora das conexões a tecla some.
    app.focus = Focus::Databases;
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

/// App com o pop-up "Editar" da conexão aberto sobre a tela de trabalho.
fn app_com_popup_de_conexao() -> App {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    let mut connection = Connection::default();
    *connection.value_mut(Field::Host) = String::from("db.local");
    *connection.value_mut(Field::Database) = String::from("app");
    *connection.value_mut(Field::Password) = String::from("segredo");
    app.connection_edit = Some(ConnectionEdit {
        saved: 0,
        connection,
        field: Field::Host,
    });
    app
}

#[test]
fn popup_de_conexao_mostra_servidor_e_autenticacao_agrupados() {
    let mut app = app_com_popup_de_conexao();
    let terminal = terminal_de(&mut app, 64, 12);
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);

    assert!(tela.contains("\u{f044} Editar"), "{tela}");
    assert!(tela.contains("Servidor"), "{tela}");
    assert!(tela.contains("Autenticação"), "{tela}");
    assert!(
        tela.contains("tab campo · enter salvar · esc cancelar"),
        "{tela}"
    );

    // Host e porta dividem a linha, e o banco vem logo abaixo.
    let (xhost, yhost) = achar(buffer, "Host");
    let (xporta, yporta) = achar(buffer, "Porta");
    let (_, ybanco) = achar(buffer, "Banco");
    assert_eq!(yhost, yporta, "host e porta lado a lado");
    assert!(xhost < xporta, "host à esquerda da porta");
    assert_eq!(ybanco, yhost + 1, "banco abaixo de host e porta");

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

/// Tabela com 14 colunas, mais do que cabe num terminal estreito.
fn app_com_muitas_colunas() -> App {
    let mut app = app_largo();
    let data = app.data.as_mut().expect("grade");
    for index in 0..6 {
        data.columns.push(format!("coluna_{index}"));
        data.kinds.push(ColumnKind::Text);
    }
    for row in data.rows.iter_mut() {
        for index in 0..6 {
            row.push(format!("valor {index}"));
        }
    }
    app
}

#[test]
fn muitas_colunas_mostram_o_nome_do_cabecalho_inteiro() {
    // 14 colunas não cabem em 100 colunas de terminal: sem a rolagem
    // horizontal, o `Table` espreme o cabeçalho e sobra só a inicial.
    let screen = screen_at(&mut app_com_muitas_colunas(), 100);
    for nome in ["id", "name", "email", "active", "created_at"] {
        assert!(screen.contains(nome), "faltou {nome}: {screen}");
    }
    // As colunas que não couberam ficam para a rolagem, e o título avisa.
    assert!(!screen.contains("coluna_0"), "{screen}");
    assert!(screen.contains("colunas 1-5/14"), "{screen}");
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
    app.larguras = super::larguras(app.data.as_ref().expect("grade"));
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

#[test]
fn popup_de_conexao_corta_o_valor_longo_sem_quebrar_o_layout() {
    let mut app = app_com_popup_de_conexao();
    if let Some(edit) = app.connection_edit.as_mut() {
        *edit.connection.value_mut(Field::Host) =
            String::from("db.producao.interno.empresa.com.br.exemplo");
    }
    let tela = screen(&mut app);
    // O valor grande não empurra as linhas: o corte fica no começo, onde o
    // cursor não está, e o pop-up mantém as ações no lugar.
    assert!(tela.contains('…'), "{tela}");
    assert!(
        tela.contains("tab campo · enter salvar · esc cancelar"),
        "{tela}"
    );
    assert!(tela.contains("Senha"), "{tela}");
}
