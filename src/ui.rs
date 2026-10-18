//! Desenho da tela. Toda função aqui só lê o estado.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, BorderType, Cell, HighlightSpacing, List, ListItem, Paragraph, Row, Table, TableState,
};

use crate::app::{App, Focus, Status};
use crate::db::{ROW_LIMIT, TableData};

const KEYS: &str = "q sair · tab foco · j/k mover · enter abrir · r recarregar · g/G topo/fim";
/// Largura máxima de uma coluna da grade.
const MAX_COL: usize = 28;
/// Espaço que a mensagem de estado nunca perde para a linha de atalhos.
const MIN_STATUS: u16 = 32;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let App {
        server,
        database,
        tables,
        current,
        data,
        focus,
        status,
        list_state,
        table_state,
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
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, side, list_state);

    draw_data(
        frame,
        main,
        data.as_ref(),
        *focus == Focus::Data,
        table_state,
    );

    draw_footer(frame, footer, status);
}

fn draw_header(frame: &mut Frame, area: Rect, server: &str, database: &str, table: Option<&str>) {
    let server = if server.is_empty() {
        "conectando…"
    } else {
        server
    };
    let mut line = Line::from(vec![
        " dbterm ".bold().cyan(),
        "·".dark_gray(),
        format!(" {server} ").into(),
        "·".dark_gray(),
        format!(" schema: {database} ").into(),
    ]);
    if let Some(table) = table {
        line.push_span("·".dark_gray());
        line.push_span(format!(" tabela: {table} "));
    }
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_data(
    frame: &mut Frame,
    area: Rect,
    data: Option<&TableData>,
    focused: bool,
    state: &mut TableState,
) {
    let Some(data) = data else {
        let hint = Paragraph::new("selecione uma tabela e pressione enter")
            .block(panel(String::from("dados"), focused))
            .style(Style::new().dark_gray());
        frame.render_widget(hint, area);
        return;
    };

    let title = format!("dados · {} linhas", data.rows.len());
    let header = Row::new(
        data.columns
            .iter()
            .map(|name| Cell::new(name.as_str()))
            .collect::<Vec<_>>(),
    )
    .style(Style::new().bold());

    let rows: Vec<Row> = data
        .rows
        .iter()
        .map(|row| {
            Row::new(
                row.iter()
                    .map(|value| {
                        let cell = Cell::new(value.as_str());
                        // NULL é a ausência de valor: vale a pena destacar.
                        if value == "NULL" {
                            cell.style(Style::new().dark_gray())
                        } else {
                            cell
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    let table = Table::new(rows, column_widths(data))
        .header(header)
        .block(panel(title, focused))
        .column_spacing(1)
        .highlight_symbol("❯ ")
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, state);
}

fn draw_footer(frame: &mut Frame, area: Rect, status: &Status) {
    let (text, style) = match status {
        Status::Connecting => (String::from("conectando…"), Style::new().yellow()),
        Status::Loading(table) => (format!("carregando {table}…"), Style::new().yellow()),
        Status::Ready => (format!("LIMIT {ROW_LIMIT}"), Style::new().dark_gray()),
        Status::Failed(message) => (format!("erro: {message}"), Style::new().red()),
    };

    let width = (KEYS.chars().count() as u16).min(area.width.saturating_sub(MIN_STATUS));
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);
    frame.render_widget(Paragraph::new(Line::from(text).style(style)), left);
    frame.render_widget(
        Paragraph::new(KEYS)
            .style(Style::new().dark_gray())
            .right_aligned(),
        right,
    );
}

fn panel(title: String, focused: bool) -> Block<'static> {
    let border = if focused {
        Style::new().cyan()
    } else {
        Style::new().dark_gray()
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(format!(" {title} "))
}

/// Largura de cada coluna: o maior entre cabeçalho e conteúdo, com teto.
///
/// Conta caracteres, não largura de exibição — CJK e emoji ficam com a coluna
/// um pouco apertada. Trocar por `unicode-width` quando incomodar.
fn column_widths(data: &TableData) -> Vec<Constraint> {
    data.columns
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let widest = data.rows.iter().fold(name.chars().count(), |widest, row| {
                row.get(index).map_or(widest, |value| {
                    widest.max(value.chars().count().min(MAX_COL))
                })
            });
            Constraint::Length(widest.min(MAX_COL) as u16)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::app::App;

    fn screen(app: &mut App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
        terminal.draw(|frame| draw(frame, app)).expect("desenho");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn app_com_dados() -> App {
        let mut app = App::new("mysql://user@localhost/app");
        app.server = String::from("MySQL 8.0.35");
        app.database = String::from("app");
        app.tables = vec![String::from("users")];
        app.list_state.select(Some(0));
        app.current = Some(String::from("users"));
        app.data = Some(TableData {
            columns: vec![String::from("id"), String::from("name")],
            rows: vec![
                vec![String::from("1"), String::from("ana")],
                vec![String::from("2"), String::from("NULL")],
            ],
        });
        app.status = Status::Ready;
        app
    }

    #[test]
    fn mostra_conexao_tabelas_e_celulas() {
        let screen = screen(&mut app_com_dados());
        assert!(screen.contains("MySQL 8.0.35"), "{screen}");
        assert!(screen.contains("users"), "{screen}");
        assert!(screen.contains("ana"), "{screen}");
        assert!(screen.contains("NULL"), "{screen}");
        assert!(screen.contains("2 linhas"), "{screen}");
    }

    #[test]
    fn sem_dados_mostra_dica() {
        let screen = screen(&mut App::new("mysql://user@localhost/app"));
        assert!(screen.contains("selecione uma tabela"), "{screen}");
    }

    #[test]
    fn erro_aparece_no_rodape() {
        let mut app = app_com_dados();
        app.status = Status::Failed(String::from("sem permissão"));
        let screen = screen(&mut app);
        assert!(screen.contains("erro: sem permissão"), "{screen}");
    }
}
