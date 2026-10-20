//! Desenho da tela. Toda função aqui só lê o estado — a exceção é a rolagem do
//! `TableState` da grade, que o widget atualiza ao desenhar.
//!
//! O layout fica aqui; a grade de dados em `grid` e os pop-ups em `popup`. A
//! medida das colunas mora em `crate::view`, usada pela app e pela grade.

mod connect;
mod grid;
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
use crate::history::Saved;
use crate::theme;
use crate::view::largura;

const KEYS: &str = "q sair · c/b/t nível · esc subir · j/k mover · enter abrir · h/l colunas · r recarregar · g/G topo/fim";
/// Na cascata o `e` entra como ação de edição do nível ativo.
const KEYS_WITH_EDIT: &str = "q sair · c/b/t nível · esc subir · e editar · j/k mover · enter abrir · r recarregar · g/G topo/fim";
/// Teclas dos pop-ups de edição (conexão, banco e tabela).
const POPUP_KEYS: &str = "ctrl+s salvar · esc cancelar";
/// Teclas da lista de opções aberta sobre um pop-up de edição.
const PICKER_KEYS: &str = "j/k mover · enter selecionar · esc voltar";
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
    if largura_tela as usize >= largura(BANNER[0]) {
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

/// Tela de trabalho: chips dos níveis, lista do nível ativo, grade e pop-ups.
fn draw_browse(frame: &mut Frame, app: &mut App) {
    let App {
        server,
        database,
        databases,
        tables,
        current,
        data,
        focus,
        status,
        mode,
        edit,
        connections_state,
        databases_state,
        list_state,
        table_state,
        column,
        hscroll,
        larguras,
        end_reached,
        connection_edit,
        database_edit,
        table_edit,
        commands,
        history,
        ..
    } = app;

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let [side, main] =
        Layout::horizontal([Constraint::Length(34), Constraint::Min(20)]).areas(body);

    draw_header(frame, header, server, database, current.as_deref());

    // Nos registros, o nível de contexto continua sendo o das tabelas.
    let ativo = match *focus {
        Focus::Data => Focus::Tables,
        outro => outro,
    };
    let [chips, lista] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(side);
    let contagens = [history.all().len(), databases.len(), tables.len()];
    // Os chips ficam centralizados sobre o painel logo abaixo.
    frame.render_widget(
        Paragraph::new(chips_line(ativo, contagens)).centered(),
        chips,
    );

    let (itens, state) = match ativo {
        Focus::Connections => (
            history.all().iter().map(conexao_item).collect::<Vec<_>>(),
            connections_state,
        ),
        Focus::Databases => (
            databases
                .iter()
                .map(database_item)
                .collect::<Vec<_>>(),
            databases_state,
        ),
        _ => (
            tables.iter().map(tabela_item).collect::<Vec<_>>(),
            list_state,
        ),
    };
    let list = List::new(itens)
        .block(panel(String::new(), *focus == ativo))
        .highlight_symbol("❯ ")
        .highlight_style(theme::selected());
    frame.render_stateful_widget(list, lista, state);

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

    // A lista aberta, em qualquer pop-up, fica com as teclas do rodapé.
    let lista_aberta = database_edit
        .as_ref()
        .is_some_and(|edit| edit.picker.is_some())
        || table_edit
            .as_ref()
            .is_some_and(|edit| edit.picker.is_some());
    let teclas = if commands.is_some() {
        popup::commands_keys()
    } else if lista_aberta {
        PICKER_KEYS
    } else if let Some(edit) = table_edit.as_ref() {
        popup::table_keys(edit)
    } else if connection_edit.is_some() || database_edit.is_some() {
        POPUP_KEYS
    } else {
        keys(*mode, *focus)
    };
    draw_footer(frame, footer, status, teclas, data, *end_reached);
    popup::draw_overlays(
        frame,
        frame.area(),
        mode,
        edit.as_ref(),
        data.as_ref(),
        current.as_deref(),
    );
    popup::draw_connection_edit(frame, frame.area(), connection_edit.as_ref());
    popup::draw_database_edit(frame, frame.area(), database_edit.as_ref());
    popup::draw_table_edit(frame, frame.area(), table_edit.as_ref());
    popup::draw_commands(frame, frame.area(), commands.as_ref(), tables);
}

fn draw_header(frame: &mut Frame, area: Rect, server: &str, database: &str, table: Option<&str>) {
    let server = if server.is_empty() {
        "conectando…"
    } else {
        server
    };
    let database = if database.is_empty() {
        "(nenhum)"
    } else {
        database
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
    keys: &str,
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
    let width = (keys.chars().count() as u16).min(area.width.saturating_sub(MIN_STATUS));
    let [left, right] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)]).areas(area);
    frame.render_widget(Paragraph::new(Line::from(text).style(style)), left);
    frame.render_widget(
        Paragraph::new(keys).style(theme::muted()).right_aligned(),
        right,
    );
}

