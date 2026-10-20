//! Pop-up "Comandos": o comando master aberto com `ctrl+p`.
//!
//! A primeira aba é a SQL, que filtra a grade com SQL livre (ex.:
//! `;select COUNT(*) from users`). O `;` inicial marca o modo SQL e já vem
//! preenchido ao abrir; o `ctrl+enter` executa.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::{popup, popup_content_width, popup_rule};
use crate::app::{CommandTab, Commands};
use crate::theme;
use crate::view::largura;

/// Teclas do comando master, como saem no rodapé.
pub(crate) fn commands_keys() -> &'static str {
    "ctrl+enter executar · esc fechar"
}

pub(crate) fn draw_commands(frame: &mut Frame, area: Rect, commands: Option<&Commands>) {
    let Some(commands) = commands else {
        return;
    };
    let conteudo = popup_content_width(area);
    let mut lines = vec![abas(commands.tab)];
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_sql(&commands.input, conteudo));
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_acoes());
    // O ícone (Nerd Font) acompanha o título dos pop-ups.
    popup(frame, area, "\u{f120} Comandos", lines);
}

/// Linha de abas: a ativa em destaque, as outras apagadas. Cada aba mostra a
/// tecla que a abre direto (`SQL ;`).
fn abas(ativa: CommandTab) -> Line<'static> {
    let mut spans = Vec::new();
    for (indice, aba) in CommandTab::ALL.iter().enumerate() {
        if indice > 0 {
            spans.push(Span::styled("  ", theme::muted()));
        }
        let rotulo = format!("{} {}", aba.titulo(), aba.atalho());
        if *aba == ativa {
            spans.push(Span::styled(format!("[{rotulo}]"), theme::accent()));
        } else {
            spans.push(Span::styled(rotulo, theme::muted()));
        }
    }
    Line::from(spans)
}

/// A linha do SQL, com o cursor no fim e a cauda visível quando não cabe.
fn linha_sql(input: &str, comprimento: usize) -> Line<'static> {
    let mut valor = format!("{input}▏");
    if largura(&valor) > comprimento {
        valor = cauda(&valor, comprimento);
    } else {
        valor.push_str(&" ".repeat(comprimento - largura(&valor)));
    }
    Line::from(valor).style(theme::field())
}

/// Fim do texto que cabe em `limite`, marcado com `…` na frente.
fn cauda(valor: &str, limite: usize) -> String {
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

/// Ações do comando master: `ctrl+enter` executa e `esc` fecha.
fn linha_acoes() -> Line<'static> {
    Line::from(vec![
        Span::styled("ctrl+enter", theme::accent()),
        Span::styled(" executar · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" fechar", theme::text()),
    ])
}
