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
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, TableState};

use crate::app::{App, Focus, Mode, QueryInstance, ResultadoQuery, RightTab, Screen, Status};
use crate::db::TableData;
use crate::history::Saved;
use crate::theme;
use crate::view::largura;

const KEYS: &str = "q sair · c/b/t/u nível · esc subir · j/k mover · enter abrir · h/l colunas · r recarregar · g/G topo/fim";
/// Na cascata o `e` entra como ação de edição do nível ativo.
const KEYS_WITH_EDIT: &str = "q sair · c/b/t/u nível · esc subir · e editar · j/k mover · enter abrir · r recarregar · g/G topo/fim";
/// Teclas da lista de queries na lateral direita.
const QUERIES_KEYS: &str =
    "q sair · c/b/t nível · esc subir · j/k mover · enter abrir · n nova · d apagar · g/G topo/fim";
/// Teclas do editor de query no centro (tudo digita no rascunho).
const QUERY_EDITOR_KEYS: &str = "setas/ctrl+h,j,k,l movem · tab indenta · ctrl+enter/ctrl+e/F5 executa · ctrl+r/ctrl+J resultados · esc volta";
/// Teclas do painel de resultado abaixo do editor.
const RESULTADO_KEYS: &str =
    "q sair · esc editor · j/k mover · h/l rolar · g/G topo/fim · r executar";
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
        query_sql,
        connection_edit,
        database_edit,
        table_edit,
        commands,
        history,
        right_tab,
        queries,
        queries_state,
        active_query,
        query_result,
        ..
    } = app;

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    // Três colunas só quando há espaço (32 + 20 + 32 = 84): em terminal
    // estreito a direita se esconde e as queries aparecem na lateral
    // esquerda, no lugar da cascata, enquanto o foco está nelas.
    let mostrar_direita = body.width >= 90;
    if mostrar_direita {
        let [side, center, right] = Layout::horizontal([
            Constraint::Length(32),
            Constraint::Min(20),
            Constraint::Length(32),
        ])
        .areas(body);
        draw_lateral_esquerda(
            frame,
            side,
            history,
            databases,
            tables,
            connections_state,
            databases_state,
            list_state,
            *focus,
        );
        draw_lateral_direita(frame, right, *right_tab, queries, queries_state, *focus);
        draw_centro(
            frame,
            center,
            data,
            query_sql.as_deref(),
            table_state,
            *column,
            *hscroll,
            larguras,
            mode,
            *focus,
            database,
            active_query.and_then(|index| queries.get(index)),
            query_result.as_mut(),
        );
    } else {
        let [side, main] =
            Layout::horizontal([Constraint::Length(32), Constraint::Min(20)]).areas(body);
        if *focus == Focus::Queries {
            draw_lateral_direita(frame, side, *right_tab, queries, queries_state, *focus);
        } else {
            draw_lateral_esquerda(
                frame,
                side,
                history,
                databases,
                tables,
                connections_state,
                databases_state,
                list_state,
                *focus,
            );
        }
        draw_centro(
            frame,
            main,
            data,
            query_sql.as_deref(),
            table_state,
            *column,
            *hscroll,
            larguras,
            mode,
            *focus,
            database,
            active_query.and_then(|index| queries.get(index)),
            query_result.as_mut(),
        );
    }

    draw_header(frame, header, server, database, current.as_deref());

    // A lista aberta, em qualquer pop-up, fica com as teclas do rodapé.
    let lista_aberta = database_edit
        .as_ref()
        .is_some_and(|edit| edit.picker.is_some())
        || table_edit
            .as_ref()
            .is_some_and(|edit| edit.picker.is_some());
    let editor_aberto = active_query.and_then(|index| queries.get(index)).is_some();
    // Com o editor aberto, o rodapé acompanha o painel visível (resultado);
    // senão, a grade de tabelas.
    let (dados_rodape, fim_rodape) = match (editor_aberto, query_result.as_ref()) {
        (true, Some(result)) => (result.data.as_ref(), result.end_reached),
        _ => (data.as_ref(), *end_reached),
    };
    let teclas = if commands.is_some() {
        popup::commands_keys()
    } else if lista_aberta {
        PICKER_KEYS
    } else if let Some(edit) = table_edit.as_ref() {
        popup::table_keys(edit)
    } else if connection_edit.is_some() || database_edit.is_some() {
        POPUP_KEYS
    } else if *focus == Focus::Resultado {
        RESULTADO_KEYS
    } else if editor_aberto && *focus == Focus::Data {
        QUERY_EDITOR_KEYS
    } else if *focus == Focus::Queries {
        QUERIES_KEYS
    } else {
        keys(*mode, *focus)
    };
    draw_footer(frame, footer, status, teclas, dados_rodape, fim_rodape);
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

