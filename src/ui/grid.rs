//! A grade de dados: a `Table` com a divisória entre as colunas.
//!
//! Cada coluna de dados é seguida por uma célula de divisória, então a `Table`
//! vê as colunas de dados nos índices 0, 2, 4, ... e as divisórias em 1, 3, 5, ...
//! O resto do programa fala só de índices de dados; a dobra fica em
//! [`table_column`].

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Cell, HighlightSpacing, Paragraph, Row, Table, TableState};

use super::panel;
use super::text::{fit, largura};
use crate::app::Mode;
use crate::db::TableData;
use crate::theme;

/// Largura máxima de uma coluna da grade.
const MAX_COL: usize = 28;
/// Respiro de cada lado da linha divisória entre colunas.
const GAP: u16 = 2;
/// O traço que separa duas colunas.
pub(super) const RULE: &str = "│";
/// Largura de [`RULE`] em colunas do terminal.
const RULE_WIDTH: u16 = 1;
/// De quanto em quanto o índice do widget pula para chegar na próxima coluna.
const RULE_STRIDE: usize = 2;

/// Índice da coluna de dados `index` no widget `Table`.
fn table_column(index: usize) -> usize {
    index * RULE_STRIDE
}

pub(super) fn draw_data(
    frame: &mut Frame,
    area: Rect,
    data: Option<&TableData>,
    focused: bool,
    state: &mut TableState,
    column: Option<usize>,
    mode: &Mode,
) {
    let Some(data) = data else {
        let hint = Paragraph::new("selecione uma tabela e pressione enter")
            .block(panel(String::from("dados"), focused))
            .style(theme::muted());
        frame.render_widget(hint, area);
        return;
    };

    let coluna = column.and_then(|index| data.columns.get(index));
    let title = match (mode, coluna) {
        (Mode::Columns | Mode::Edit | Mode::Confirm | Mode::ConfirmAgain, Some(nome)) => {
            format!("dados · {} linhas · col: {nome}", data.rows.len())
        }
        _ => format!("dados · {} linhas", data.rows.len()),
    };
    let colunas = column_widths(data, area.width);
    let header = Row::new(header_cells(data, &colunas)).style(theme::header());
    let rows: Vec<Row> = data
        .rows
        .iter()
        .map(|row| Row::new(cells(row, &colunas)))
        .collect();

    let table = Table::new(rows, constraints(&colunas))
        .header(header)
        .block(panel(title, focused))
        .column_spacing(0)
        .highlight_symbol("❯ ")
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(theme::selected())
        .cell_highlight_style(theme::cell_selected());
    // O widget vê uma divisória entre cada par de colunas, então o índice de
    // dados é dobrado aqui: o estado guarda só a coluna de dados.
    state.select_column(column.map(table_column));
    frame.render_stateful_widget(table, area, state);
}

/// A linha vertical que separa duas colunas.
fn rule() -> Cell<'static> {
    Cell::new(RULE).style(theme::muted())
}

/// Cabeçalho com a divisória entre as colunas.
fn header_cells<'a>(data: &'a TableData, colunas: &[usize]) -> Vec<Cell<'a>> {
    let mut cells = Vec::with_capacity(data.columns.len() * 2);
    for (index, name) in data.columns.iter().enumerate() {
        if index > 0 {
            cells.push(rule());
        }
        cells.push(
            match fit(name, colunas.get(index).copied().unwrap_or(MAX_COL)) {
                Some(cortado) => Cell::new(cortado),
                None => Cell::new(name.as_str()),
            },
        );
    }
    cells
}

/// Linha de dados com a divisória entre as colunas; `NULL` sai em cinza.
fn cells<'a>(row: &'a [String], colunas: &[usize]) -> Vec<Cell<'a>> {
    let mut cells = Vec::with_capacity(row.len() * 2);
    for (index, value) in row.iter().enumerate() {
        if index > 0 {
            cells.push(rule());
        }
        let cell = match fit(value, colunas.get(index).copied().unwrap_or(MAX_COL)) {
            Some(cortado) => Cell::new(cortado),
            None => Cell::new(value.as_str()),
        };
        cells.push(if value == "NULL" {
            cell.style(theme::muted())
        } else {
            cell
        });
    }
    cells
}

/// Largura desejada de cada coluna, já cabendo em `area` (sem as divisórias).
///
/// O `Table` encolhe todas as colunas por igual quando a soma passa da área, e
/// aí o cabeçalho vira `n`, `e`, `a`. Então encolhemos aqui, sempre na coluna
/// mais larga, sem descer abaixo do nome do cabeçalho.
fn column_widths(data: &TableData, area: u16) -> Vec<usize> {
    let mut desejadas: Vec<usize> = Vec::with_capacity(data.columns.len());
    let mut minimas: Vec<usize> = Vec::with_capacity(data.columns.len());
    for (index, name) in data.columns.iter().enumerate() {
        let cabecalho = largura(name);
        let conteudo = data.rows.iter().fold(cabecalho, |widest, row| {
            row.get(index)
                .map_or(widest, |value| widest.max(largura(value).min(MAX_COL)))
        });
        desejadas.push(conteudo);
        minimas.push(cabecalho);
    }

    let divisao = if data.columns.len() > 1 {
        data.columns.len() - 1
    } else {
        0
    };
    let ornamento = 2 /* bordas */
        + 2 /* "❯ " */
        + data.columns.len() * GAP as usize
        + divisao * RULE_WIDTH as usize;
    let orcamento = (area as usize).saturating_sub(ornamento);

    while desejadas.iter().sum::<usize>() > orcamento {
        // A coluna mais larga que ainda pode encolher.
        let Some(index) = (0..desejadas.len())
            .filter(|&index| desejadas[index] > minimas[index])
            .max_by_key(|&index| desejadas[index])
        else {
            break;
        };
        desejadas[index] -= 1;
    }

    desejadas
}

/// Constraints do `Table`: coluna (com o respiro antes da divisória), divisória,
/// coluna, ... A divisória fica colada no início da coluna seguinte.
fn constraints(colunas: &[usize]) -> Vec<Constraint> {
    let mut widths = Vec::with_capacity(colunas.len() * 2);
    for (index, largura) in colunas.iter().enumerate() {
        if index > 0 {
            widths.push(Constraint::Length(RULE_WIDTH));
        }
        widths.push(Constraint::Length(*largura as u16 + GAP));
    }
    widths
}
