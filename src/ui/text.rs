//! Corte de texto com `…`; a largura de exibição mora em `crate::view`.

use crate::view::largura;

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
