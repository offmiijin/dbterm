//! Tema visual do dbterm.
//!
//! Cores da paleta oficial do Catppuccin, variante Macchiato
//! (<https://catppuccin.com/palette>). As constantes batem com os nomes da
//! paleta; as funções abaixo dão o papel de cada cor na tela, para o `ui` não
//! espalhar hexadecimais soltos.

use ratatui::style::{Color, Modifier, Style};

/// Cores nomeadas da paleta Macchiato.
pub const ROSEWATER: Color = Color::Rgb(0xf4, 0xdb, 0xd6);
pub const FLAMINGO: Color = Color::Rgb(0xf0, 0xc6, 0xc6);
pub const PINK: Color = Color::Rgb(0xf5, 0xbd, 0xe6);
pub const MAUVE: Color = Color::Rgb(0xc6, 0xa0, 0xf6);
pub const RED: Color = Color::Rgb(0xed, 0x87, 0x96);
pub const MAROON: Color = Color::Rgb(0xee, 0x99, 0xa0);
pub const PEACH: Color = Color::Rgb(0xf5, 0xa9, 0x7f);
pub const YELLOW: Color = Color::Rgb(0xee, 0xd4, 0x9f);
pub const GREEN: Color = Color::Rgb(0xa6, 0xda, 0x95);
pub const TEAL: Color = Color::Rgb(0x8b, 0xd5, 0xca);
pub const SKY: Color = Color::Rgb(0x91, 0xd7, 0xe3);
pub const SAPPHIRE: Color = Color::Rgb(0x7d, 0xc4, 0xe4);
pub const BLUE: Color = Color::Rgb(0x8a, 0xad, 0xf4);
pub const LAVENDER: Color = Color::Rgb(0xb7, 0xbd, 0xf8);
pub const TEXT: Color = Color::Rgb(0xca, 0xd3, 0xf5);
pub const SUBTEXT1: Color = Color::Rgb(0xb8, 0xc0, 0xe0);
pub const SUBTEXT0: Color = Color::Rgb(0xa5, 0xad, 0xcb);
pub const OVERLAY2: Color = Color::Rgb(0x93, 0x9a, 0xb7);
pub const OVERLAY1: Color = Color::Rgb(0x80, 0x87, 0xa2);
pub const OVERLAY0: Color = Color::Rgb(0x6e, 0x73, 0x8d);
pub const SURFACE2: Color = Color::Rgb(0x5b, 0x60, 0x78);
pub const SURFACE1: Color = Color::Rgb(0x49, 0x4d, 0x64);
pub const SURFACE0: Color = Color::Rgb(0x36, 0x3a, 0x4f);
pub const BASE: Color = Color::Rgb(0x24, 0x27, 0x3a);
pub const MANTLE: Color = Color::Rgb(0x1e, 0x20, 0x30);
pub const CRUST: Color = Color::Rgb(0x18, 0x19, 0x26);

/// Fundo e texto padrão da tela inteira.
pub fn base() -> Style {
    Style::new().fg(TEXT).bg(BASE)
}

/// Marca do app e destaque de foco.
pub fn accent() -> Style {
    Style::new().fg(MAUVE).add_modifier(Modifier::BOLD)
}

/// Texto normal.
pub fn text() -> Style {
    Style::new().fg(TEXT)
}

/// Texto secundário: separadores, dicas, `NULL` e atalhos.
pub fn muted() -> Style {
    Style::new().fg(OVERLAY0)
}

/// Cabeçalho de coluna da grade.
pub fn header() -> Style {
    Style::new().fg(LAVENDER).add_modifier(Modifier::BOLD)
}

/// Item ou linha selecionada: só troca o fundo, para não sobrescrever as
/// cores do conteúdo (o `Table` aplica o destaque depois das células).
pub fn selected() -> Style {
    Style::new().bg(SURFACE0)
}

/// Célula selecionada no modo coluna.
pub fn cell_selected() -> Style {
    Style::new().fg(TEXT).bg(SURFACE1)
}

/// Fundo do campo em edição.
pub fn field() -> Style {
    Style::new().fg(TEXT).bg(SURFACE0)
}

/// Literal de texto (`'...'`) no preview de SQL.
pub fn string() -> Style {
    Style::new().fg(GREEN)
}

/// Literal numérico no preview de SQL.
pub fn number() -> Style {
    Style::new().fg(PEACH)
}

/// Borda de painel, acesa quando o painel tem o foco.
pub fn border(focused: bool) -> Style {
    Style::new().fg(if focused { MAUVE } else { SURFACE1 })
}

/// Avisos (conectando, carregando).
pub fn warning() -> Style {
    Style::new().fg(YELLOW)
}

/// Ações irreversíveis.
pub fn danger() -> Style {
    Style::new().fg(RED)
}

/// Erros.
pub fn error() -> Style {
    Style::new().fg(RED)
}
