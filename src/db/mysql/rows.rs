//! Leitura e escrita de linhas: `SELECT` paginado e `UPDATE` por PK.
//!
//! Tudo chega à UI como texto em [`TableData`]; a chave primária acompanha a
//! página para o `UPDATE` saber qual linha gravar.

use mysql::prelude::Queryable;
use mysql::{Conn, Value};

use super::super::{ColumnKind, TableData};
use super::values::{cell, kind_of, quote};

pub(crate) fn rows(
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
pub(crate) fn update(
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

/// Troca o banco padrão da conexão, sem reabrir a sessão.
pub(crate) fn use_database(conn: &mut Conn, database: &str) -> mysql::Result<()> {
    conn.query_drop(format!("USE `{}`", quote(database)))
}

#[cfg(test)]
mod tests {
    use super::{rows_sql, update_sql};

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
}
