//! dbterm: gerenciador de banco de dados via terminal.

use std::io;

use dbterm::app::App;
use dbterm::config::url;
use dbterm::history::migrar_chave;
use ratatui::crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;

fn main() -> io::Result<()> {
    let url = match url(std::env::args().nth(1)) {
        Ok(url) => url,
        Err(message) => {
            eprintln!("dbterm: {message}");
            eprintln!("uso: dbterm [mysql://usuario:senha@host:porta/schema]");
            std::process::exit(2);
        }
    };

    // A chave da cifra sai do `chave.bin` para o cofre do sistema, quando há
    // cofre; sem ele, o arquivo continua sendo usado.
    migrar_chave();

    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("não consegui iniciar o terminal: {err}"),
            ));
        }
    };
    // Teclado estendido (protocolo Kitty): sem ele o terminal entrega o
    // `ctrl+enter` como um `enter` comum e o editor de query quebraria linha
    // em vez de executar. Terminais sem suporte ignoram o pedido e seguem
    // como antes.
    let teclado_aprimorado = execute!(
        io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )
    .is_ok();
    let result = App::new(url.as_deref()).run(&mut terminal);
    if teclado_aprimorado {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}
