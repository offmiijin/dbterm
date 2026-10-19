//! As consultas ao MySQL e a formatação dos valores que vêm dele.

use mysql::consts::ColumnType;
use mysql::prelude::Queryable;
use mysql::{Conn, Value};

use super::{
    Charset, Collation, ColumnKind, DatabaseAlter, DatabaseInfo, TableData, identificador,
};

pub(super) fn tables(conn: &mut Conn) -> mysql::Result<Vec<String>> {
    conn.query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' \
         ORDER BY table_name",
    )
}

/// Bancos do servidor; o schema atual não precisa existir.
pub(super) fn databases(conn: &mut Conn) -> mysql::Result<Vec<String>> {
    conn.query("SELECT schema_name FROM information_schema.schemata ORDER BY schema_name")
}

/// Propriedades do banco: os padrões de charset/collation e, quando o servidor
/// informa, a opção de somente leitura.
pub(super) fn database_info(conn: &mut Conn, database: &str) -> Result<DatabaseInfo, String> {
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
fn charsets(conn: &mut Conn) -> mysql::Result<Vec<Charset>> {
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
fn collations(conn: &mut Conn) -> mysql::Result<Vec<Collation>> {
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

/// Aplica no servidor as opções que mudaram no banco.
pub(super) fn alter_database(
    conn: &mut Conn,
    database: &str,
    alter: &DatabaseAlter,
) -> Result<(), String> {
    let Some(sql) = alter_database_sql(database, alter)? else {
        return Ok(());
    };
    conn.query_drop(sql).map_err(|err| err.to_string())
}

/// Monta o `ALTER DATABASE` só com o que mudou; sem mudança, não há query.
///
/// Charset e collation são nomes de identificador e vão para o texto do DDL:
/// [`identificador`] barra qualquer coisa fora de letras, números e `_`.
fn alter_database_sql(database: &str, alter: &DatabaseAlter) -> Result<Option<String>, String> {
    let mut opcoes = Vec::new();
    if let Some(charset) = &alter.charset {
        opcoes.push(format!("CHARACTER SET = {}", nome(charset, "charset")?));
    }
    if let Some(collation) = &alter.collation {
        opcoes.push(format!("COLLATE = {}", nome(collation, "collation")?));
    }
    if let Some(read_only) = alter.read_only {
        opcoes.push(format!("READ ONLY = {}", u8::from(read_only)));
    }
    if opcoes.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "ALTER DATABASE `{}` {}",
        quote(database),
        opcoes.join(" ")
    )))
}

/// Nome de charset/collation, recusado quando não é um identificador simples.
fn nome(valor: &str, tipo: &str) -> Result<String, String> {
    if identificador(valor) {
        Ok(valor.to_owned())
    } else {
        Err(format!("{tipo} inválido: {valor}"))
    }
}

/// Troca o banco padrão da conexão, sem reabrir a sessão.
pub(super) fn use_database(conn: &mut Conn, database: &str) -> mysql::Result<()> {
    conn.query_drop(format!("USE `{}`", quote(database)))
}

pub(super) fn rows(
    conn: &mut Conn,
    table: &str,
    offset: usize,
    limit: usize,
) -> mysql::Result<TableData> {
    let sql = rows_sql(table, offset, limit);
    let mut result = conn.query_iter(sql)?;
    let columns = result.columns();
    let kinds: Vec<ColumnKind> = columns
        .as_ref()
        .iter()
        .map(|column| kind_of(column.column_type()))
        .collect();
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
        kinds,
        rows,
        primary_key,
    })
}

/// `SELECT *` de uma página da tabela. O identificador não vai como parâmetro:
/// escapamos com crase dobrada.
fn rows_sql(table: &str, offset: usize, limit: usize) -> String {
    format!(
        "SELECT * FROM `{}` LIMIT {limit} OFFSET {offset}",
        quote(table)
    )
}

/// Número ou texto, a partir do tipo que veio no result set.
fn kind_of(column_type: ColumnType) -> ColumnKind {
    if column_type.is_numeric_type() {
        ColumnKind::Number
    } else {
        ColumnKind::Text
    }
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
pub(super) fn update(
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
    fn monta_select_paginado() {
        assert_eq!(
            rows_sql("users", 0, 200),
            "SELECT * FROM `users` LIMIT 200 OFFSET 0"
        );
        assert_eq!(
            rows_sql("we`ird", 200, 200),
            "SELECT * FROM `we``ird` LIMIT 200 OFFSET 200"
        );
    }

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

    #[test]
    fn monta_alter_database_com_as_opcoes_alteradas() {
        let alter = DatabaseAlter {
            charset: Some(String::from("utf8mb4")),
            collation: Some(String::from("utf8mb4_0900_ai_ci")),
            read_only: Some(true),
        };
        assert_eq!(
            alter_database_sql("app", &alter).unwrap().unwrap(),
            "ALTER DATABASE `app` CHARACTER SET = utf8mb4 \
             COLLATE = utf8mb4_0900_ai_ci READ ONLY = 1"
        );
    }

    #[test]
    fn alter_database_sem_mudanca_nao_vira_query() {
        assert!(
            alter_database_sql("app", &DatabaseAlter::default())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn alter_database_escapa_crases_do_nome_do_banco() {
        let alter = DatabaseAlter {
            read_only: Some(false),
            ..DatabaseAlter::default()
        };
        assert_eq!(
            alter_database_sql("we`ird", &alter).unwrap().unwrap(),
            "ALTER DATABASE `we``ird` READ ONLY = 0"
        );
    }

    #[test]
    fn alter_database_recusa_charset_ou_collation_estranhos() {
        let charset = DatabaseAlter {
            charset: Some(String::from("utf8mb4; DROP DATABASE app")),
            ..DatabaseAlter::default()
        };
        assert!(alter_database_sql("app", &charset).is_err());

        let vazio = DatabaseAlter {
            collation: Some(String::new()),
            ..DatabaseAlter::default()
        };
        assert!(alter_database_sql("app", &vazio).is_err());
    }

    #[test]
    fn tipo_da_coluna_vira_numero_ou_texto() {
        assert_eq!(kind_of(ColumnType::MYSQL_TYPE_LONG), ColumnKind::Number);
        assert_eq!(kind_of(ColumnType::MYSQL_TYPE_FLOAT), ColumnKind::Number);
        assert_eq!(
            kind_of(ColumnType::MYSQL_TYPE_NEWDECIMAL),
            ColumnKind::Number
        );
        assert_eq!(kind_of(ColumnType::MYSQL_TYPE_VAR_STRING), ColumnKind::Text);
        assert_eq!(kind_of(ColumnType::MYSQL_TYPE_DATETIME), ColumnKind::Text);
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