/// Lateral esquerda: chips e lista do nível ativo da cascata. Nas queries
/// e nos registros, o contexto continua sendo o das tabelas.
#[allow(clippy::too_many_arguments)]
fn draw_lateral_esquerda(
    frame: &mut Frame,
    area: Rect,
    history: &crate::history::History,
    databases: &[String],
    tables: &[String],
    connections_state: &mut ListState,
    databases_state: &mut ListState,
    list_state: &mut ListState,
    focus: Focus,
) {
    let ativo = match focus {
        Focus::Data | Focus::Queries | Focus::Resultado => Focus::Tables,
        outro => outro,
    };
    let [chips, lista] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
    let contagens = [history.all().len(), databases.len(), tables.len()];
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
            databases.iter().map(database_item).collect::<Vec<_>>(),
            databases_state,
        ),
        _ => (
            tables.iter().map(tabela_item).collect::<Vec<_>>(),
            list_state,
        ),
    };
    let list = List::new(itens)
        .block(panel(String::new(), focus == ativo))
        .highlight_symbol("❯ ")
        .highlight_style(theme::selected());
    frame.render_stateful_widget(list, lista, state);
}

/// Lateral direita, espelhando a esquerda: chips das abas e a lista da aba
/// ativa (hoje só queries, com as instâncias criadas por `n`).
fn draw_lateral_direita(
    frame: &mut Frame,
    area: Rect,
    right_tab: RightTab,
    queries: &[QueryInstance],
    queries_state: &mut ListState,
    focus: Focus,
) {
    let [chips_dir, lista_dir] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
    frame.render_widget(
        Paragraph::new(right_chips_line(
            right_tab,
            focus == Focus::Queries,
            queries.len(),
        ))
        .centered(),
        chips_dir,
    );
    // Sem instâncias, o meio do painel vira o convite de criação.
    if queries.is_empty() {
        let bloco = panel(String::new(), focus == Focus::Queries);
        let dentro = bloco.inner(lista_dir);
        frame.render_widget(bloco, lista_dir);
        let [_, meio, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(dentro);
        let dica = Line::from(vec![
            Span::styled("[", theme::muted()),
            Span::styled("n", theme::accent()),
            Span::styled("]", theme::muted()),
            Span::styled(" nova query", theme::text()),
        ]);
        frame.render_widget(Paragraph::new(dica).centered(), meio);
        return;
    }
    let itens_dir: Vec<ListItem> = queries.iter().map(query_item).collect();
    let list_dir = List::new(itens_dir)
        .block(panel(String::new(), focus == Focus::Queries))
        .highlight_symbol("❯ ")
        .highlight_style(theme::selected());
    frame.render_stateful_widget(list_dir, lista_dir, queries_state);
}

/// Centro: editor da query aberta ou a grade de registros. A faixa da busca
/// fica reservada nos dois modos para o editor ter a altura da grade.
#[allow(clippy::too_many_arguments)]
fn draw_centro(
    frame: &mut Frame,
    area: Rect,
    data: &Option<TableData>,
    query_sql: Option<&str>,
    table_state: &mut TableState,
    column: Option<usize>,
    hscroll: usize,
    larguras: &[usize],
    mode: &Mode,
    focus: Focus,
    database: &str,
    query_aberta: Option<&QueryInstance>,
    resultado: Option<&mut ResultadoQuery>,
) {
    // A faixa da busca fica reservada nos dois modos para o editor começar
    // na mesma linha da grade; sem busca ou sem query aberta, ela fica em
    // branco mas ocupa a linha para não pular.
    let [faixa, conteudo] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
    // Com execução, o conteúdo divide: editor em cima e resultado embaixo;
    // sem ela, o editor ocupa o conteúdo todo, na mesma altura da grade.
    if let Some(query) = query_aberta {
        match resultado {
            Some(result) => {
                let [acima, abaixo] =
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(conteudo);
                draw_query_editor(frame, acima, query, database, focus == Focus::Data);
                draw_resultado(frame, abaixo, result, focus == Focus::Resultado);
            }
            None => draw_query_editor(frame, conteudo, query, database, focus == Focus::Data),
        }
        return;
    }
    if let Some(sql) = query_sql {
        let icone = format!("{ICONE_TABELA} ");
        let disponivel = (faixa.width as usize).saturating_sub(largura(&icone));
        let cortado =
            text::fit(sql, disponivel).map(|corte| corte.trim_end_matches('…').to_owned());
        let texto = cortado.as_deref().unwrap_or(sql);
        let mut spans = vec![Span::styled(icone, theme::accent())];
        spans.extend(
            text::realces(texto)
                .into_iter()
                .map(|(pedaco, papel)| Span::styled(pedaco, popup::parte_estilo(papel))),
        );
        if cortado.is_some() {
            spans.push(Span::styled("…", theme::muted()));
        }
        let linha = Line::from(spans);
        frame.render_widget(Paragraph::new(linha).style(theme::selected()), faixa);
    }
    grid::draw_data(
        frame,
        conteudo,
        data.as_ref(),
        focus == Focus::Data,
        table_state,
        grid::Posicao {
            coluna: column,
            rolagem: hscroll,
            larguras,
        },
        mode,
        "dados",
        "linhas",
    );
}

/// Painel inferior do editor: os registros da última execução, com a tabela
/// e a contagem no título. Sem dados ainda, o aviso de carregamento.
fn draw_resultado(frame: &mut Frame, area: Rect, result: &mut ResultadoQuery, focused: bool) {
    let base = match result.tabela.as_deref() {
        Some(tabela) => format!("resultado · {tabela}"),
        None => String::from("resultado · consulta"),
    };
    let Some(data) = result.data.as_ref() else {
        let hint = Paragraph::new("carregando consulta…")
            .block(panel(format!("{base} · 0 registros"), focused))
            .style(theme::warning());
        frame.render_widget(hint, area);
        return;
    };
    grid::draw_data(
        frame,
        area,
        Some(data),
        focused,
        &mut result.state,
        grid::Posicao {
            coluna: None,
            rolagem: result.hscroll,
            larguras: &result.larguras,
        },
        &Mode::Rows,
        &base,
        "registros",
    );
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
    data: Option<&TableData>,
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
    if focus == Focus::Queries {
        return QUERIES_KEYS;
    }
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

/// Ícone Nerd Font da aba de queries na lateral direita.
const ICONE_QUERY: char = '\u{f120}'; // terminal

/// Linha de chips da lateral direita, espelhando a esquerda. Hoje só tem a
/// aba queries; as próximas entram em `RightTab::ALL` e aparecem aqui. O
/// número entre parênteses é o tanto de instâncias na aba.
fn right_chips_line(ativa: RightTab, focada: bool, total: usize) -> Line<'static> {
    let mut spans = Vec::new();
    for (indice, aba) in RightTab::ALL.iter().enumerate() {
        if indice > 0 {
            spans.push(Span::styled(" ", theme::muted()));
        }
        let rotulo = format!("{} {} ({total})", aba.icone(), aba.titulo());
        if *aba == ativa && focada {
            spans.push(Span::styled(rotulo, theme::accent()));
        } else if *aba == ativa {
            spans.push(Span::styled(rotulo, theme::text()));
        } else {
            spans.push(Span::styled("[", theme::muted()));
            spans.push(Span::styled(format!("{} ", aba.icone()), theme::accent()));
            spans.push(Span::styled(aba.atalho().to_string(), theme::accent()));
            spans.push(Span::styled("]", theme::muted()));
        }
    }
    Line::from(spans)
}

/// Uma instância de query como item da lista da direita: ícone, nome e
/// tamanho (ou `vazio` ao criar).
fn query_item(query: &crate::app::QueryInstance) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(format!("{ICONE_QUERY} "), theme::accent()),
        Span::styled(query.nome.clone(), theme::text()),
        Span::styled(" ", theme::muted()),
        Span::styled(query.resumo(), theme::muted()),
    ]))
}

