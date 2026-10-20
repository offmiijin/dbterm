//! DDL: `ALTER DATABASE` e `ALTER TABLE` montados só com o que mudou.
//!
//! Identificadores não aceitam placeholder e entram no texto do DDL: nomes de
//! tabela/banco vão escapados com crase e charset/collation/engine passam por
//! [`identificador`]. O comentário vai como parâmetro `?`.

use mysql::prelude::Queryable;
use mysql::{Conn, Value};

use super::super::{DatabaseAlter, TableAlter, identificador};
use super::values::quote;

pub(crate) fn alter_database(
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

/// Aplica na tabela as opções que mudaram e devolve o nome final: o novo, no
/// caso de rename.
pub(crate) fn alter_table(
    conn: &mut Conn,
    table: &str,
    alter: &TableAlter,
) -> Result<String, String> {
    let Some((sql, params)) = alter_table_sql(table, alter)? else {
        return Ok(table.to_owned());
    };
    let params: Vec<Value> = params.into_iter().map(Value::from).collect();
    conn.exec_drop(sql, params).map_err(|err| err.to_string())?;
    Ok(alter.rename.clone().unwrap_or_else(|| table.to_owned()))
}

/// Monta o `ALTER TABLE` só com o que mudou; sem mudança, não há query.
///
/// Identificadores (nome novo, engine, formato, charset, collation) não aceitam
/// placeholder e entram no texto do DDL: o nome novo vai escapado com crase e o
/// resto passa por [`identificador`]. O comentário vai como parâmetro `?`.
fn alter_table_sql(
    table: &str,
    alter: &TableAlter,
) -> Result<Option<(String, Vec<String>)>, String> {
    let mut opcoes = Vec::new();
    let mut params = Vec::new();
    if let Some(rename) = &alter.rename {
        let rename = rename.trim();
        if rename.is_empty() {
            return Err(String::from("informe o nome da tabela"));
        }
        opcoes.push(format!("RENAME TO `{}`", quote(rename)));
    }
    if let Some(comment) = &alter.comment {
        opcoes.push(String::from("COMMENT = ?"));
        params.push(comment.clone());
    }
    if let Some(engine) = &alter.engine {
        opcoes.push(format!("ENGINE = {}", nome(engine, "engine")?));
    }
    if let Some(row_format) = &alter.row_format {
        opcoes.push(format!(
            "ROW_FORMAT = {}",
            nome(row_format, "formato de linha")?
        ));
    }
    if let Some(auto_increment) = alter.auto_increment {
        opcoes.push(format!("AUTO_INCREMENT = {auto_increment}"));
    }
    if let Some(charset) = &alter.charset {
        opcoes.push(format!("CHARACTER SET = {}", nome(charset, "charset")?));
    }
    if let Some(collation) = &alter.collation {
        opcoes.push(format!("COLLATE = {}", nome(collation, "collation")?));
    }
    if opcoes.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        format!("ALTER TABLE `{}` {}", quote(table), opcoes.join(", ")),
        params,
    )))
}

#[cfg(test)]
mod tests {
    use super::super::super::{DatabaseAlter, TableAlter};
    use super::{alter_database_sql, alter_table_sql};

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
    fn monta_alter_table_com_as_opcoes_alteradas() {
        let alter = TableAlter {
            rename: Some(String::from("clientes")),
            comment: Some(String::from("cadastro")),
            engine: Some(String::from("InnoDB")),
            row_format: Some(String::from("Dynamic")),
            auto_increment: Some(2048),
            charset: Some(String::from("utf8mb4")),
            collation: Some(String::from("utf8mb4_bin")),
        };

        let (sql, params) = alter_table_sql("users", &alter).unwrap().unwrap();

        assert_eq!(
            sql,
            "ALTER TABLE `users` RENAME TO `clientes`, COMMENT = ?, ENGINE = InnoDB, \
             ROW_FORMAT = Dynamic, AUTO_INCREMENT = 2048, CHARACTER SET = utf8mb4, \
             COLLATE = utf8mb4_bin"
        );
        assert_eq!(params, ["cadastro"], "o comentário vai como parâmetro");
    }

    #[test]
    fn alter_table_sem_mudanca_nao_vira_query() {
        assert!(
            alter_table_sql("users", &TableAlter::default())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn alter_table_escapa_crases_dos_nomes() {
        let alter = TableAlter {
            rename: Some(String::from("we`ird")),
            ..TableAlter::default()
        };
        let (sql, params) = alter_table_sql("outra`tabela", &alter).unwrap().unwrap();
        assert_eq!(sql, "ALTER TABLE `outra``tabela` RENAME TO `we``ird`");
        assert!(params.is_empty());
    }

    #[test]
    fn alter_table_recusa_rename_vazio_e_identificador_estranho() {
        let vazio = TableAlter {
            rename: Some(String::from("  ")),
            ..TableAlter::default()
        };
        assert!(alter_table_sql("users", &vazio).is_err());

        let engine = TableAlter {
            engine: Some(String::from("InnoDB; DROP TABLE users")),
            ..TableAlter::default()
        };
        assert!(alter_table_sql("users", &engine).is_err());
    }
}
