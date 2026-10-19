//! Medidas de texto: largura de exibição e corte com `…`.

use ratatui::text::Span;

/// `Some(texto)` quando o valor não cabe na coluna: corta e marca com `…`.
pub(super) fn fit(value: &str, limite: usize) -> Option<String> {
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
pub(super) fn largura(texto: &str) -> usize {
    Span::raw(texto).width()
}
