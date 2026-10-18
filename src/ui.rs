//! Desenho da tela. Toda função aqui só lê o estado.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Cell, Clear, HighlightSpacing, List, ListItem, Paragraph, Row, Table,
    TableState,
};

use crate::app::{App, Edit, Focus, Mode, Status};
use crate::db::{ROW_LIMIT, TableData};
use crate::theme;

const KEYS: &str =
    "q sair · tab foco · j/k mover · enter abrir/colunas · r recarregar · g/G topo/fim";
/// Largura máxima de uma coluna da grade.
const MAX_COL: usize = 28;
/// Espaço que a mensagem de estado nunca perde para a linha de atalhos.
const MIN_STATUS: u16 = 32;
/// Respiro de cada lado da linha divisória entre colunas.
const GAP: u16 = 2;
/// O traço que separa duas colunas.
const RULE: &str = "│";
/// Largura de [`RULE`] em colunas do terminal.
const RULE_WIDTH: u16 = 1;
/// Cada coluna de dados é seguida por uma célula de divisória, então a `Table`
/// vê as colunas de dados nos índices 0, 2, 4, ... e as divisórias em 1, 3, 5, ...
const RULE_STRIDE: usize = 2;

/// Índice da coluna de dados `index` no widget `Table`.
pub fn table_column(index: usize) -> usize {
    index * RULE_STRIDE
}

/// Índice da coluna de dados a partir do índice de coluna do widget `Table`.
pub fn data_column(table_column: usize) -> usize {
    table_column / RULE_STRIDE
}

