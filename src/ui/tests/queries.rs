use super::*;

#[test]
fn lateral_direita_mostra_a_aba_queries_com_as_instancias() {
    let mut app = app_com_dados();
    app.new_query();
    app.focus = Focus::Queries;
    let tela = screen_at(&mut app, 130);
    assert!(tela.contains("queries"), "{tela}");
    assert!(tela.contains("q1"), "{tela}");
    assert!(tela.contains("vazio"), "{tela}");
    assert!(tela.contains("n nova"), "{tela}");
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
