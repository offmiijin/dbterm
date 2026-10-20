//! Pop-up "Comandos": o comando master aberto com `ctrl+p`, `;` ou `#`.
//!
//! O prefixo fixo seleciona o comando: `;` é SQL, que filtra a grade com
//! SQL livre (ex.: `;select COUNT(*) from users`), e `#` busca uma tabela
//! pelo nome (ex.: `#products`), com a caixa de completar. O `enter`
//! executa e o `tab` troca de aba levando o texto junto.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::{popup, popup_content_width, popup_rule};
use crate::app::{CommandTab, Commands};
use crate::theme;
use crate::view::largura;

/// Teclas do comando master, como saem no rodapé.
pub(crate) fn commands_keys() -> &'static str {
    "ctrl+enter executar · tab aba · ctrl+j/k sugestão · esc fechar"
}

pub(crate) fn draw_commands(
    frame: &mut Frame,
    area: Rect,
    commands: Option<&Commands>,
    tables: &[String],
) {
    let Some(commands) = commands else {
        return;
    };
    let conteudo = popup_content_width(area);
    let mut lines = vec![abas(commands.tab)];
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_comando(&commands.input, conteudo));
    if let Some(linha) = linha_tabela(commands) {
        lines.push(linha);
    }
    lines.extend(linhas_sugestoes(commands, tables));
    if let Some(linha) = linha_navegacao(commands) {
        lines.push(linha);
    }
    lines.push(Line::from(popup_rule(area)).style(theme::muted()));
    lines.push(linha_acoes());
    // O ícone (Nerd Font) acompanha o título dos pop-ups.
    popup(frame, area, "\u{f120} Comandos", lines);
}

/// Linha de abas: a ativa em destaque, as outras apagadas. Cada aba mostra o
/// ícone, o prefixo que a seleciona (` SQL ;`, ` Tabela #`).
fn abas(ativa: CommandTab) -> Line<'static> {
    let mut spans = Vec::new();
    for (indice, aba) in CommandTab::ALL.iter().enumerate() {
        if indice > 0 {
            spans.push(Span::styled("  ", theme::muted()));
        }
        let rotulo = format!("{} {} {}", aba.icone(), aba.titulo(), aba.atalho());
        if *aba == ativa {
            spans.push(Span::styled(format!("[{rotulo}]"), theme::accent()));
        } else {
            spans.push(Span::styled(rotulo, theme::muted()));
        }
    }
    Line::from(spans)
}

/// A linha do comando, com o cursor no fim e a cauda visível quando não cabe.
fn linha_comando(input: &str, comprimento: usize) -> Line<'static> {
    let mut valor = format!("{input}▏");
    if largura(&valor) > comprimento {
        valor = cauda(&valor, comprimento);
    } else {
        valor.push_str(&" ".repeat(comprimento - largura(&valor)));
    }
    Line::from(valor).style(theme::field())
}

/// Tabela aberta em que o SQL busca, fixada ao abrir o pop-up. Sem tabela,
/// avisa. Só aparece na aba SQL; a busca de tabela não precisa dela.
fn linha_tabela(commands: &Commands) -> Option<Line<'static>> {
    if commands.tab != CommandTab::Sql {
        return None;
    }
    match commands.tabela.as_deref() {
        Some(tabela) => Some(Line::from(vec![
            Span::styled("tabela ", theme::muted()),
            Span::styled(tabela.to_owned(), theme::text()),
        ])),
        None => Some(Line::from("sem tabela aberta").style(theme::muted())),
    }
}

/// Dica de navegação da caixa de completar. Só na aba Tabela.
fn linha_navegacao(commands: &Commands) -> Option<Line<'static>> {
    if commands.tab != CommandTab::Table {
        return None;
    }
    Some(Line::from("ctrl+j desce · ctrl+k sobe na caixa").style(theme::muted()))
}

/// A caixa de completar da busca de tabela: as que combinam com o que foi
/// digitado, com a destacada (`ctrl+j`/`ctrl+k`, `enter` abre ela) em evidência.
fn linhas_sugestoes(commands: &Commands, tables: &[String]) -> Vec<Line<'static>> {
    if commands.tab != CommandTab::Table {
        return Vec::new();
    }
    let sugestoes = commands.suggestions(tables);
    if sugestoes.is_empty() {
        return vec![Line::from("nenhuma tabela encontrada").style(theme::muted())];
    }
    sugestoes
        .iter()
        .enumerate()
        .map(|(posicao, &indice)| {
            let nome = tables.get(indice).cloned().unwrap_or_default();
            if posicao == commands.sugestao {
                Line::from(Span::styled(format!("❯ {nome}"), theme::selected()))
            } else {
                Line::from(format!("  {nome}"))
            }
        })
        .collect()
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

/// Ações do comando master: `ctrl+enter` executa, `tab` troca de aba e `esc`
/// fecha.
fn linha_acoes() -> Line<'static> {
    Line::from(vec![
        Span::styled("ctrl+enter", theme::accent()),
        Span::styled(" executar · ", theme::text()),
        Span::styled("tab", theme::accent()),
        Span::styled(" aba · ", theme::text()),
        Span::styled("esc", theme::accent()),
        Span::styled(" fechar", theme::text()),
    ])
}
