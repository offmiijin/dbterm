//! As consultas ao MySQL e a formatação dos valores que vêm dele.

use mysql::consts::ColumnType;
use mysql::prelude::Queryable;
use mysql::{Conn, Value};

use super::{ColumnKind, ROW_LIMIT, TableData};

pub(super) fn tables(conn: &mut Conn) -> mysql::Result<Vec<String>> {
    conn.query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' \
         ORDER BY table_name",
    )
}

pub(super) fn rows(conn: &mut Conn, table: &str) -> mysql::Result<TableData> {
    // Identificador não vai como parâmetro: escapamos com crase dobrada.
    let sql = format!(
        "SELECT * FROM `{}` LIMIT {ROW_LIMIT}",
        table.replace('`', "``")
    );
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
