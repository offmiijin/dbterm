//! Medidas neutras da grade: sem estado do app, sem desenho.
//!
//! `app` mede uma vez por mudança de dados e `ui` consome; nenhum dos dois
//! precisa conhecer o outro por causa disso.

use ratatui::text::Span;

use crate::db::TableData;

/// Largura máxima de uma coluna da grade.
const MAX_COL: usize = 28;

/// Largura de exibição, pela mesma conta que o `ratatui` usa ao desenhar —
/// senão uma coluna com kanji fica mais estreita que o desenhado.
pub(crate) fn largura(texto: &str) -> usize {
    Span::raw(texto).width()
}

/// Largura natural de cada coluna: o nome do cabeçalho inteiro e o conteúdo,
/// cortado em [`MAX_COL`].
///
/// Varre todas as linhas carregadas, então quem chama mede uma vez por mudança
/// de dados — não a cada quadro.
pub(crate) fn larguras(data: &TableData) -> Vec<usize> {
    let mut ideais = Vec::with_capacity(data.columns.len());
    for (index, name) in data.columns.iter().enumerate() {
        let cabecalho = largura(name);
        let conteudo = data.rows.iter().fold(cabecalho, |maior, row| {
            row.get(index)
                .map_or(maior, |value| maior.max(largura(value).min(MAX_COL)))
        });
        ideais.push(conteudo);
    }
    ideais
}
