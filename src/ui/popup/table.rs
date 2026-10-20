//! Pop-up "Editar" de uma tabela: a identificação, o armazenamento e os
//! padrões de texto.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::{CAMPO_ESPACO, COMENTARIO_LINHAS, EDIT_ROTULO, campo, popup, popup_content_width, popup_rule, tamanho};
use super::draw_picker;
use crate::app::TableEdit;
use crate::config::TableField;
use crate::theme;
use crate::view::largura;

pub(crate) fn draw_table_edit(frame: &mut Frame, area: Rect, edit: Option<&TableEdit>) {
    let Some(edit) = edit else {
        return;
    };
    // A lista aberta toma o lugar do pop-up até a escolha.
    if let Some(picker) = edit.picker.as_ref() {
        draw_picker(
            frame,
            area,
            &picker.items,
            picker.index,
            picker.field.titulo(),
        );
        return;
    }
    let conteudo = popup_content_width(area);
    let mut lines = vec![
        Line::from("Identificação").style(theme::header()),
        // O nome ocupa a linha inteira: nome de tabela pode ser longo.
        linha_tabela(edit, TableField::Name, conteudo),
        // O tamanho é só informação, e fica logo abaixo do nome.
        Line::from(campo(
            &tamanho(edit.values.size_bytes),
            "Tamanho",
            theme::text(),
            EDIT_ROTULO,
            conteudo,
        )),
    ];
    // O comentário fica na linha do rótulo, com a caixa descendo duas linhas.
    lines.extend(caixa_comentario(edit, conteudo));
    lines.push(Line::from("Armazenamento").style(theme::header()));
    lines.push(linha_tabela(edit, TableField::Engine, conteudo));
    lines.push(linha_tabela(edit, TableField::RowFormat, conteudo));
    if edit.available(TableField::AutoIncrement) {
        lines.push(linha_tabela(edit, TableField::AutoIncrement, conteudo));
    }
    lines.push(Line::from("Padrões de texto").style(theme::header()));
    lines.push(linha_tabela(edit, TableField::Charset, conteudo));
    lines.push(linha_tabela(edit, TableField::Collation, conteudo));
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_acoes_tabela(edit));
    // O ícone (Nerd Font) acompanha o título do pop-up.
    popup(frame, area, "\u{f044} Editar", lines);
}

/// Uma linha do pop-up de tabela com um campo ocupando a largura toda.
fn linha_tabela(edit: &TableEdit, field: TableField, comprimento: usize) -> Line<'static> {
    Line::from(campo_tabela(edit, field, EDIT_ROTULO, comprimento))
}

/// Spans de um campo da tabela: os de lista levam o marcador de dropdown e os
/// digitados, o cursor quando estão com o foco.
fn campo_tabela(
    edit: &TableEdit,
    field: TableField,
    rotulo_largura: usize,
    comprimento: usize,
) -> Vec<Span<'static>> {
    let focado = edit.field == field;
    let mut valor = edit.value(field);
    if field.dropdown() {
        valor.insert_str(0, "▾ ");
    } else if focado {
        valor.push('▏');
    }
    let estilo = if focado {
        theme::field()
    } else {
        theme::text()
    };
    campo(&valor, field.titulo(), estilo, rotulo_largura, comprimento)
}

