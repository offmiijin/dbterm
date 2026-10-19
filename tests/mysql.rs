//! Teste de integração: garante um MySQL em container (com tabelas e dados) e
//! confere que o dbterm lê o que foi semeado.
//!
//! Se o container já estiver no ar, ele é reaproveitado; senão o teste sobe com
//! `docker compose` / `podman compose` / `podman-compose`. O container **fica no
//! ar** de propósito, para você abrir a TUI nele:
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
//! Sem container no ar e sem nenhum compose no PATH o teste é pulado em vez de
//! falhar.

use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use dbterm::db::{Db, ROW_LIMIT, Request, Response, TableData};

const URL: &str = "mysql://dbterm:dbterm@127.0.0.1:13306/dbterm";
/// Conexão recusada é imediata: não vale esperar muito antes de decidir subir.
const TENTATIVA_CURTA: Duration = Duration::from_secs(3);
/// A primeira subida baixa a imagem do MySQL: seja generoso.
const ESPERA: Duration = Duration::from_secs(600);

#[test]
fn container_mysql_serve_dados_para_a_tui() {
    let arquivo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/compose.yaml");
    let arquivo = arquivo.to_str().expect("caminho do compose não é UTF-8");

    let (db, subiu) = match conectar(TENTATIVA_CURTA) {
        Some(db) => {
            eprintln!("container já estava no ar, reaproveitando");
            (db, None)
        }
        None => match compose() {
            Some(compose) => (subir(&compose, arquivo), Some(compose)),
            None => {
                eprintln!(
                    "SKIP: {URL} não respondeu e não achei `docker compose`, \
                     `podman compose` nem `podman-compose` no PATH"
                );
                return;
            }
        },
    };

    let tabelas = tabelas(&db);
    for esperada in ["users", "products", "orders", "events"] {
        assert!(
            tabelas.contains(&esperada.to_string()),
            "faltou {esperada}: {tabelas:?}"
        );
    }

    let usuarios = paginas(&db, "users", 0);
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

    let pedidos = paginas(&db, "orders", 0);
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

    // A grade do dbterm busca ROW_LIMIT linhas por página e vai pedindo as
    // seguintes conforme o usuário desce: 500 eventos = 200 + 200 + 100.
    let eventos = paginas(&db, "events", 0);
    assert_eq!(eventos.rows.len(), ROW_LIMIT);
    let segunda = paginas(&db, "events", ROW_LIMIT);
    assert_eq!(segunda.rows.len(), ROW_LIMIT);
    let ultima = paginas(&db, "events", 2 * ROW_LIMIT);
    assert_eq!(
        ultima.rows.len(),
        100,
        "500 eventos: a última página tem 100"
    );
    // Página além do fim: a tabela acabou.
    assert!(paginas(&db, "events", 500).rows.is_empty());

    // Primeiro caminho de escrita: a chave primária vem junto com a página.
    let id_idx = usuarios
        .columns
        .iter()
        .position(|coluna| coluna == "id")
        .expect("coluna id");
    let notes_idx = usuarios
        .columns
        .iter()
        .position(|coluna| coluna == "notes")
        .expect("coluna notes");
    assert_eq!(usuarios.primary_key, vec![id_idx]);

    // Atualiza o `notes` de um usuário gerado (30) e restaura para NULL.
    let alvo = usuarios
        .rows
        .iter()
        .position(|linha| linha[id_idx] == "30")
        .expect("usuário 30");
    let id = usuarios.rows[alvo][id_idx].clone();
    db.request(Request::Update {
        table: String::from("users"),
        column: String::from("notes"),
        value: Some(String::from("editado pela tui")),
        key: vec![(String::from("id"), id.clone())],
    });
    match esperar(&db, Duration::from_secs(30)) {
        Some(Response::Updated { .. }) => {}
        outra => panic!("esperava Updated depois do UPDATE, veio {outra:?}"),
    }
    let depois = paginas(&db, "users", 0);
    let atualizado = depois
        .rows
        .iter()
        .find(|linha| linha[id_idx] == id)
        .expect("usuário atualizado");
    assert_eq!(atualizado[notes_idx], "editado pela tui");

    db.request(Request::Update {
        table: String::from("users"),
        column: String::from("notes"),
        value: None,
        key: vec![(String::from("id"), id)],
    });
    match esperar(&db, Duration::from_secs(30)) {
        Some(Response::Updated { .. }) => {}
        outra => panic!("esperava Updated ao restaurar, veio {outra:?}"),
    }

    let derrubar = subiu.map_or_else(
        || String::from("podman compose"),
        |compose| compose.join(" "),
    );
    eprintln!(
        "\nMySQL de teste no ar: {} tabelas, {} usuários, {} pedidos, {} eventos.\n\
         Abra a TUI com:\n    cargo run -- {URL}\n\
         Para derrubar:\n    {derrubar} -f {arquivo} down -v\n",
        tabelas.len(),
        usuarios.rows.len(),
        pedidos.rows.len(),
        eventos.rows.len(),
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

/// Sobe o container pelo compose e espera o MySQL aceitar conexão.
fn subir(compose: &[String], arquivo: &str) -> Db {
    if let Err(erro) = rodar(compose, &["-f", arquivo, "up", "-d"]) {
        panic!("não consegui subir o container:\n{erro}");
    }
    conectar(ESPERA).unwrap_or_else(|| panic!("MySQL de teste não respondeu em {ESPERA:?}"))
}

/// Conecta do mesmo jeito que a TUI: worker próprio, tentando de novo enquanto o
/// MySQL inicializa (imagem, banco, seed e só então a porta aceita conexão).
fn conectar(prazo: Duration) -> Option<Db> {
    let limite = Instant::now() + prazo;
    let mut ultimo = String::from("sem resposta");
    loop {
        let db = Db::connect(URL);
        match esperar(&db, Duration::from_secs(5)) {
            Some(Response::Connected { .. }) => return Some(db),
            Some(Response::Failed(erro)) => ultimo = erro,
            _ => {}
        }
        if Instant::now() >= limite {
            eprintln!("não consegui conectar em {URL}: {ultimo}");
            return None;
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn tabelas(db: &Db) -> Vec<String> {
    db.request(Request::Tables);
    match esperar(db, Duration::from_secs(30)) {
        Some(Response::Tables(tabelas)) => tabelas,
        outra => panic!("esperava a lista de tabelas, veio {outra:?}"),
    }
}

fn paginas(db: &Db, tabela: &str, offset: usize) -> TableData {
    db.request(Request::Rows {
        table: tabela.to_string(),
        offset,
        limit: ROW_LIMIT,
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
