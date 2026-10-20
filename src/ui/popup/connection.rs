//! Pop-up "Editar" de uma conexão salva: o servidor em cima, a autenticação
//! embaixo.

use super::super::connect::mostrado;
use super::{CAMPO_ROTULO, campo, popup, popup_content_width, popup_rule};
use crate::app::ConnectionEdit;
use crate::config::Field;
use crate::theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

pub(crate) fn draw_connection_edit(frame: &mut Frame, area: Rect, edit: Option<&ConnectionEdit>) {
    let Some(edit) = edit else {
        return;
    };
    let conteudo = popup_content_width(area);

    let lines = vec![
        Line::from("Servidor").style(theme::header()),
        campo_linha(edit, Field::Host, conteudo),
        campo_linha(edit, Field::Port, conteudo),
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

/// Ações do pop-up de conexão: `ctrl+s` grava e `esc` descarta.
fn linha_edicao() -> Line<'static> {
    Line::from(vec![
        Span::styled("ctrl+s", theme::accent()),
        Span::styled(" salvar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" cancelar", theme::text()),
    ])
}
