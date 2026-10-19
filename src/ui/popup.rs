//! Pop-ups de edição e das duas confirmações, com o preview do SQL.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph};

use super::connect::mostrado;
use super::text::largura;
use crate::app::{ConnectionEdit, DatabaseEdit, Edit, Mode, Picker};
use crate::config::{DatabaseField, Field};
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
/// Largura do rótulo dos campos do pop-up de banco ("Somente leitura" manda).
const BANCO_ROTULO: usize = 15;
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
    let mut linha_host = campo_conexao(edit, Field::Host, CAMPO_ROTULO, host);
    linha_host.push(Span::styled(" ".repeat(CAMPO_ESPACO), theme::base()));
    linha_host.extend(campo_conexao(edit, Field::Port, CAMPO_ROTULO, PORTA));

    let lines = vec![
        Line::from("Servidor").style(theme::header()),
        Line::from(linha_host),
        campo_linha(edit, Field::Database, conteudo),
        Line::from(""),
        Line::from("Autenticação").style(theme::header()),
        campo_linha(edit, Field::User, conteudo),
        campo_linha(edit, Field::Password, conteudo),
        Line::from(popup_rule(area)).style(theme::muted()),
        linha_edicao(),
    ];
    // O ícone (Nerd Font) acompanha o título do pop-up.
    popup(frame, area, "\u{f044} Editar", lines);
}

/// Uma linha do pop-up de conexão com um campo ocupando a largura toda.
fn campo_linha(edit: &ConnectionEdit, field: Field, comprimento: usize) -> Line<'static> {
    Line::from(campo_conexao(edit, field, CAMPO_ROTULO, comprimento))
}

/// Spans de um campo da conexão, com o cursor quando ele tem o foco.
fn campo_conexao(
    edit: &ConnectionEdit,
    field: Field,
    rotulo_largura: usize,
    comprimento: usize,
) -> Vec<Span<'static>> {
    let focado = edit.field == field;
    let mut valor = mostrado(&edit.connection, field);
    if focado {
        valor.push('▏');
    }
    let estilo = if focado {
        theme::field()
    } else {
        theme::text()
    };
    campo(&valor, field.titulo(), estilo, rotulo_largura, comprimento)
}

/// Pop-up "Editar" de um banco: a identificação, os padrões de texto e as
/// opções que o servidor informa.
pub(super) fn draw_database_edit(frame: &mut Frame, area: Rect, edit: Option<&DatabaseEdit>) {
    let Some(edit) = edit else {
        return;
    };
    // A lista aberta toma o lugar do pop-up até a escolha.
    if let Some(picker) = edit.picker.as_ref() {
        draw_picker(frame, area, picker);
        return;
    }
    let conteudo = popup_content_width(area);
    let mut lines = vec![
        Line::from("Identificação").style(theme::header()),
        // No MySQL não existe renomear banco: o nome é só identificação. O
        // valor sai como o de um campo preenchido; só o rótulo fica apagado.
        Line::from(campo(
            &edit.values.name,
            "Banco",
            theme::text(),
            BANCO_ROTULO,
            conteudo,
        )),
        Line::from(campo(
            &tamanho(edit.values.size_bytes),
            "Tamanho",
            theme::text(),
            BANCO_ROTULO,
            conteudo,
        )),
        Line::from("Padrões").style(theme::header()),
        linha_banco(edit, DatabaseField::Charset, conteudo),
        linha_banco(edit, DatabaseField::Collation, conteudo),
    ];
    if edit.available(DatabaseField::ReadOnly) {
        lines.push(Line::from("Opções").style(theme::header()));
        lines.push(linha_banco(edit, DatabaseField::ReadOnly, conteudo));
    }
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_acoes_banco());
    // O ícone (Nerd Font) acompanha o título do pop-up.
    popup(frame, area, "\u{f044} Editar", lines);
}

/// Uma linha do pop-up de banco; os campos de lista levam o marcador de
/// dropdown e o de sim/não mostra o valor direto.
fn linha_banco(edit: &DatabaseEdit, field: DatabaseField, comprimento: usize) -> Line<'static> {
    let focado = edit.field == field;
    let mut valor = edit.value(field);
    if field.dropdown() {
        valor.insert_str(0, "▾ ");
    }
    let estilo = if focado {
        theme::field()
    } else {
        theme::text()
    };
    Line::from(campo(
        &valor,
        field.titulo(),
        estilo,
        BANCO_ROTULO,
        comprimento,
    ))
}

