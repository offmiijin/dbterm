use super::*;
use crate::app::Commands;

fn app_com_comandos() -> App {
    let mut app = app_com_dados();
    app.commands = Some(Commands::new());
    app
}

#[test]
fn popup_sql_mostra_a_tabela_fixada() {
    let mut app = app_com_dados();
    let mut commands = Commands::new();
    commands.tabela = Some(String::from("users"));
    app.commands = Some(commands);
    let tela = screen(&mut app);

    assert!(tela.contains("tabela users"), "{tela}");
}

#[test]
fn popup_sql_sem_tabela_avisa() {
    let mut app = app_com_comandos();
    let tela = screen(&mut app);

    assert!(tela.contains("sem tabela aberta"), "{tela}");
}

#[test]
fn popup_de_comandos_mostra_abas_prefixos_e_acoes() {
    let mut app = app_com_comandos();
    let tela = screen(&mut app);

    assert!(tela.contains("Comandos"), "{tela}");
    assert!(tela.contains("[\u{f1c0} SQL ;]"), "{tela}");
    assert!(tela.contains("\u{f0ce} Tabela #"), "{tela}");
    assert!(tela.contains(";"), "{tela}");
    assert!(
        screen_at(&mut app_com_comandos(), 100)
            .contains("ctrl+enter executar · tab aba · ctrl+j/k sugestão · esc fechar"),
        "rodapé com os atalhos"
    );
}

#[test]
fn digitacao_no_comando_aparece_na_tela() {
    let mut app = app_com_comandos();
    if let Some(commands) = app.commands.as_mut() {
        for caractere in "select 1".chars() {
            commands.type_char(caractere);
        }
    }
    let tela = screen(&mut app);
    assert!(tela.contains(";select 1"), "{tela}");
}

#[test]
fn busca_de_tabela_mostra_a_caixa_de_completar() {
    let mut app = app_com_dados();
    app.tables = vec![
        String::from("orders"),
        String::from("products"),
        String::from("profiles"),
    ];
    app.commands = Some(Commands::with_input(String::from("#pro")));
    let tela = screen(&mut app);

    assert!(tela.contains("[\u{f0ce} Tabela #]"), "{tela}");
    assert!(tela.contains("#pro"), "{tela}");
    // A caixa completa `#pro` com as tabelas que combinam.
    assert!(tela.contains("products"), "{tela}");
    assert!(tela.contains("profiles"), "{tela}");
    assert!(!tela.contains("orders"), "{tela}");
    // A primeira sugestão (`enter` abre ela) sai em destaque.
    assert!(tela.contains("❯ products"), "{tela}");
    // A dica de navegação só aparece na Tabela.
    assert!(
        tela.contains("ctrl+j desce · ctrl+k sobe na caixa"),
        "{tela}"
    );
}

#[test]
fn dica_de_navegacao_nao_aparece_no_sql() {
    let tela = screen(&mut app_com_comandos());
    assert!(!tela.contains("ctrl+j desce"), "{tela}");
}

#[test]
fn destaque_da_caixa_acompanha_a_navegacao() {
    let mut app = app_com_dados();
    app.tables = vec![
        String::from("orders"),
        String::from("products"),
        String::from("profiles"),
    ];
    let mut commands = Commands::with_input(String::from("#pro"));
    commands.sugestao = 1;
    app.commands = Some(commands);
    let tela = screen(&mut app);

    assert!(tela.contains("❯ profiles"), "{tela}");
    assert!(!tela.contains("❯ products"), "{tela}");
}

#[test]
fn busca_sem_tabela_avisa_na_caixa() {
    let mut app = app_com_dados();
    app.tables = vec![String::from("orders")];
    app.commands = Some(Commands::with_input(String::from("#xyz")));
    let tela = screen(&mut app);

    assert!(tela.contains("nenhuma tabela encontrada"), "{tela}");
}
