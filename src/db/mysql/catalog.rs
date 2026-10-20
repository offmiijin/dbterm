//! Catálogo: listas e propriedades que o `information_schema` informa.
//!
//! Tabelas, bancos, detalhes de banco/tabela e os catálogos (charsets,
//! collations, engines) que alimentam as listas dos pop-ups.

use mysql::prelude::Queryable;
use mysql::Conn;

use super::super::{Charset, Collation, DatabaseInfo, TableInfo};

pub(crate) fn tables(conn: &mut Conn) -> mysql::Result<Vec<String>> {
    conn.query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' \
         ORDER BY table_name",
    )
}

/// Bancos do servidor; o schema atual não precisa existir.
pub(crate) fn databases(conn: &mut Conn) -> mysql::Result<Vec<String>> {
    conn.query("SELECT schema_name FROM information_schema.schemata ORDER BY schema_name")
}

/// Propriedades do banco: os padrões de charset/collation e, quando o servidor
/// informa, a opção de somente leitura.
pub(crate) fn database_info(conn: &mut Conn, database: &str) -> Result<DatabaseInfo, String> {
    let padroes = conn
        .exec::<(String, String), _, _>(
            "SELECT default_character_set_name, default_collation_name \
             FROM information_schema.schemata WHERE schema_name = ?",
            (database,),
        )
        .map_err(|err| err.to_string())?;
    let Some((charset, collation)) = padroes.into_iter().next() else {
        return Err(format!("banco não encontrado: {database}"));
    };
    Ok(DatabaseInfo {
        name: database.to_owned(),
        charset,
        collation,
        read_only: read_only(conn, database),
        size_bytes: size(conn, database).map_err(|err| err.to_string())?,
        charsets: charsets(conn).map_err(|err| err.to_string())?,
        collations: collations(conn).map_err(|err| err.to_string())?,
    })
}

/// Tamanho do banco: dados mais índices das tabelas. É o retrato que o servidor
/// dá nas estatísticas do `information_schema` — no InnoDB, aproximado.
fn size(conn: &mut Conn, database: &str) -> mysql::Result<u64> {
    let bytes: Option<u64> = conn
        .exec(
            "SELECT CAST(COALESCE(SUM(data_length + index_length), 0) AS UNSIGNED) \
             FROM information_schema.tables WHERE table_schema = ?",
            (database,),
        )?
        .into_iter()
        .next()
        .flatten();
    Ok(bytes.unwrap_or(0))
}

/// Charsets que o servidor oferece, com a collation padrão de cada um.
pub(crate) fn charsets(conn: &mut Conn) -> mysql::Result<Vec<Charset>> {
    let linhas: Vec<(String, String)> = conn.query(
        "SELECT character_set_name, default_collate_name \
         FROM information_schema.character_sets ORDER BY character_set_name",
    )?;
    Ok(linhas
        .into_iter()
        .map(|(name, default_collation)| Charset {
            name,
            default_collation,
        })
        .collect())
}

/// Collations que o servidor oferece; o charset de cada uma filtra a lista do
/// pop-up.
pub(crate) fn collations(conn: &mut Conn) -> mysql::Result<Vec<Collation>> {
    let linhas: Vec<(String, String)> = conn.query(
        "SELECT collation_name, character_set_name \
         FROM information_schema.collations ORDER BY collation_name",
    )?;
    Ok(linhas
        .into_iter()
        .map(|(name, charset)| Collation { name, charset })
        .collect())
}

/// `READ ONLY` é uma opção de `SCHEMATA_EXTENSIONS`, tabela que só existe da
/// 8.0.22 em diante: em servidor antigo a consulta falha e o campo fica
/// indisponível em vez de derrubar o pop-up.
fn read_only(conn: &mut Conn, database: &str) -> Option<bool> {
    let linhas: Vec<Option<String>> = conn
        .exec(
            "SELECT options FROM information_schema.schemata_extensions \
             WHERE schema_name = ?",
            (database,),
        )
        .ok()?;
    // A opção aparece na lista quando o banco é somente leitura; sem ela, o
    // banco aceita escrita.
    Some(
        linhas
            .into_iter()
            .next()
            .flatten()
            .is_some_and(|options| options.contains("READ ONLY=1")),
    )
}

/// Propriedades da tabela: engine, formato de linha, auto-increment, comentário
/// e os padrões de texto, com o catálogo de engines, charsets e collations.
pub(crate) fn table_info(conn: &mut Conn, table: &str) -> Result<TableInfo, String> {
    let linhas = conn
        .exec::<(
            Option<String>,
            Option<String>,
            Option<u64>,
            String,
            Option<String>,
            u64,
        ), _, _>(
            "SELECT engine, row_format, auto_increment, table_comment, table_collation, \
             CAST(COALESCE(data_length + index_length, 0) AS UNSIGNED) \
             FROM information_schema.tables \
             WHERE table_schema = DATABASE() AND table_name = ?",
            (table,),
        )
        .map_err(|err| err.to_string())?;
    let Some((engine, row_format, auto_increment, comment, collation, size_bytes)) =
        linhas.into_iter().next()
    else {
        return Err(format!("tabela não encontrada: {table}"));
    };
    let collation = collation.unwrap_or_default();
    let collations = collations(conn).map_err(|err| err.to_string())?;
    // A collation da tabela diz a que charset os padrões dela pertencem; sem
    // essa informação (tabela sem colunas de texto), o campo fica vazio.
    let charset = collations
        .iter()
        .find(|candidata| candidata.name == collation)
        .map(|candidata| candidata.charset.clone())
        .unwrap_or_default();
    Ok(TableInfo {
        name: table.to_owned(),
        comment,
        engine: engine.unwrap_or_default(),
        row_format: row_format.unwrap_or_default(),
        auto_increment,
        charset,
        collation,
        size_bytes,
        engines: engines(conn).map_err(|err| err.to_string())?,
        charsets: charsets(conn).map_err(|err| err.to_string())?,
        collations,
    })
}

/// Engines que o servidor aceita; as indisponíveis (`NO`, `DISABLED`) ficam de
/// fora da lista.
fn engines(conn: &mut Conn) -> mysql::Result<Vec<String>> {
    conn.query(
        "SELECT engine FROM information_schema.engines \
         WHERE support IN ('YES', 'DEFAULT') ORDER BY engine",
    )
}