pub fn draw(frame: &mut Frame, app: &mut App) {
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
        ..
    } = app;

    frame.render_widget(Block::new().style(theme::base()), frame.area());

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

    draw_data(
        frame,
        main,
        data.as_ref(),
        *focus == Focus::Data,
        table_state,
        mode,
    );

    draw_footer(frame, footer, status, mode);
    draw_overlays(
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

fn draw_data(
    frame: &mut Frame,
    area: Rect,
    data: Option<&TableData>,
    focused: bool,
    state: &mut TableState,
    mode: &Mode,
) {
    let Some(data) = data else {
        let hint = Paragraph::new("selecione uma tabela e pressione enter")
            .block(panel(String::from("dados"), focused))
            .style(theme::muted());
        frame.render_widget(hint, area);
        return;
    };

    let coluna = state
        .selected_column()
        .map(data_column)
        .and_then(|index| data.columns.get(index));
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

fn draw_footer(frame: &mut Frame, area: Rect, status: &Status, mode: &Mode) {
    let (text, style) = match status {
        Status::Connecting => (String::from("conectando…"), theme::warning()),
        Status::Loading(table) => (format!("carregando {table}…"), theme::warning()),
        Status::Ready => (format!("LIMIT {ROW_LIMIT}"), theme::muted()),
        Status::Notice(message) => (message.clone(), theme::warning()),
        Status::Failed(message) => (format!("erro: {message}"), theme::error()),
    };
    let keys = match mode {
        Mode::Rows => KEYS,
        Mode::Columns => "h/l coluna · enter editar · esc voltar",
        Mode::Edit => "enter continuar · esc cancelar",
        Mode::Confirm => "enter sim · esc voltar",
        Mode::ConfirmAgain => "enter gravar · esc voltar",
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

/// Popups de edição e das duas confirmações.
fn draw_overlays(
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
            let lines = vec![
                Line::from(format!("{table}.{column}")).style(theme::muted()),
                Line::from(format!("{}▏", edit.input)).style(theme::text()),
                Line::from("enter continuar · esc cancelar").style(theme::muted()),
            ];
            popup(frame, area, "editar", lines);
        }
        Mode::Confirm => {
            let novo = if edit.input.is_empty() {
                "NULL"
            } else {
                edit.input.as_str()
            };
            let lines = vec![
                Line::from(format!("{table}.{column}")).style(theme::muted()),
                Line::from(format!("De: {}", edit.original)),
                Line::from(format!("Para: {novo}")),
                Line::from("Deseja realmente alterar este registro?"),
                Line::from("enter sim · esc voltar").style(theme::muted()),
            ];
            popup(frame, area, "alterar registro", lines);
        }
        Mode::ConfirmAgain => {
            let lines = vec![
                Line::from("Esta alteração será gravada no banco."),
                Line::from(""),
                Line::from("Tem certeza?"),
                Line::from("enter gravar · esc voltar").style(theme::muted()),
            ];
            popup(frame, area, "confirmar", lines);
        }
        Mode::Rows | Mode::Columns => {}
    }
}

/// Caixa centralizada que cobre o conteúdo por baixo.
fn popup(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    let width = area.width.saturating_sub(4).min(64);
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
        .style(theme::base())
        .title(format!(" {title} "));
    frame.render_widget(
        Paragraph::new(lines).block(block).style(theme::base()),
        area,
    );
}

fn panel(title: String, focused: bool) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border(focused))
        .title(format!(" {title} "))
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

/// `Some(texto)` quando o valor não cabe na coluna: corta e marca com `…`.
fn fit(value: &str, limite: usize) -> Option<String> {
    if largura(value) <= limite {
        return None;
    }
    let mut cortado = String::new();
    for caractere in value.chars() {
        cortado.push(caractere);
        if largura(&cortado) + 1 > limite {
            cortado.pop();
            break;
        }
    }
    cortado.push('…');
    Some(cortado)
}

/// Largura de exibição, pela mesma conta que o `ratatui` usa ao desenhar —
/// senão uma coluna com kanji fica mais estreita que o desenhado.
fn largura(texto: &str) -> usize {
    Span::raw(texto).width()
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::app::App;

    fn screen(app: &mut App) -> String {
        screen_at(app, 64)
    }

    fn screen_at(app: &mut App, largura: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(largura, 12)).expect("terminal de teste");
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
            primary_key: vec![0],
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

    fn app_largo() -> App {
        let mut app = App::new("mysql://user@localhost/app");
        app.tables = vec![String::from("users")];
        app.list_state.select(Some(0));
        app.current = Some(String::from("users"));
        app.data = Some(TableData {
            columns: [
                "id",
                "name",
                "email",
                "active",
                "created_at",
                "last_login",
                "notes",
                "meta",
            ]
            .iter()
            .map(|nome| nome.to_string())
            .collect(),
            rows: vec![vec![
                String::from("1"),
                String::from("Ana Prado"),
                String::from("ana@exemplo.com"),
                String::from("1"),
                String::from("2024-01-05 09:12:00"),
                String::from("2026-10-17 08:00:00"),
                String::from("cliente desde o lançamento"),
                String::from("{\"plano\": \"pro\"}"),
            ]],
            primary_key: vec![0],
        });
        app.status = Status::Ready;
        app
    }

    #[test]
    fn colunas_tem_traco_divisorio() {
        let screen = screen(&mut app_com_dados());
        assert!(screen.contains('│'), "{screen}");
        assert!(screen.contains("id"), "{screen}");
        assert!(screen.contains("name"), "{screen}");
    }

    #[test]
    fn cabecalho_nao_e_espremido_em_grade_estreita() {
        // Sem o cálculo de largura, o `Table` encolhe tudo e o cabeçalho vira
        // "c", "l", "n" — todas as colunas precisam aparecer por inteiro.
        let screen = screen_at(&mut app_largo(), 120);
        for nome in [
            "id",
            "name",
            "email",
            "active",
            "created_at",
            "last_login",
            "notes",
            "meta",
        ] {
            assert!(screen.contains(nome), "faltou {nome}: {screen}");
        }
    }

    #[test]
    fn valor_que_nao_cabe_termina_em_reticencias() {
        let mut app = app_com_dados();
        app.data = Some(TableData {
            columns: vec![String::from("id"), String::from("name")],
            rows: vec![vec![
                String::from("1"),
                String::from("um valor bem comprido que nao cabe na coluna"),
            ]],
            primary_key: vec![0],
        });
        let screen = screen(&mut app);
        assert!(screen.contains('…'), "{screen}");
        assert!(!screen.contains("cabe na coluna"), "{screen}");
    }

    #[test]
    fn erro_aparece_no_rodape() {
        let mut app = app_com_dados();
        app.status = Status::Failed(String::from("sem permissão"));
        let screen = screen(&mut app);
        assert!(screen.contains("erro: sem permissão"), "{screen}");
    }

    #[test]
    fn confirmacao_dupla_aparece_na_tela() {
        let mut app = app_com_dados();
        app.mode = crate::app::Mode::Confirm;
        app.edit = Some(crate::app::Edit {
            row: 0,
            column: 1,
            original: String::from("ana"),
            input: String::from("bia"),
        });
        let tela = screen(&mut app);
        assert!(tela.contains("De: ana"), "{tela}");
        assert!(tela.contains("Para: bia"), "{tela}");
        assert!(tela.contains("Deseja realmente alterar"), "{tela}");

        app.mode = crate::app::Mode::ConfirmAgain;
        let tela = screen(&mut app);
        assert!(tela.contains("Tem certeza?"), "{tela}");
    }

    #[test]
    fn selecao_preserva_a_cor_da_divisoria() {
        let mut app = app_com_dados();
        app.table_state.select(Some(0));
        app.table_state.select_column(Some(0));
        let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("desenho");
        // A divisória da linha selecionada tem o fundo da seleção, mas mantém o
        // cinza de `muted` em vez de herdar o `fg` do destaque.
        let divisoria = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .find(|cell| cell.symbol() == RULE && cell.bg == crate::theme::SURFACE0)
            .expect("esperava uma divisória na linha selecionada");
        assert_eq!(divisoria.fg, crate::theme::OVERLAY0);
    }

    #[test]
    fn nome_da_coluna_comeca_colado_na_divisoria() {
        let screen = screen(&mut app_com_dados());
        assert!(screen.contains("│name"), "{screen}");
        assert!(!screen.contains("│ name"), "{screen}");
    }

    #[test]
    fn destaque_da_coluna_cobre_a_largura_inteira() {
        let mut app = app_com_dados();
        app.table_state.select(Some(0));
        app.table_state.select_column(Some(0));
        app.mode = crate::app::Mode::Columns;
        let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("desenho");
        let destaque = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|cell| cell.bg == crate::theme::SURFACE1)
            .count();
        // `id` tem largura 2; somando os 2 espaços até a divisória, dá 4.
        assert_eq!(destaque, 4);
    }

    #[test]
    fn aplica_paleta_catppuccin_macchiato() {
        let mut app = app_com_dados();
        let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal de teste");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .expect("desenho");
        // O primeiro caractere é o espaço de " dbterm ": fundo `base` e accent `mauve`.
        let inicio = &terminal.backend().buffer().content()[0];
        assert_eq!(inicio.bg, crate::theme::BASE);
        assert_eq!(inicio.fg, crate::theme::MAUVE);
    }
}
