//! dbterm: gerenciador de banco de dados via terminal.

use std::io;

use dbterm::app::App;

fn main() -> io::Result<()> {
    let url = match url(std::env::args().nth(1)) {
        Ok(url) => url,
        Err(message) => {
            eprintln!("dbterm: {message}");
            eprintln!("uso: dbterm <mysql://usuario:senha@host:porta/schema>");
            std::process::exit(2);
        }
    };

    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("não consegui iniciar o terminal: {err}"),
            ));
        }
    };
    let result = App::new(&url).run(&mut terminal);
    ratatui::restore();
    result
}

/// A URL vem do argumento ou, sem ele, de `DATABASE_URL` / `MYSQL_URL`.
fn url(argument: Option<String>) -> Result<String, String> {
    let url = argument
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .or_else(|| std::env::var("MYSQL_URL").ok())
        .ok_or("informe a URL de conexão")?;
    if !url.starts_with("mysql://") {
        return Err(format!("URL não suportada: {url}"));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::url;

    #[test]
    fn aceita_somente_mysql() {
        assert!(url(Some(String::from("mysql://user@localhost/app"))).is_ok());
        assert!(url(Some(String::from("postgres://localhost/app"))).is_err());
        assert!(url(Some(String::from("localhost"))).is_err());
    }
}
