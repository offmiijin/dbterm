//! Tela de conexão: a marca em letras grandes e o formulário.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::banner;
use crate::app::App;
use crate::view::largura;
use crate::config::{Connection, Field};
use crate::theme;

/// Largura do rótulo de cada campo, alinhado à direita.
const ROTULO: usize = 8;
/// Espaço entre o rótulo e o valor.
const ESPACO: usize = 2;
/// Largura mínima do grupo de campos.
const GRUPO_MIN: usize = 40;

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let linhas = linhas(app, area.width);
    let [_, meio, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(linhas.len() as u16),
        Constraint::Fill(1),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(linhas), meio);
}

fn linhas(app: &App, largura_tela: u16) -> Vec<Line<'static>> {
    let mut linhas = banner(largura_tela);
    linhas.push(Line::from(""));
    linhas.push(
        Line::from("conecte-se a um banco MySQL")
            .style(theme::muted())
            .centered(),
    );
    linhas.push(Line::from(""));

    let grupo = grupo(&app.connection);
    for field in Field::ALL {
        linhas.push(campo(&app.connection, field, field == app.field, grupo).centered());
    }

    linhas.push(Line::from(""));
    linhas.push(aviso(app).centered());
    linhas
}

/// Largura do grupo de campos: o rótulo mais largo, mais o espaço do cursor,
/// nunca menos que [`GRUPO_MIN`] para a caixa não tremer ao digitar.
fn grupo(connection: &Connection) -> usize {
    Field::ALL
        .iter()
        .map(|field| ROTULO + ESPACO + largura(&mostrado(connection, *field)))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(GRUPO_MIN)
}

/// Valor como aparece na tela: a senha sai como bolinhas.
pub(super) fn mostrado(connection: &Connection, field: Field) -> String {
    let valor = connection.value(field);
    if field == Field::Password {
        "•".repeat(valor.chars().count())
    } else {
        valor.to_owned()
    }
}

/// Uma linha do formulário: rótulo em tom secundário e o valor; o campo com
/// foco ganha fundo e o cursor no fim.
fn campo(connection: &Connection, field: Field, focado: bool, grupo: usize) -> Line<'static> {
    let rotulo = format!(
        "{:>width$}{}",
        field.label(),
        " ".repeat(ESPACO),
        width = ROTULO
    );
    let mut valor = mostrado(connection, field);
    if focado {
        valor.push('▏');
    }
    let preenchimento = grupo.saturating_sub(ROTULO + ESPACO + largura(&valor));
    let estilo = if focado {
        theme::field()
    } else {
        theme::text()
    };
    Line::from(vec![
        Span::styled(rotulo, theme::muted()),
        Span::styled(format!("{valor}{}", " ".repeat(preenchimento)), estilo),
    ])
}

/// Rodapé da tela: o que está acontecendo ou a dica de teclas.
fn aviso(app: &App) -> Line<'static> {
    if app.connecting {
        Line::from("conectando…").style(theme::warning())
    } else if let Some(erro) = &app.connect_error {
        Line::from(format!("erro: {erro}")).style(theme::error())
    } else {
        Line::from("enter conectar · tab próximo campo · esc voltar").style(theme::muted())
    }
}
