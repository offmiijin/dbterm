//! A thread de trabalho: um canal de pedidos e um de respostas, com a conexão
//! MySQL aberta sob demanda por [`Request::Connect`].

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use mysql::{Conn, Opts};

use super::mysql::{
    alter_database, alter_table, database_info, databases, rows, table_info, tables, update,
    use_database,
};
use super::{Request, Response};

/// Canal da UI para a thread de trabalho e volta.
pub struct Db {
    requests: Sender<Request>,
    responses: Receiver<Response>,
}

impl Db {
    /// Um `Db` ainda sem conexão: a URL chega depois, com `Request::Connect`.
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();
        thread::spawn(move || worker(&res_tx, &req_rx));
        Self {
            requests: req_tx,
            responses: res_rx,
        }
    }

    /// Um `Db` que já pede a conexão com a URL.
    pub fn connect(url: &str) -> Self {
        let db = Self::spawn();
        db.request(Request::Connect {
            url: url.to_owned(),
        });
        db
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

fn worker(tx: &Sender<Response>, rx: &Receiver<Request>) {
    // A conexão nasce no primeiro `Connect`; antes disso, os pedidos de dados
    // falham com "sem conexão" em vez de travar a UI.
    let mut conn: Option<Conn> = None;
    while let Ok(request) = rx.recv() {
        let response = match request {
            Request::Connect { url } => match open(&url) {
                Ok((opened, response)) => {
                    conn = Some(opened);
                    response
                }
                Err(message) => Response::Failed(message),
            },
            Request::Tables => match conn.as_mut() {
                Some(conn) => match tables(conn) {
                    Ok(tables) => Response::Tables(tables),
                    Err(err) => Response::Failed(err.to_string()),
                },
                None => sem_conexao(),
            },
            Request::Databases => match conn.as_mut() {
                Some(conn) => match databases(conn) {
                    Ok(databases) => Response::Databases(databases),
                    Err(err) => Response::Failed(err.to_string()),
                },
                None => sem_conexao(),
            },
            Request::DatabaseInfo { database } => match conn.as_mut() {
                Some(conn) => match database_info(conn, &database) {
                    Ok(info) => Response::DatabaseInfo(info),
                    Err(err) => Response::Failed(err),
                },
                None => sem_conexao(),
            },
            Request::AlterDatabase { database, alter } => match conn.as_mut() {
                Some(conn) => match alter_database(conn, &database, &alter) {
                    Ok(()) => Response::DatabaseUpdated { database },
                    Err(err) => Response::Failed(err),
                },
                None => sem_conexao(),
            },
            Request::TableInfo { table } => match conn.as_mut() {
                Some(conn) => match table_info(conn, &table) {
                    Ok(info) => Response::TableInfo(info),
                    Err(err) => Response::Failed(err),
                },
                None => sem_conexao(),
            },
            Request::AlterTable { table, alter } => match conn.as_mut() {
                Some(conn) => match alter_table(conn, &table, &alter) {
                    Ok(after) => Response::TableUpdated {
                        before: table,
                        after,
                    },
                    Err(err) => Response::Failed(err),
                },
                None => sem_conexao(),
            },
            Request::UseDatabase { database } => match conn.as_mut() {
                Some(conn) => match use_database(conn, &database) {
                    Ok(()) => Response::Database(database),
                    Err(err) => Response::Failed(err.to_string()),
                },
                None => sem_conexao(),
            },
            Request::Rows {
                table,
                offset,
                limit,
            } => match conn.as_mut() {
                Some(conn) => match rows(conn, &table, offset, limit) {
                    Ok(data) => Response::Rows {
                        table,
                        offset,
                        limit,
                        data,
                    },
                    Err(err) => Response::Failed(err.to_string()),
                },
                None => sem_conexao(),
            },
            Request::Update {
                table,
                column,
                value,
                key,
            } => match conn.as_mut() {
                Some(conn) => match update(conn, &table, &column, value, &key) {
                    Ok(()) => Response::Updated { table },
                    Err(err) => Response::Failed(err),
                },
                None => sem_conexao(),
            },
        };
        if tx.send(response).is_err() {
            return;
        }
    }
}

/// Abre a conexão e monta a resposta de boas-vindas, ou explica a falha.
fn open(url: &str) -> Result<(Conn, Response), String> {
    let opts = Opts::from_url(url).map_err(|err| format!("URL inválida: {err}"))?;
    let database = opts.get_db_name().unwrap_or_default().to_owned();
    let conn = Conn::new(opts).map_err(|err| format!("falha ao conectar: {err}"))?;
    let (major, minor, patch) = conn.server_version();
    let response = Response::Connected {
        server: format!("MySQL {major}.{minor}.{patch}"),
        database,
    };
    Ok((conn, response))
}

fn sem_conexao() -> Response {
    Response::Failed(String::from("sem conexão"))
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

    /// Sem `Connect`, um pedido de dados responde "sem conexão" na hora.
    #[test]
    fn pedido_sem_conexao_falha() {
        use std::time::{Duration, Instant};

        let db = Db::spawn();
        db.request(Request::Tables);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(Response::Failed(message)) = db.poll() {
                assert_eq!(message, "sem conexão");
                return;
            }
            assert!(Instant::now() < deadline, "worker não respondeu");
            thread::sleep(Duration::from_millis(20));
        }
    }
}
