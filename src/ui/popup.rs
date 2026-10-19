//! Pop-ups de edição e das duas confirmações, com o preview do SQL.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph};

use super::connect::mostrado;
use super::text::largura;
use crate::app::{ConnectionEdit, Edit, Mode};
use crate::config::{Connection, Field};
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
    // A confirmação de saída não depende de uma edição em andamento.
    if matches!(mode, Mode::ConfirmQuit) {
        let lines = vec![
            Line::from("Deseja fechar o dbterm?"),
            Line::from(popup_rule(area)).style(theme::muted()),
            linha_acoes("sair"),
        ];
        popup(frame, area, "\u{f011} Sair", lines);
        return;
    }

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
        Mode::Rows | Mode::Columns | Mode::ConfirmQuit => {}
    }
}

/// Largura do rótulo de cada campo do pop-up de conexão.
const CAMPO_ROTULO: usize = 8;
/// Respiro entre o rótulo e o valor e entre dois campos na mesma linha.
const CAMPO_ESPACO: usize = 2;
/// Largura do campo "Porta", que divide a linha com o "Host".
const PORTA: usize = 16;

/// Pop-up "Editar" de uma conexão salva: o servidor em cima, a autenticação
/// embaixo.
pub(super) fn draw_connection_edit(frame: &mut Frame, area: Rect, edit: Option<&ConnectionEdit>) {
    let Some(edit) = edit else {
        return;
    };
    let conteudo = popup_content_width(area);
    // "Host" e "Porta" lado a lado: a porta fica com a largura fixa e o host
    // com o resto da linha.
    let host = conteudo.saturating_sub(PORTA + CAMPO_ESPACO);
    let mut linha_host = campo(
        &edit.connection,
        Field::Host,
        edit.field == Field::Host,
        host,
    );
    linha_host.push(Span::styled(" ".repeat(CAMPO_ESPACO), theme::base()));
    linha_host.extend(campo(
        &edit.connection,
        Field::Port,
        edit.field == Field::Port,
        PORTA,
    ));

    let lines = vec![
        Line::from("Servidor").style(theme::header()),
        Line::from(linha_host),
        campo_linha(&edit.connection, edit, Field::Database, conteudo),
        Line::from(""),
        Line::from("Autenticação").style(theme::header()),
        campo_linha(&edit.connection, edit, Field::User, conteudo),
        campo_linha(&edit.connection, edit, Field::Password, conteudo),
        Line::from(popup_rule(area)).style(theme::muted()),
        linha_edicao(),
    ];
    // O ícone (Nerd Font) acompanha o título do pop-up.
    popup(frame, area, "\u{f044} Editar", lines);
}

/// Uma linha do pop-up com um campo ocupando a largura toda.
fn campo_linha(
    connection: &Connection,
    edit: &ConnectionEdit,
    field: Field,
    comprimento: usize,
) -> Line<'static> {
    Line::from(campo(connection, field, edit.field == field, comprimento))
}

/// Rótulo à direita e valor ocupando o resto de `comprimento`; o campo com foco
/// ganha fundo e o cursor no fim.
fn campo(
    connection: &Connection,
    field: Field,
    focado: bool,
    comprimento: usize,
) -> Vec<Span<'static>> {
    let rotulo = format!(
        "{:>width$}{}",
        field.titulo(),
        " ".repeat(CAMPO_ESPACO),
        width = CAMPO_ROTULO
    );
    let mut valor = mostrado(connection, field);
    if focado {
        valor.push('▏');
    }
    let disponivel = comprimento.saturating_sub(CAMPO_ROTULO + CAMPO_ESPACO);
    let valor = recorta(&valor, disponivel);
    let preenchimento = disponivel.saturating_sub(largura(&valor));
    let estilo = if focado {
        theme::field()
    } else {
        theme::text()
    };
    vec![
        Span::styled(rotulo, theme::muted()),
        Span::styled(format!("{valor}{}", " ".repeat(preenchimento)), estilo),
    ]
}

/// Corta o começo do valor quando ele não cabe no campo: o fim, onde fica o
/// cursor, é o que importa enquanto se digita.
fn recorta(valor: &str, limite: usize) -> String {
    if largura(valor) <= limite {
        return valor.to_owned();
    }
    let mut caracteres: Vec<char> = valor.chars().collect();
    while !caracteres.is_empty() {
        let resto: String = format!("…{}", caracteres.iter().collect::<String>());
        if largura(&resto) <= limite {
            return resto;
        }
        caracteres.remove(0);
    }
    String::from("…")
}

/// Ações do pop-up de conexão: aqui `tab` também anda nos campos.
fn linha_edicao() -> Line<'static> {
    Line::from(vec![
        Span::styled("tab", theme::accent()),
        Span::styled(" campo · ", theme::text()),
        Span::styled("enter", theme::accent()),
        Span::styled(" salvar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" cancelar", theme::text()),
    ])
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
