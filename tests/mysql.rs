//! Teste de integração: sobe um MySQL em container (com tabelas e dados) e
//! confere que o dbterm lê o que foi semeado.
//!
//! O container **fica no ar** de propósito, para você abrir a TUI nele:
//!
//! ```text
//! cargo run -- mysql://dbterm:dbterm@127.0.0.1:13306/dbterm
//! ```
//!
//! Para derrubar (o `-v` apaga os dados e força nova semeadura):
//!
//! ```text
//! podman compose -f tests/compose.yaml down -v
//! ```
//!
//! Sem `docker compose`, `podman compose` ou `podman-compose` no PATH o teste
//! é pulado em vez de falhar.

use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use dbterm::db::{Db, ROW_LIMIT, Request, Response, TableData};

const URL: &str = "mysql://dbterm:dbterm@127.0.0.1:13306/dbterm";
/// A primeira execução baixa a imagem do MySQL: seja generoso.
const ESPERA: Duration = Duration::from_secs(600);

#[test]
fn container_mysql_serve_dados_para_a_tui() {
    let Some(compose) = compose() else {
        eprintln!("SKIP: não achei `docker compose` nem `podman compose` no PATH");
        return;
    };

    let arquivo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/compose.yaml");
    let arquivo = arquivo.to_str().expect("caminho do compose não é UTF-8");
    if let Err(erro) = rodar(&compose, &["-f", arquivo, "up", "-d"]) {
        panic!("não consegui subir o container:\n{erro}");
    }

    let db = conectar();

    let tabelas = tabelas(&db);
    for esperada in ["users", "products", "orders", "events"] {
        assert!(
            tabelas.contains(&esperada.to_string()),
            "faltou {esperada}: {tabelas:?}"
        );
    }

    let usuarios = paginas(&db, "users");
    assert!(
        usuarios.rows.len() >= 30,
        "esperava 30+ usuários, veio {}",
        usuarios.rows.len()
    );
    let valores = usuarios.rows.iter().flatten().collect::<Vec<_>>();
    assert!(
        valores.iter().any(|valor| valor.contains('ç')),
        "esperava acento"
    );
    assert!(
        valores.iter().any(|valor| valor.contains('石')),
        "esperava kanji"
    );
    assert!(
        valores.iter().any(|valor| valor.contains("👩🏽‍💻")),
        "esperava emoji"
    );
    assert!(
        valores.iter().any(|valor| valor.as_str() == "NULL"),
        "esperava NULL"
    );

    let pedidos = paginas(&db, "orders");
    assert!(
        pedidos.columns.contains(&String::from("total")),
        "colunas: {:?}",
        pedidos.columns
    );
    assert!(
        pedidos.rows.len() >= 40,
        "esperava 40+ pedidos, veio {}",
        pedidos.rows.len()
    );

    // A grade do dbterm busca no máximo ROW_LIMIT linhas por consulta.
    let eventos = paginas(&db, "events");
    assert_eq!(eventos.rows.len(), ROW_LIMIT);

    eprintln!(
        "\nMySQL de teste no ar: {} tabelas, {} usuários, {} pedidos, {} eventos.\n\
         Abra a TUI com:\n    cargo run -- {URL}\n\
         Para derrubar:\n    {} -f {arquivo} down -v\n",
        tabelas.len(),
        usuarios.rows.len(),
        pedidos.rows.len(),
        eventos.rows.len(),
        compose.join(" ")
    );
}

/// O comando de compose disponível, na ordem de preferência.
fn compose() -> Option<Vec<String>> {
    let candidatos = [
        vec!["docker", "compose"],
        vec!["podman", "compose"],
        vec!["podman-compose"],
    ];
    candidatos
        .into_iter()
        .find(|candidato| {
            Command::new(candidato[0])
                .args(&candidato[1..])
                .arg("version")
                .output()
                .is_ok_and(|saida| saida.status.success())
        })
        .map(|candidato| candidato.iter().map(|parte| parte.to_string()).collect())
}

fn rodar(compose: &[String], argumentos: &[&str]) -> Result<String, String> {
    let saida = Command::new(&compose[0])
        .args(&compose[1..])
        .args(argumentos)
        .output()
        .map_err(|erro| erro.to_string())?;
    let texto = format!(
        "{}{}",
        String::from_utf8_lossy(&saida.stdout),
        String::from_utf8_lossy(&saida.stderr)
    );
    if saida.status.success() {
        Ok(texto)
    } else {
        Err(texto)
    }
}

/// Conecta do mesmo jeito que a TUI: worker próprio, tentando de novo enquanto o
/// MySQL inicializa (imagem, banco, seed e só então a porta aceita conexão).
fn conectar() -> Db {
    let prazo = Instant::now() + ESPERA;
    let mut ultimo = String::from("sem resposta");
    while Instant::now() < prazo {
        let db = Db::connect(URL);
        match esperar(&db, Duration::from_secs(5)) {
            Some(Response::Connected { .. }) => return db,
            Some(Response::Failed(erro)) => ultimo = erro,
            _ => {}
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("MySQL de teste não respondeu em {ESPERA:?}: {ultimo}");
}

fn tabelas(db: &Db) -> Vec<String> {
    db.request(Request::Tables);
    match esperar(db, Duration::from_secs(30)) {
        Some(Response::Tables(tabelas)) => tabelas,
        outra => panic!("esperava a lista de tabelas, veio {outra:?}"),
    }
}

fn paginas(db: &Db, tabela: &str) -> TableData {
    db.request(Request::Rows {
        table: tabela.to_string(),
    });
    match esperar(db, Duration::from_secs(30)) {
        Some(Response::Rows { data, .. }) => data,
        outra => panic!("esperava as linhas de {tabela}, veio {outra:?}"),
    }
}

/// A próxima resposta do worker, esperando até o prazo estourar.
fn esperar(db: &Db, prazo: Duration) -> Option<Response> {
    let limite = Instant::now() + prazo;
    loop {
        if let Some(resposta) = db.poll() {
            return Some(resposta);
        }
        if Instant::now() >= limite {
            return None;
        }
        thread::sleep(Duration::from_millis(20));
    }
}
