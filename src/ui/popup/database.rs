//! Pop-up "Editar" de um banco: a identificação, os padrões de texto e as
//! opções que o servidor informa.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::draw_picker;
use super::{EDIT_ROTULO, campo, popup, popup_content_width, popup_rule, tamanho};
use crate::app::DatabaseEdit;
use crate::config::DatabaseField;
use crate::theme;

pub(crate) fn draw_database_edit(frame: &mut Frame, area: Rect, edit: Option<&DatabaseEdit>) {
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
        // No MySQL não existe renomear banco: o nome é só identificação. O
        // valor sai como o de um campo preenchido; só o rótulo fica apagado.
        Line::from(campo(
            &edit.values.name,
            "Banco",
            theme::text(),
            EDIT_ROTULO,
            conteudo,
        )),
        Line::from(campo(
            &tamanho(edit.values.size_bytes),
            "Tamanho",
            theme::text(),
            EDIT_ROTULO,
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
        EDIT_ROTULO,
        comprimento,
    ))
}

/// Ações do pop-up de banco: `enter` abre a lista do campo (ou alterna o
/// sim/não) e `ctrl+s` grava.
fn linha_acoes_banco() -> Line<'static> {
    Line::from(vec![
        Span::styled("ctrl+s", theme::accent()),
        Span::styled(" salvar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" cancelar", theme::text()),
    ])
}