/// Caixa do comentário da tabela: o texto na frente do rótulo e as linhas
/// seguintes logo abaixo, na mesma coluna e com a mesma largura do valor.
fn caixa_comentario(edit: &TableEdit, comprimento: usize) -> Vec<Line<'static>> {
    let focado = edit.field == TableField::Comment;
    let mut valor = edit.value(TableField::Comment);
    if focado {
        valor.push('▏');
    }
    let disponivel = comprimento.saturating_sub(EDIT_ROTULO + CAMPO_ESPACO);
    let estilo = if focado {
        theme::field()
    } else {
        theme::text()
    };
    em_linhas(&valor, disponivel, COMENTARIO_LINHAS)
        .into_iter()
        .enumerate()
        .map(|(indice, linha)| {
            // Só a primeira linha leva o rótulo; as de baixo começam na coluna
            // do valor, para as caixas ficarem do mesmo tamanho.
            let rotulo = if indice == 0 {
                format!(
                    "{:>width$}{}",
                    TableField::Comment.titulo(),
                    " ".repeat(CAMPO_ESPACO),
                    width = EDIT_ROTULO
                )
            } else {
                " ".repeat(EDIT_ROTULO + CAMPO_ESPACO)
            };
            let preenchimento = disponivel.saturating_sub(largura(&linha));
            Line::from(vec![
                Span::styled(rotulo, theme::muted()),
                Span::styled(format!("{linha}{}", " ".repeat(preenchimento)), estilo),
            ])
        })
        .collect()
}

/// Texto dividido nas linhas da caixa. Quando passa do que cabe, a janela
/// mostra as últimas linhas — é onde fica o cursor —, sem repetir texto: cada
/// linha aparece uma vez só.
fn em_linhas(texto: &str, limite: usize, linhas: usize) -> Vec<String> {
    if limite == 0 {
        return vec![String::new(); linhas];
    }
    let mut pedacos = quebra(texto, limite);
    if pedacos.len() > linhas {
        pedacos = pedacos.split_off(pedacos.len() - linhas);
    }
    pedacos.resize(linhas, String::new());
    pedacos
}

/// Linhas de até `limite` colunas, sem cortar palavra: a que não couber no fim
/// da linha desce inteira. Só uma palavra maior que a linha é partida.
fn quebra(texto: &str, limite: usize) -> Vec<String> {
    let mut linhas = Vec::new();
    let mut atual = String::new();
    for palavra in texto.split(' ') {
        let separador = if atual.is_empty() { "" } else { " " };
        let candidata = format!("{atual}{separador}{palavra}");
        if largura(&candidata) <= limite {
            atual = candidata;
            continue;
        }
        if !atual.is_empty() {
            linhas.push(std::mem::take(&mut atual));
        }
        if largura(palavra) <= limite {
            atual = palavra.to_owned();
            continue;
        }
        // Palavra sozinha maior que a linha inteira: aí não tem como descer.
        let mut pedacos = quebra_dura(palavra, limite);
        atual = pedacos.pop().unwrap_or_default();
        linhas.extend(pedacos);
    }
    linhas.push(atual);
    linhas
}

/// Pedaços de até `limite` colunas, cortando no meio da palavra.
fn quebra_dura(texto: &str, limite: usize) -> Vec<String> {
    let mut pedacos = Vec::new();
    let mut atual = String::new();
    for caractere in texto.chars() {
        let mut candidato = atual.clone();
        candidato.push(caractere);
        if largura(&candidato) > limite {
            pedacos.push(std::mem::take(&mut atual));
        }
        atual.push(caractere);
    }
    pedacos.push(atual);
    pedacos
}

/// Ações do pop-up de tabela: no campo de texto `enter` salva; nos de lista,
/// `enter` abre a lista e `s` salva.
fn linha_acoes_tabela(edit: &TableEdit) -> Line<'static> {
    if edit.field.dropdown() {
        return Line::from(vec![
            Span::styled("enter", theme::accent()),
            Span::styled(" lista · ", theme::text()),
            Span::styled("s", theme::accent()),
            Span::styled(" salvar · ", theme::text()),
            Span::styled("esc", theme::accent()),
            Span::styled(" cancelar", theme::text()),
        ]);
    }
    Line::from(vec![
        Span::styled("enter", theme::accent()),
        Span::styled(" salvar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" cancelar", theme::text()),
    ])
}

/// Teclas do pop-up da tabela, como saem no rodapé.
pub(crate) fn table_keys(edit: &TableEdit) -> &'static str {
    if edit.field.dropdown() {
        "enter lista · s salvar · esc cancelar"
    } else {
        "enter salvar · esc cancelar"
    }
}
