//! Pop-ups de edição e das duas confirmações, com o preview do SQL.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph};

use super::text::largura;
use crate::app::{Edit, Mode};
use crate::db::{SqlPart, TableData, update_preview};
use crate::theme;

pub(super) fn draw_overlays(
    frame: &mut Frame,
    area: Rect,
    mode: &Mode,
    edit: Option<&Edit>,
    data: Option<&TableData>,
    table: Option<&str>,
) {
    let (Some(edit), Some(data)) = (edit, data) else {
        return;
    };
    let Some(column) = data.columns.get(edit.column) else {
        return;
    };
    let table = table.unwrap_or("?");

    match mode {
        Mode::Edit => {
            let regua = popup_rule(area);
            let mut valor = format!("{}▏", edit.input);
            // O `Paragraph` só pinta o estilo da linha sobre os caracteres
            // escritos; para o fundo do campo cobrir a largura, preenchemos.
            let comprimento = popup_content_width(area);
            if largura(&valor) < comprimento {
                valor.push_str(&" ".repeat(comprimento - largura(&valor)));
            }
            let lines = vec![
                Line::from(format!("TABLE {table} COLUMN {column}")).style(theme::muted()),
                Line::from(regua.clone()).style(theme::muted()),
                Line::from(valor).style(theme::field()),
                Line::from(regua).style(theme::muted()),
                linha_acoes("continuar"),
            ];
            // O ícone (Nerd Font) acompanha o título do pop-up.
            popup(frame, area, "\u{f040} Editar", lines);
        }
        Mode::Confirm => {
            let mut lines = bloco_query(table, column, data, edit, area);
            lines.push(linha_acoes("executar"));
            popup(frame, area, "\u{f04b} Execução", lines);
        }
        Mode::ConfirmAgain => {
            let mut lines = bloco_query(table, column, data, edit, area);
            // O aviso é texto padrão, menos a declaração, que sai como na query.
            lines.push(Line::from(vec![
                Span::styled("1 registro será alterado na execução do ", theme::text()),
                Span::styled("UPDATE", theme::accent()),
                Span::styled(".", theme::text()),
            ]));
            lines.push(Line::from("ESSA AÇÃO É IRREVERSÍVEL.").style(theme::danger()));
            lines.push(Line::from(""));
            lines.push(Line::from("Você confirma?"));
            lines.push(Line::from(popup_rule(area)).style(theme::muted()));
            lines.push(linha_acoes("confirmo"));
            // O ícone (Nerd Font) acompanha o título do pop-up.
            popup(frame, area, "\u{f00c} Confirmar", lines);
        }
        Mode::Rows | Mode::Columns => {}
    }
}

/// Bloco que os pop-ups de confirmação compartilham: a identificação do alvo,
/// a régua, as linhas da query e a régua de fecho.
fn bloco_query(
    table: &str,
    column: &str,
    data: &TableData,
    edit: &Edit,
    area: Rect,
) -> Vec<Line<'static>> {
    let valor = if edit.input.is_empty() {
        None
    } else {
        Some(edit.input.as_str())
    };
    let mut lines = vec![
        Line::from(format!("TABLE {table} COLUMN {column}")).style(theme::muted()),
        Line::from(popup_rule(area)).style(theme::muted()),
    ];
    lines.extend(
        update_preview(table, data, edit.row, edit.column, valor)
            .iter()
            .map(|linha| linha_query(linha)),
    );
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines
}

/// Linha de ação dos pop-ups: teclas em accent, verbo em texto padrão.
fn linha_acoes(verbo: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled("enter", theme::accent()),
        Span::styled(format!(" {verbo} · "), theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" cancelar", theme::text()),
    ])
}

/// Linha da query: cada pedaço na cor do seu papel.
fn linha_query(pedacos: &[(String, SqlPart)]) -> Line<'static> {
    Line::from(
        pedacos
            .iter()
            .map(|(texto, papel)| Span::styled(texto.clone(), parte_estilo(*papel)))
            .collect::<Vec<_>>(),
    )
}

/// Cor de cada papel da query: declaração como o título, texto e número como
/// literais da linguagem, `NULL` como o resto do que é secundário.
fn parte_estilo(papel: SqlPart) -> Style {
    match papel {
        SqlPart::Declaration => theme::accent(),
        SqlPart::Plain => theme::text(),
        SqlPart::Text => theme::string(),
        SqlPart::Number => theme::number(),
        SqlPart::Null => theme::muted(),
    }
}

/// Largura útil do popup (descontando as bordas laterais).
fn popup_width(area: Rect) -> u16 {
    area.width.saturating_sub(4).min(64)
}

/// Régua horizontal do popup, do tamanho da área interna (após o respiro).
fn popup_rule(area: Rect) -> String {
    "─".repeat(popup_content_width(area))
}

/// Largura da área interna do popup (bordas e respiro da esquerda descontados).
fn popup_content_width(area: Rect) -> usize {
    popup_width(area).saturating_sub(3) as usize
}

/// Caixa centralizada que cobre o conteúdo por baixo.
fn popup(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    let width = popup_width(area);
    let height = (lines.len() as u16 + 2).min(area.height);
    if width < 10 || height < 3 {
        return;
    }
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    let area = Rect::new(x, y, width, height);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::accent())
        .padding(Padding::left(1))
        .style(theme::base())
        .title(format!(" {title} "));
    frame.render_widget(
        Paragraph::new(lines).block(block).style(theme::base()),
        area,
    );
}
