//! Desenho da tela. Toda função aqui só lê o estado — a exceção é a rolagem do
//! `TableState` da grade, que o widget atualiza ao desenhar.
//!
//! O layout fica aqui; a grade de dados em `grid` e os pop-ups em `popup`. A app
//! também usa [`larguras`] para medir as colunas uma vez por mudança de dados.

mod connect;
mod grid;
/// Largura natural de cada coluna: quem chama é a app, uma vez por mudança de
/// dados, porque medir varre todas as linhas carregadas.
pub(crate) use grid::larguras;
mod popup;
mod quick;
#[cfg(test)]
mod tests;
mod text;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph};

use crate::app::{App, Focus, Mode, Screen, Status};
use crate::db::TableData;
use crate::theme;

const KEYS: &str = "q sair · h/l colunas · esc voltar/sair · j/k mover · enter abrir/colunas · r recarregar · g/G topo/fim";
/// Espaço que a mensagem de estado nunca perde para a linha de atalhos.
const MIN_STATUS: u16 = 32;

/// "DBTERM" na fonte ANSI Shadow do figlet; cada linha tem 57 colunas.
const BANNER: [&str; 6] = [
    "██████╗  ██████╗  ████████╗ ███████╗ ██████╗  ███╗   ███╗",
    "██╔══██╗ ██╔══██╗ ╚══██╔══╝ ██╔════╝ ██╔══██╗ ████╗ ████║",
    "██║  ██║ ██████╔╝    ██║    █████╗   ██████╔╝ ██╔████╔██║",
    "██║  ██║ ██╔══██╗    ██║    ██╔══╝   ██╔══██╗ ██║╚██╔╝██║",
    "██████╔╝ ██████╔╝    ██║    ███████╗ ██║  ██║ ██║ ╚═╝ ██║",
    "╚═════╝  ╚═════╝     ╚═╝    ╚══════╝ ╚═╝  ╚═╝ ╚═╝     ╚═╝",
];

/// A marca em letras grandes; em terminal estreito, só o nome.
fn banner(largura_tela: u16) -> Vec<Line<'static>> {
    if largura_tela as usize >= text::largura(BANNER[0]) {
        BANNER
            .iter()
            .map(|linha| Line::from(*linha).style(theme::accent()).centered())
            .collect()
    } else {
        vec![Line::from("dbterm").style(theme::accent()).centered()]
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Block::new().style(theme::base()), frame.area());
    match app.screen {
        Screen::Quick => quick::draw(frame, frame.area(), app),
        Screen::Connect => connect::draw(frame, frame.area(), app),
        Screen::Browse => draw_browse(frame, app),
    }
}

/// Tela de trabalho: lista de tabelas, grade e pop-ups.
fn draw_browse(frame: &mut Frame, app: &mut App) {
    let App {
        server,
        database,
        tables,
        current,
        data,
        focus,
        status,
        mode,
        edit,
        list_state,
        table_state,
        column,
        hscroll,
        larguras,
        end_reached,
        ..
    } = app;

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let [side, main] =
        Layout::horizontal([Constraint::Length(28), Constraint::Min(20)]).areas(body);

    draw_header(frame, header, server, database, current.as_deref());

    let titles: Vec<ListItem> = tables
        .iter()
        .map(|table| ListItem::new(table.as_str()))
        .collect();
    let list = List::new(titles)
        .block(panel(
            format!("tabelas ({})", tables.len()),
            *focus == Focus::Tables,
        ))
        .highlight_symbol("❯ ")
        .highlight_style(theme::selected());
    frame.render_stateful_widget(list, side, list_state);

    grid::draw_data(
        frame,
        main,
        data.as_ref(),
        *focus == Focus::Data,
        table_state,
        grid::Posicao {
            coluna: *column,
            rolagem: *hscroll,
            larguras,
        },
        mode,
    );

    draw_footer(frame, footer, status, mode, data, *end_reached);
    popup::draw_overlays(
        frame,
        frame.area(),
        mode,
        edit.as_ref(),
        data.as_ref(),
        current.as_deref(),
    );
}

fn draw_header(frame: &mut Frame, area: Rect, server: &str, database: &str, table: Option<&str>) {
    let server = if server.is_empty() {
        "conectando…"
    } else {
        server
    };
    let mut line = Line::from(vec![
        Span::styled(" dbterm ", theme::accent()),
        Span::styled("·", theme::muted()),
        Span::styled(format!(" {server} "), theme::text()),
        Span::styled("·", theme::muted()),
        Span::styled(format!(" schema: {database} "), theme::text()),
    ]);
    if let Some(table) = table {
        line.push_span(Span::styled("·", theme::muted()));
        line.push_span(Span::styled(format!(" tabela: {table} "), theme::text()));
    }
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_footer(
    frame: &mut Frame,
    area: Rect,
    status: &Status,
    mode: &Mode,
    data: &Option<TableData>,
    end_reached: bool,
) {
    let (text, style) = match status {
        Status::Connecting => (String::from("conectando…"), theme::warning()),
        Status::Loading(table) => (format!("carregando {table}…"), theme::warning()),
        // Quantas linhas estão na grade e se ainda há página para carregar.
        Status::Ready => match (data, end_reached) {
            (Some(data), false) => (
                format!("{} linhas · ↓ carrega mais", data.rows.len()),
                theme::muted(),
            ),
            (Some(data), true) => (format!("{} linhas", data.rows.len()), theme::muted()),
            (None, _) => (String::new(), theme::muted()),
        },
        Status::Notice(message) => (message.clone(), theme::warning()),
        Status::Failed(message) => (format!("erro: {message}"), theme::error()),
    };
    let keys = match mode {
        Mode::Rows => KEYS,
        Mode::Columns => "h/l coluna · enter editar · esc voltar",
        Mode::Edit => "enter continuar · esc cancelar",
        Mode::Confirm => "enter executar · esc cancelar",
        Mode::ConfirmAgain => "enter confirmo · esc cancelar",
        Mode::ConfirmQuit => "enter sair · esc cancelar",
    };

    let width = (keys.chars().count() as u16).min(area.width.saturating_sub(MIN_STATUS));
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);
    frame.render_widget(Paragraph::new(Line::from(text).style(style)), left);
    frame.render_widget(
        Paragraph::new(keys).style(theme::muted()).right_aligned(),
        right,
    );
}

/// Moldura de painel, usada pela grade e pela lista de tabelas.
pub(super) fn panel(title: String, focused: bool) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border(focused))
        .title(format!(" {title} "))
}