/// Lista de opções do campo: a janela acompanha o item destacado.
fn draw_picker(frame: &mut Frame, area: Rect, picker: &Picker) {
    let conteudo = popup_content_width(area);
    // O pop-up gasta duas linhas com a régua e as ações, mais as bordas.
    let altura = area.height.saturating_sub(4) as usize;
    let total = picker.items.len();
    let visiveis = altura.min(total).max(1);
    let inicio = janela(picker.index, visiveis, total);
    let mut lines: Vec<Line<'static>> = picker.items[inicio..inicio + visiveis]
        .iter()
        .enumerate()
        .map(|(offset, item)| item_lista(item, inicio + offset == picker.index, conteudo))
        .collect();
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_acoes_lista());
    // O ícone (Nerd Font) de caret para baixo indica a lista aberta.
    popup(
        frame,
        area,
        &format!("\u{f0d7} {}", picker.field.titulo()),
        lines,
    );
}

/// Primeira linha da janela da lista: o item destacado fica no meio quando há
/// espaço dos dois lados e a janela não passa do fim.
pub(super) fn janela(index: usize, visiveis: usize, total: usize) -> usize {
    index
        .saturating_sub(visiveis / 2)
        .min(total.saturating_sub(visiveis))
}

/// Uma opção da lista, com o fundo da seleção cobrindo a linha inteira.
fn item_lista(item: &str, selecionado: bool, comprimento: usize) -> Line<'static> {
    let marcador = if selecionado { "❯ " } else { "  " };
    let mut texto = format!("{marcador}{item}");
    texto.push_str(&" ".repeat(comprimento.saturating_sub(largura(&texto))));
    let estilo = if selecionado {
        theme::selected()
    } else {
        theme::text()
    };
    Line::from(Span::styled(texto, estilo))
}

/// Tamanho legível: `0 B`, `12.3 KB`, `742 MB`, `1.5 GB`.
pub(super) fn tamanho(bytes: u64) -> String {
    const UNIDADES: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut valor = bytes as f64;
    let mut unidade = 0;
    while valor >= 1024.0 && unidade + 1 < UNIDADES.len() {
        valor /= 1024.0;
        unidade += 1;
    }
    if unidade == 0 {
        return format!("{bytes} B");
    }
    if valor < 10.0 {
        format!("{valor:.1} {}", UNIDADES[unidade])
    } else {
        format!("{valor:.0} {}", UNIDADES[unidade])
    }
}

/// Rótulo à direita e valor ocupando o resto de `comprimento`, no estilo dado.
fn campo(
    valor: &str,
    rotulo: &str,
    estilo: Style,
    rotulo_largura: usize,
    comprimento: usize,
) -> Vec<Span<'static>> {
    let rotulo = format!(
        "{:>width$}{}",
        rotulo,
        " ".repeat(CAMPO_ESPACO),
        width = rotulo_largura
    );
    let disponivel = comprimento.saturating_sub(rotulo_largura + CAMPO_ESPACO);
    let valor = recorta(valor, disponivel);
    let preenchimento = disponivel.saturating_sub(largura(&valor));
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

/// Ações do pop-up de conexão: `s` grava e `esc` descarta.
fn linha_edicao() -> Line<'static> {
    Line::from(vec![
        Span::styled("s", theme::accent()),
        Span::styled(" salvar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" cancelar", theme::text()),
    ])
}

/// Ações do pop-up de banco: `enter` abre a lista do campo (ou alterna o
/// sim/não) e `s` grava.
fn linha_acoes_banco() -> Line<'static> {
    Line::from(vec![
        Span::styled("s", theme::accent()),
        Span::styled(" salvar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" cancelar", theme::text()),
    ])
}

/// Ações da lista de opções.
fn linha_acoes_lista() -> Line<'static> {
    Line::from(vec![
        Span::styled("j/k", theme::accent()),
        Span::styled(" mover · ", theme::text()),
        Span::styled("enter", theme::accent()),
        Span::styled(" escolher · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" voltar", theme::text()),
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
