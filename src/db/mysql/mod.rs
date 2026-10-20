//! As consultas ao MySQL e a formatação dos valores que vêm dele.
//!
//! O worker chama as funções daqui; cada responsabilidade mora num submódulo:
//! `catalog` lê o `information_schema`, `ddl` monta os `ALTER`s, `rows` lê e
//! grava linhas e `values` converte tipos e escapa identificadores.

mod catalog;
mod ddl;
mod rows;
mod values;

pub(crate) use catalog::{database_info, databases, table_info, tables};
pub(crate) use ddl::{alter_database, alter_table};
pub(crate) use rows::{rows, update, use_database};