/// Editor vazio da query no centro, no lugar da grade. O título traz o
/// relevante da instância: nome, schema atual e tamanho em chars/linhas;
/// vazio mostra a dica em vez do texto.
fn draw_query_editor(
    frame: &mut Frame,
    area: Rect,
    query: &crate::app::QueryInstance,
    database: &str,
    focused: bool,
) {
    let schema = if database.is_empty() {
        "(nenhum)"
    } else {
        database
    };
    let title = if query.sql.is_empty() {
        format!("query · {} · schema: {schema} · vazio", query.nome)
    } else {
        let chars = query.sql.chars().count();
        let linhas = query.sql.lines().count().max(1);
        format!(
            "query · {} · schema: {schema} · {chars} chars · {linhas} linhas",
            query.nome
        )
    };
    if query.sql.is_empty() {
        let dica = Paragraph::new("digite o SQL · esc volta às queries")
            .block(panel(title, focused))
            .style(theme::muted());
        frame.render_widget(dica, area);
        return;
    }
    // O realce vale para o texto inteiro (literal pode cruzar linhas); cada
    // `\n` vira uma `Line` para o `Paragraph` desenhar as quebras. Com foco,
    // o caractere sob o cursor sai invertido; no fim da linha, um bloco.
    let mut linhas: Vec<Vec<(String, Style)>> = vec![Vec::new()];
    for (pedaco, papel) in text::realces(&query.sql) {
        let estilo = popup::parte_estilo(papel);
        let mut partes = pedaco.split('\n');
        if let Some(primeira) = partes.next() {
            if !primeira.is_empty() {
                linhas
                    .last_mut()
                    .expect("linha")
                    .push((primeira.to_owned(), estilo));
            }
            for parte in partes {
                linhas.push(if parte.is_empty() {
                    Vec::new()
                } else {
                    vec![(parte.to_owned(), estilo)]
                });
            }
        }
    }
    let (linha_cursor, coluna_cursor) = query.linha_col();
    let texto: Vec<Line> = linhas
        .into_iter()
        .enumerate()
        .map(|(indice, pedacos)| {
            let coluna = (focused && indice == linha_cursor).then_some(coluna_cursor);
            Line::from(aplicar_cursor(pedacos, coluna))
        })
        .collect();
    let editor = Paragraph::new(texto).block(panel(title, focused));
    frame.render_widget(editor, area);
}

