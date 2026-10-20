//! Pop-ups de edição e das duas confirmações, com o preview do SQL.
//!
//! As caixas de cada pop-up moram nos submódulos (`connection`, `database`,
//! `table`, `picker`, `confirm`); os primitivos compartilhados — caixa
//! centralizada, campo rotulado e tamanho legível — ficam aqui.

mod commands;
mod confirm;
mod connection;
mod database;
mod picker;
mod table;

pub(super) use commands::{commands_keys, draw_commands};
pub(super) use confirm::{draw_overlays, parte_estilo};
pub(super) use connection::draw_connection_edit;
pub(super) use database::draw_database_edit;
pub(super) use picker::draw_picker;
#[cfg(test)]
pub(super) use picker::janela;
pub(super) use table::{draw_table_edit, table_keys};

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Padding, Paragraph};

use crate::theme;
use crate::view::largura;

/// Largura do rótulo de cada campo do pop-up de conexão.
pub(super) const CAMPO_ROTULO: usize = 8;
/// Largura do rótulo dos campos de banco e tabela ("Auto-increment" manda).
pub(super) const EDIT_ROTULO: usize = 15;
/// Respiro entre o rótulo e o valor e entre dois campos na mesma linha.
pub(super) const CAMPO_ESPACO: usize = 2;
/// Linhas da caixa do comentário da tabela: a linha do rótulo e mais duas.
pub(super) const COMENTARIO_LINHAS: usize = 3;

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
pub(super) fn campo(
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

/// Largura útil do popup (descontando as bordas laterais).
fn popup_width(area: Rect) -> u16 {
    area.width.saturating_sub(4).min(64)
}

/// Régua horizontal do popup, do tamanho da área interna (após o respiro).
pub(super) fn popup_rule(area: Rect) -> String {
    "─".repeat(popup_content_width(area))
}

/// Largura da área interna do popup (bordas e respiro da esquerda descontados).
pub(super) fn popup_content_width(area: Rect) -> usize {
    popup_width(area).saturating_sub(3) as usize
}

/// Caixa centralizada que cobre o conteúdo por baixo.
pub(super) fn popup(frame: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
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
