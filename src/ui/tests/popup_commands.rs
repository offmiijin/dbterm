use super::*;
use crate::app::Commands;

fn app_com_comandos() -> App {
    let mut app = app_com_dados();
    app.commands = Some(Commands::new());
    app
}

#[test]
fn popup_de_comandos_mostra_abas_prefixos_e_acoes() {
    let mut app = app_com_comandos();
    let tela = screen(&mut app);

    assert!(tela.contains("Comandos"), "{tela}");
    assert!(tela.contains("[SQL ;]"), "{tela}");
    assert!(tela.contains("Table #"), "{tela}");
    assert!(tela.contains(";"), "{tela}");
    assert!(
        tela.contains("ctrl+enter executar · tab aba · esc fechar"),
        "{tela}"
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

    assert!(tela.contains("[Table #]"), "{tela}");
    assert!(tela.contains("#pro"), "{tela}");
    // A caixa completa `#pro` com as tabelas que combinam.
    assert!(tela.contains("products"), "{tela}");
    assert!(tela.contains("profiles"), "{tela}");
    assert!(!tela.contains("orders"), "{tela}");
    // A primeira sugestão (`enter` abre ela) sai em destaque.
    assert!(tela.contains("❯ products"), "{tela}");
}

#[test]
fn busca_sem_tabela_avisa_na_caixa() {
    let mut app = app_com_dados();
    app.tables = vec![String::from("orders")];
    app.commands = Some(Commands::with_input(String::from("#xyz")));
    let tela = screen(&mut app);

    assert!(tela.contains("nenhuma tabela encontrada"), "{tela}");
}