/// Os pedaços de uma linha do editor como `Span`s, com o cursor aplicado:
/// o caractere na coluna sai invertido e, no fim da linha, um bloco.
/// Sem coluna (`None`, editor sem foco), só converte os estilos.
fn aplicar_cursor(pedacos: Vec<(String, Style)>, coluna: Option<usize>) -> Vec<Span<'static>> {
    let Some(coluna) = coluna else {
        return pedacos
            .into_iter()
            .map(|(texto, estilo)| Span::styled(texto, estilo))
            .collect();
    };
    let mut spans = Vec::new();
    let mut vistos = 0;
    let mut aplicado = false;
    for (texto, estilo) in pedacos {
        let tamanho = texto.chars().count();
        if !aplicado && coluna >= vistos && coluna < vistos + tamanho {
            let antes: String = texto.chars().take(coluna - vistos).collect();
            let mut resto = texto.chars().skip(coluna - vistos);
            let letra = resto.next().unwrap_or(' ');
            let depois: String = resto.collect();
            if !antes.is_empty() {
                spans.push(Span::styled(antes, estilo));
            }
            spans.push(Span::styled(letra.to_string(), theme::cursor()));
            if !depois.is_empty() {
                spans.push(Span::styled(depois, estilo));
            }
            aplicado = true;
        } else {
            spans.push(Span::styled(texto, estilo));
        }
        vistos += tamanho;
    }
    if !aplicado {
        spans.push(Span::styled(" ", theme::cursor()));
    }
    spans
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
