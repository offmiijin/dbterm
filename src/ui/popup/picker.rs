//! Lista de opções aberta sobre um pop-up de edição.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::{popup, popup_content_width, popup_rule};
use crate::theme;
use crate::view::largura;

/// Lista de opções do campo: a janela acompanha o item destacado.
pub(crate) fn draw_picker(
    frame: &mut Frame,
    area: Rect,
    items: &[String],
    index: usize,
    titulo: &str,
) {
    let conteudo = popup_content_width(area);
    // O pop-up gasta duas linhas com a régua e as ações, mais as bordas.
    let altura = area.height.saturating_sub(4) as usize;
    let total = items.len();
    let visiveis = altura.min(total).max(1);
    let inicio = janela(index, visiveis, total);
    let mut lines: Vec<Line<'static>> = items[inicio..inicio + visiveis]
        .iter()
        .enumerate()
        .map(|(offset, item)| item_lista(item, inicio + offset == index, conteudo))
        .collect();
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_acoes_lista());
    // O ícone (Nerd Font) de caret para baixo indica a lista aberta.
    popup(frame, area, &format!("\u{f0d7} {titulo}"), lines);
}

/// Primeira linha da janela da lista: o item destacado fica no meio quando há
/// espaço dos dois lados e a janela não passa do fim.
pub(crate) fn janela(index: usize, visiveis: usize, total: usize) -> usize {
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

/// Ações da lista de opções.
fn linha_acoes_lista() -> Line<'static> {
    Line::from(vec![
        Span::styled("j/k", theme::accent()),
        Span::styled(" mover · ", theme::text()),
        Span::styled("enter", theme::accent()),
        Span::styled(" selecionar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" voltar", theme::text()),
    ])
}
