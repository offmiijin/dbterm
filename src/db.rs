//! Acesso ao MySQL: uma thread de trabalho por conexão.
//!
//! A UI nunca fala com o banco direto. Ela envia [`Request`] por canal e lê
//! [`Response`] no loop de eventos, então uma query lenta nunca trava o desenho.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use mysql::prelude::Queryable;
use mysql::{Conn, Opts, Value};

/// Quantas linhas de cada tabela buscamos por página (MVP: sem paginação).
pub const ROW_LIMIT: usize = 200;

/// Uma página de dados pronta para desenhar: tudo já virou texto.
#[derive(Debug)]
pub struct TableData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Índices das colunas da chave primária; vazio quando a tabela não tem PK.
    pub primary_key: Vec<usize>,
}

#[derive(Debug)]
pub enum Request {
    /// Lista as tabelas do schema atual.
    Tables,
    /// Busca as primeiras [`ROW_LIMIT`] linhas de uma tabela.
    Rows { table: String },
    /// Grava `value` (`None` = `NULL`) na coluna `column` da linha identificada
    /// pelos valores atuais da chave primária.
    Update {
        table: String,
        column: String,
        value: Option<String>,
        key: Vec<(String, String)>,
    },
}

#[derive(Debug)]
pub enum Response {
    Connected {
        server: String,
        database: String,
    },
    Tables(Vec<String>),
    Rows {
        table: String,
        data: TableData,
    },
    /// O `UPDATE` foi aplicado; a UI recarrega a tabela.
    Updated {
        table: String,
    },
    Failed(String),
}

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

fn tables(conn: &mut Conn) -> mysql::Result<Vec<String>> {
    conn.query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' \
         ORDER BY table_name",
    )
}

fn rows(conn: &mut Conn, table: &str) -> mysql::Result<TableData> {
    // Identificador não vai como parâmetro: escapamos com crase dobrada.
    let sql = format!(
        "SELECT * FROM `{}` LIMIT {ROW_LIMIT}",
        table.replace('`', "``")
    );
    let mut result = conn.query_iter(sql)?;
    let columns = result.columns();
    let columns: Vec<String> = columns
        .as_ref()
        .iter()
        .map(|column| column.name_str().into_owned())
        .collect();

    let mut rows = Vec::new();
    if let Some(set) = result.iter() {
        for row in set {
            rows.push(row?.unwrap().iter().map(cell).collect());
        }
    }
    drop(result);

    let primary_key = primary_key(conn, table, &columns)?;
    Ok(TableData {
        columns,
        rows,
        primary_key,
    })
}

/// Índices das colunas que formam a chave primária da tabela.
fn primary_key(conn: &mut Conn, table: &str, columns: &[String]) -> mysql::Result<Vec<usize>> {
    let names: Vec<String> = conn.exec(
        "SELECT column_name FROM information_schema.key_column_usage \
         WHERE table_schema = DATABASE() AND table_name = ? AND constraint_name = 'PRIMARY' \
         ORDER BY ordinal_position",
        (table,),
    )?;
    Ok(names
        .iter()
        .filter_map(|name| columns.iter().position(|column| column == name))
        .collect())
}

/// Grava `value` na coluna `column` da linha cuja chave primária está em `key`.
fn update(
    conn: &mut Conn,
    table: &str,
    column: &str,
    value: Option<String>,
    key: &[(String, String)],
) -> Result<(), String> {
    if key.is_empty() {
        return Err(String::from("edição exige chave primária"));
    }
    let sql = update_sql(table, column, key);
    let mut params = Vec::with_capacity(1 + key.len());
    params.push(value.map_or(Value::NULL, Value::from));
    params.extend(key.iter().map(|(_, value)| Value::from(value.as_str())));
    conn.exec_drop(sql, params).map_err(|err| err.to_string())
}

/// `UPDATE` parametrizado: um `?` para o valor novo e um por coluna da PK.
fn update_sql(table: &str, column: &str, key: &[(String, String)]) -> String {
    let condition = key
        .iter()
        .map(|(name, _)| format!("`{}` = ?", quote(name)))
        .collect::<Vec<_>>()
        .join(" AND ");
    format!(
        "UPDATE `{}` SET `{}` = ? WHERE {condition}",
        quote(table),
        quote(column)
    )
}

/// Escapa um identificador para ir entre crases.
fn quote(identifier: &str) -> String {
    identifier.replace('`', "``")
}

/// Formata um valor do MySQL como texto de célula.
fn cell(value: &Value) -> String {
    match value {
        Value::NULL => String::from("NULL"),
        Value::Bytes(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        Value::Int(int) => int.to_string(),
        Value::UInt(uint) => uint.to_string(),
        Value::Float(float) => float.to_string(),
        Value::Double(double) => double.to_string(),
        Value::Date(year, month, day, hour, minute, second, _) => {
            if *hour == 0 && *minute == 0 && *second == 0 {
                format!("{year:04}-{month:02}-{day:02}")
            } else {
                format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
            }
        }
        Value::Time(negative, days, hours, minutes, seconds, _) => {
            let sign = if *negative { "-" } else { "" };
            let hours = days * 24 + u32::from(*hours);
            format!("{sign}{hours:02}:{minutes:02}:{seconds:02}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monta_update_parametrizado() {
        let chave = vec![(String::from("id"), String::from("1"))];
        assert_eq!(
            update_sql("users", "notes", &chave),
            "UPDATE `users` SET `notes` = ? WHERE `id` = ?"
        );
    }

    #[test]
    fn update_usa_and_em_chave_composta() {
        let chave = vec![
            (String::from("a"), String::from("1")),
            (String::from("b"), String::from("2")),
        ];
        assert_eq!(
            update_sql("t", "c", &chave),
            "UPDATE `t` SET `c` = ? WHERE `a` = ? AND `b` = ?"
        );
    }

    #[test]
    fn update_escapa_crases_nos_identificadores() {
        let chave = vec![(String::from("i`d"), String::from("1"))];
        assert_eq!(
            update_sql("we`ird", "na`me", &chave),
            "UPDATE `we``ird` SET `na``me` = ? WHERE `i``d` = ?"
        );
    }

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

    #[test]
    fn formata_valores_do_mysql() {
        assert_eq!(cell(&Value::NULL), "NULL");
        assert_eq!(cell(&Value::Int(-3)), "-3");
        assert_eq!(cell(&Value::UInt(7)), "7");
        assert_eq!(cell(&Value::Bytes(b"ana".to_vec())), "ana");
        assert_eq!(cell(&Value::Date(2026, 10, 18, 0, 0, 0, 0)), "2026-10-18");
        assert_eq!(
            cell(&Value::Date(2026, 10, 18, 13, 5, 7, 0)),
            "2026-10-18 13:05:07"
        );
        assert_eq!(cell(&Value::Time(true, 1, 2, 3, 4, 0)), "-26:03:04");
    }
}
