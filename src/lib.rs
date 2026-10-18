//! dbterm: gerenciador de banco de dados via terminal.
//!
//! A biblioteca existe para o binário e os testes de integração (`tests/`)
//! compartilharem o mesmo código: `db` fala com o banco, `app` guarda o estado,
//! `ui` desenha.

pub mod app;
pub mod db;
pub mod ui;
