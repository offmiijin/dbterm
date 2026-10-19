//! Tela inicial: as conexões rápidas salvas, em blocos empilhados.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};

use super::banner;
use crate::app::App;
use crate::history::Saved;
use crate::theme;

/// Altura de um bloco: borda de cima, duas linhas de conteúdo e borda de baixo.
const BLOCO: u16 = 4;
/// Linha em branco entre blocos.
const INTERVALO: u16 = 1;
/// Largura máxima da coluna dos blocos.
const COLUNA: u16 = 52;

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let recentes = app.history.recent();
    let blocos = if recentes.is_empty() {
        1
    } else {
        recentes.len() as u16 * BLOCO + (recentes.len() as u16 - 1) * INTERVALO
    };
    let marca = banner(area.width).len() as u16;
    // marca + respiro + título + respiro + blocos + respiro + dica.
    let altura = marca + blocos + 5;

    let [_, corpo, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(altura),
        Constraint::Fill(1),
    ])
    .areas(area);

    let areas = Layout::vertical([
        Constraint::Length(marca),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(blocos),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas::<7>(corpo);

    frame.render_widget(Paragraph::new(banner(area.width)), areas[0]);
    frame.render_widget(
        Paragraph::new(
            Line::from("Conexões Rápidas")
                .style(theme::header())
                .centered(),
        ),
        areas[2],
    );

    let [_, cartoes, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(corpo.width.min(COLUNA)),
        Constraint::Fill(1),
    ])
    .areas(areas[4]);

    if recentes.is_empty() {
        frame.render_widget(
            Paragraph::new(
                Line::from("nenhuma conexão salva")
                    .style(theme::muted())
                    .centered(),
            ),
            cartoes,
        );
        frame.render_widget(Paragraph::new(dica(false).centered()), areas[6]);
        return;
    }

    for (indice, saved) in recentes.iter().enumerate() {
        let deslocamento = indice as u16 * (BLOCO + INTERVALO);
        let altura = BLOCO.min(cartoes.height.saturating_sub(deslocamento));
        if altura == 0 {
            break;
        }
        let area = Rect::new(cartoes.x, cartoes.y + deslocamento, cartoes.width, altura);
        bloco(frame, area, saved, app.quick == Some(indice));
    }
    frame.render_widget(Paragraph::new(dica(true).centered()), areas[6]);
}

/// Uma conexão: motor e endereço na primeira linha, usuário e schema na
/// segunda. Nada sensível é mostrado.
fn bloco(frame: &mut Frame, area: Rect, saved: &Saved, selecionada: bool) {
    let moldura = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border(selecionada))
        .padding(Padding::horizontal(1));
    let marcador = if selecionada { "❯ " } else { "  " };
    let linhas = vec![
        Line::from(vec![
            Span::styled(marcador, theme::accent()),
            Span::styled(engine_label(&saved.engine), theme::accent()),
            Span::styled(" · ", theme::muted()),
            Span::styled(format!("{}:{}", saved.host, saved.port), theme::text()),
        ]),
        Line::from(vec![
            Span::styled("  ", theme::text()),
            Span::styled(saved.user.clone(), theme::text()),
            Span::styled(" @ ", theme::muted()),
            Span::styled(saved.database.clone(), theme::text()),
        ]),
    ];
    let interior = moldura.inner(area);
    frame.render_widget(moldura, area);
    frame.render_widget(Paragraph::new(linhas), interior);
}

/// Nome de exibição do motor guardado na conexão.
fn engine_label(engine: &str) -> String {
    match engine {
        Saved::MYSQL => String::from("MySQL"),
        outro => outro.to_uppercase(),
    }
}

/// Rodapé: as teclas da tela.
fn dica(com_conexoes: bool) -> Line<'static> {
    let texto = if com_conexoes {
        "enter abrir · j/k mover · n nova conexão · esc sair"
    } else {
        "n nova conexão · esc sair"
    };
    Line::from(texto).style(theme::muted())
}
