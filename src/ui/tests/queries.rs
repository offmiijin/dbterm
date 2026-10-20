use super::*;
use crate::app::ResultadoQuery;

/// App com o editor aberto e o painel de resultado preenchido com 1 linha.
fn app_com_resultado() -> App {
    let mut app = app_com_dados();
    app.new_query();
    if let Some(query) = app.queries.get_mut(0) {
        query.sql = String::from("SELECT * FROM users");
        query.fim();
    }
    app.query_result = Some(ResultadoQuery {
        sql: String::from("SELECT * FROM users"),
        tabela: Some(String::from("users")),
        data: Some(TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("1")]],
            primary_key: Vec::new(),
        }),
        larguras: vec![2],
        end_reached: true,
        ..ResultadoQuery::default()
    });
    app.query_result
        .as_mut()
        .expect("painel")
        .state
        .select(Some(0));
    app
}

#[test]
fn lateral_direita_mostra_a_aba_queries_com_as_instancias() {
    let mut app = app_com_dados();
    app.new_query();
    app.focus = Focus::Queries;
    let tela = screen_at(&mut app, 130);
    assert!(tela.contains("queries"), "{tela}");
    assert!(tela.contains("(1)"), "{tela}");
    assert!(tela.contains("q1"), "{tela}");
    assert!(tela.contains("vazio"), "{tela}");
}

#[test]
fn chip_mostra_a_contagem_de_instancias() {
    let mut app = app_com_dados();
    app.new_query();
    app.new_query();
    app.focus = Focus::Queries;
    let tela = screen_at(&mut app, 130);
    assert!(tela.contains("(2)"), "{tela}");
    assert!(!tela.contains("[n]"), "{tela}");
}

#[test]
fn sem_instancias_o_painel_convida_a_criar_no_meio() {
    let mut app = app_com_dados();
    assert!(app.queries.is_empty());
    app.focus = Focus::Queries;
    let terminal = terminal_de(&mut app, 130, 12);
    let buffer = terminal.backend().buffer();
    let tela = texto(buffer);
    assert!(tela.contains("queries"), "{tela}");
    assert!(tela.contains("(0)"), "{tela}");
    assert!(tela.contains("[n] nova query"), "{tela}");
    // No meio vertical do painel, não no topo.
    let (_, y) = achar(buffer, "nova query");
    assert!(y > 4 && y < 10, "no meio do painel: y={y} {tela}");
}

#[test]
fn editor_vazio_mostra_dica_e_titulo_com_nome_e_schema() {
    let mut app = app_com_dados();
    app.new_query();
    let tela = screen_at(&mut app, 130);
    assert!(tela.contains("query · q1 · schema: app · vazio"), "{tela}");
    assert!(tela.contains("digite o SQL"), "{tela}");
    // A grade sai do centro quando o editor abre.
    assert!(!tela.contains("dados ·"), "{tela}");
}

#[test]
fn editor_com_texto_mostra_chars_e_linhas_no_topo() {
    let mut app = app_com_dados();
    app.new_query();
    if let Some(query) = app.queries.get_mut(0) {
        query.sql = String::from("select 1\nselect 2");
    }
    let tela = screen_at(&mut app, 130);
    assert!(tela.contains("query · q1"), "{tela}");
    assert!(tela.contains("17 chars"), "{tela}");
    assert!(tela.contains("2 linhas"), "{tela}");
    assert!(tela.contains("select 1"), "{tela}");
}

#[test]
fn terminal_estreito_mostra_queries_na_lateral_quando_focada() {
    let mut app = app_com_dados();
    app.new_query();
    app.focus = Focus::Queries;
    // Sem espaço para três colunas, a esquerda vira a lista de queries.
    let tela = screen_at(&mut app, 64);
    assert!(tela.contains("queries"), "{tela}");
    assert!(tela.contains("q1"), "{tela}");
}

#[test]
fn editor_mostra_cores_de_declaracao_texto_e_numero() {
    let mut app = app_com_dados();
    app.new_query();
    if let Some(query) = app.queries.get_mut(0) {
        query.sql = String::from("SELECT * FROM t WHERE nome = 'ana' AND id = 189");
        query.fim();
    }
    let terminal = terminal_de(&mut app, 130, 12);
    let buffer = terminal.backend().buffer();

    let (x, y) = achar(buffer, "SELECT");
    assert_eq!(buffer.cell((x, y)).unwrap().fg, crate::theme::MAUVE);
    let (xw, _) = achar(buffer, "WHERE");
    assert_eq!(buffer.cell((xw, y)).unwrap().fg, crate::theme::MAUVE);
    let (xt, _) = achar(buffer, "'ana'");
    assert_eq!(buffer.cell((xt, y)).unwrap().fg, crate::theme::GREEN);
    let (xn, _) = achar(buffer, "189");
    assert_eq!(buffer.cell((xn, y)).unwrap().fg, crate::theme::PEACH);
}

#[test]
fn cursor_do_editor_inverte_o_caractere() {
    let mut app = app_com_dados();
    app.new_query();
    app.queries[0].sql = String::from("xqy");
    app.queries[0].cursor = 1;
    let terminal = terminal_de(&mut app, 130, 12);
    let buffer = terminal.backend().buffer();
    let (x, y) = achar(buffer, "xqy");
    let celula = buffer.cell((x + 1, y)).expect("letra sob o cursor");
    assert_eq!(celula.symbol(), "q");
    assert_eq!(celula.fg, crate::theme::BASE);
    assert_eq!(celula.bg, crate::theme::MAUVE);
}

#[test]
fn execucao_abre_o_resultado_embaixo_do_editor() {
    let tela = screen_at(&mut app_com_resultado(), 130);
    // Editor em cima, com o SQL; resultado embaixo, com tabela e contagem.
    assert!(tela.contains("SELECT * FROM users"), "{tela}");
    assert!(tela.contains("resultado · users · 1 registros"), "{tela}");
    assert!(tela.contains("1 registros"), "{tela}");
    // A grade principal não aparece: o centro é só editor + resultado.
    assert!(!tela.contains("dados ·"), "{tela}");
}

#[test]
fn sem_dados_o_resultado_mostra_o_carregamento() {
    let mut app = app_com_dados();
    app.new_query();
    app.query_result = Some(ResultadoQuery {
        sql: String::from("SELECT * FROM users"),
        tabela: Some(String::from("users")),
        ..ResultadoQuery::default()
    });
    let tela = screen_at(&mut app, 130);
    assert!(tela.contains("resultado · users · 0 registros"), "{tela}");
    assert!(tela.contains("carregando consulta"), "{tela}");
}