/// Atalhos mostrados no rodapé: o modo decide o texto e, na cascata, o `e`
/// entra como ação de edição do nível ativo.
fn keys(mode: Mode, focus: Focus) -> &'static str {
    match mode {
        Mode::Rows if matches!(focus, Focus::Connections | Focus::Databases | Focus::Tables) => {
            KEYS_WITH_EDIT
        }
        Mode::Rows => KEYS,
        Mode::Columns => "h/l coluna · enter editar · esc voltar",
        Mode::Edit => "enter continuar · esc cancelar",
        Mode::Confirm => "enter executar · esc cancelar",
        Mode::ConfirmAgain => "enter confirmo · esc cancelar",
        Mode::ConfirmQuit => "enter sair · esc cancelar",
    }
}

/// Ícones Nerd Font dos níveis da cascata.
const ICONE_CONEXAO: char = '\u{f1e6}'; // plug
const ICONE_BANCO: char = '\u{f1c0}'; // database
const ICONE_TABELA: char = '\u{f0ce}'; // table

/// Linha de chips dos três níveis, sempre na ordem conexões → bancos →
/// tabelas. O nível ativo mostra ícone, nome e contagem; os outros, a letra
/// do atalho e o ícone em destaque.
fn chips_line(ativo: Focus, contagens: [usize; 3]) -> Line<'static> {
    const NIVEIS: [(Focus, &str, &str, char); 3] = [
        (Focus::Connections, "c", "conexões", ICONE_CONEXAO),
        (Focus::Databases, "b", "bancos", ICONE_BANCO),
        (Focus::Tables, "t", "tabelas", ICONE_TABELA),
    ];
    let mut spans = Vec::new();
    for (indice, (nivel, letra, nome, icone)) in NIVEIS.iter().enumerate() {
        if indice > 0 {
            spans.push(Span::styled(" ", theme::muted()));
        }
        if *nivel == ativo {
            // Mesma cor da marca no topo: o nível ativo inteiro fica roxo.
            spans.push(Span::styled(
                format!("{} {nome} ({})", icone, contagens[indice]),
                theme::accent(),
            ));
        } else {
            spans.push(Span::styled("[", theme::muted()));
            spans.push(Span::styled(format!("{icone} "), theme::accent()));
            spans.push(Span::styled(*letra, theme::accent()));
            spans.push(Span::styled("]", theme::muted()));
        }
    }
    Line::from(spans)
}

/// Uma conexão salva como item da lista: ícone, endereço e schema, sem senha.
fn conexao_item(saved: &Saved) -> ListItem<'static> {
    let mut spans = vec![
        Span::styled(format!("{ICONE_CONEXAO} "), theme::accent()),
        Span::styled(format!("{}:{}", saved.host, saved.port), theme::text()),
    ];
    if !saved.database.is_empty() {
        spans.push(Span::styled(" ", theme::muted()));
        spans.push(Span::styled(saved.database.clone(), theme::muted()));
    }
    ListItem::new(Line::from(spans))
}

/// Um banco como item da lista, com o ícone de banco.
fn database_item(database: &String) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(format!("{ICONE_BANCO} "), theme::accent()),
        Span::styled(database.clone(), theme::text()),
    ]))
}

/// Uma tabela como item da lista, com o ícone de tabela.
fn tabela_item(table: &String) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(format!("{ICONE_TABELA} "), theme::accent()),
        Span::styled(table.clone(), theme::text()),
    ]))
}

/// Moldura de painel, usada pela grade e pela lista de tabelas.
pub(super) fn panel(title: String, focused: bool) -> Block<'static> {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border(focused));
    if title.is_empty() {
        block
    } else {
        block.title(format!(" {title} "))
    }
}
