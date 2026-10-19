//! dbterm: gerenciador de banco de dados via terminal.

use std::io;

use dbterm::app::App;
use dbterm::config::url;
use dbterm::history::migrar_chave;

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
    let result = App::new(url.as_deref()).run(&mut terminal);
    ratatui::restore();
    result
}
