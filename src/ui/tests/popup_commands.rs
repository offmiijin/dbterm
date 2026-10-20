use super::*;
use crate::app::Commands;

fn app_com_comandos() -> App {
    let mut app = app_com_dados();
    app.commands = Some(Commands::new());
    app
}

#[test]
fn popup_de_comandos_mostra_aba_sql_e_prefixo() {
    let mut app = app_com_comandos();
    let tela = screen(&mut app);

    assert!(tela.contains("Comandos"), "{tela}");
    assert!(tela.contains("[SQL ;]"), "{tela}");
    assert!(tela.contains(";"), "{tela}");
    assert!(tela.contains("ctrl+enter executar · esc fechar"), "{tela}");
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
