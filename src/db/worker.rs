//! A thread de trabalho: uma conexão MySQL, um canal de pedidos e um de
//! respostas.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use mysql::{Conn, Opts};

use super::mysql::{rows, tables, update};
use super::{Request, Response};

/// Canal da UI para a thread de trabalho e volta.
pub struct Db {
    requests: Sender<Request>,
    responses: Receiver<Response>,
}

impl Db {
    pub fn connect(url: &str) -> Self {
        let (req_tx, req_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();
        let url = url.to_owned();
        thread::spawn(move || worker(&url, &res_tx, &req_rx));
        Self {
            requests: req_tx,
            responses: res_rx,
        }
    }

    pub fn request(&self, request: Request) {
        let _ = self.requests.send(request);
    }

    /// A resposta que já chegou, se houver. `None` também quando a thread morreu.
    pub fn poll(&self) -> Option<Response> {
        match self.responses.try_recv() {
            Ok(response) => Some(response),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Response::Failed(String::from("conexão encerrada")))
            }
        }
    }
}

fn worker(url: &str, tx: &Sender<Response>, rx: &Receiver<Request>) {
    let opts = match Opts::from_url(url) {
        Ok(opts) => opts,
        Err(err) => return fail(tx, format!("URL inválida: {err}")),
    };
    let database = opts.get_db_name().unwrap_or("(nenhum schema)").to_owned();

    let mut conn = match Conn::new(opts) {
        Ok(conn) => conn,
        Err(err) => return fail(tx, format!("falha ao conectar: {err}")),
    };

    let (major, minor, patch) = conn.server_version();
    let _ = tx.send(Response::Connected {
        server: format!("MySQL {major}.{minor}.{patch}"),
        database,
    });

    while let Ok(request) = rx.recv() {
        let response = match request {
            Request::Tables => match tables(&mut conn) {
                Ok(tables) => Response::Tables(tables),
                Err(err) => Response::Failed(err.to_string()),
            },
            Request::Rows { table } => match rows(&mut conn, &table) {
                Ok(data) => Response::Rows { table, data },
                Err(err) => Response::Failed(err.to_string()),
            },
            Request::Update {
                table,
                column,
                value,
                key,
            } => match update(&mut conn, &table, &column, value, &key) {
                Ok(()) => Response::Updated { table },
                Err(err) => Response::Failed(err),
            },
        };
        if tx.send(response).is_err() {
            return;
        }
    }
}

fn fail(tx: &Sender<Response>, message: String) {
    let _ = tx.send(Response::Failed(message));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Não precisa de servidor: a porta 1 recusa na hora e o erro volta
    /// pelo canal, que é o caminho que a UI usa para mostrar falhas.
    #[test]
    fn falha_de_conexao_volta_como_erro() {
        use std::time::{Duration, Instant};

        let db = Db::connect("mysql://user@127.0.0.1:1/app");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(Response::Failed(message)) = db.poll() {
                assert!(message.contains("conectar"), "{message}");
                return;
            }
            assert!(Instant::now() < deadline, "worker não respondeu");
            thread::sleep(Duration::from_millis(20));
        }
    }
}
