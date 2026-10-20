//! Valores do MySQL como texto: escape de identificadores, tipos e células.

use mysql::consts::ColumnType;
use mysql::Value;

use super::super::ColumnKind;

/// Escapa um identificador para ir entre crases.
pub(crate) fn quote(identifier: &str) -> String {
    identifier.replace('`', "``")
}

/// Número ou texto, a partir do tipo que veio no result set.
pub(crate) fn kind_of(column_type: ColumnType) -> ColumnKind {
    if column_type.is_numeric_type() {
        ColumnKind::Number
    } else {
        ColumnKind::Text
    }
}

/// Formata um valor do MySQL como texto de célula.
pub(crate) fn cell(value: &Value) -> String {
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
    use mysql::consts::ColumnType;
    use mysql::Value;

    use super::super::super::ColumnKind;
    use super::{cell, kind_of};

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
