//! A grade de dados: a `Table` com a divisória entre as colunas.
//!
//! Cada coluna de dados é seguida por uma célula de divisória, então a `Table`
//! vê as colunas de dados nos índices 0, 2, 4, ... e as divisórias em 1, 3, 5, ...
//! O resto do programa fala só de índices de dados; a dobra fica em
//! [`table_column`].

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Cell, HighlightSpacing, Paragraph, Row, Table, TableState};

use super::panel;
use super::text::{fit, largura};
use crate::app::Mode;
use crate::db::TableData;
use crate::theme;

/// Largura máxima de uma coluna da grade.
const MAX_COL: usize = 28;
/// Linhas que o painel come: duas bordas e o cabeçalho da grade.
const BORDAS: u16 = 2;
const HEADER: u16 = 1;
/// Respiro de cada lado da linha divisória entre colunas.
const GAP: u16 = 2;
/// O traço que separa duas colunas.
pub(super) const RULE: &str = "│";
/// Largura de [`RULE`] em colunas do terminal.
const RULE_WIDTH: u16 = 1;
/// De quanto em quanto o índice do widget pula para chegar na próxima coluna.
const RULE_STRIDE: usize = 2;

/// Onde a grade está na horizontal e quanto mede cada coluna.
pub(super) struct Posicao<'a> {
    pub coluna: Option<usize>,
    pub rolagem: usize,
    /// Largura natural de cada coluna, medida pela app quando os dados mudam.
    pub larguras: &'a [usize],
}

/// Índice da coluna de dados `index` no widget `Table`.
fn table_column(index: usize) -> usize {
    index * RULE_STRIDE
}

pub(super) fn draw_data(
    frame: &mut Frame,
    area: Rect,
    data: Option<&TableData>,
    focused: bool,
    state: &mut TableState,
    posicao: Posicao,
    mode: &Mode,
) {
    let Some(data) = data else {
        let hint = Paragraph::new("selecione uma tabela e pressione enter")
            .block(panel(String::from("dados"), focused))
            .style(theme::muted());
        frame.render_widget(hint, area);
        return;
    };

    // Medir as colunas é varrer todas as linhas carregadas, então a app mede uma
    // vez por mudança de dados; só sem a medida pronta é que se mede aqui.
    let medidas = (posicao.larguras.len() != data.columns.len()).then(|| larguras(data));
    let ideais = medidas.as_deref().unwrap_or(posicao.larguras);
    let column = posicao.coluna;
    let janela = janela(data, area.width, posicao.rolagem, column, ideais);
    let coluna = column.and_then(|index| data.columns.get(index));
    let mut title = match (mode, coluna) {
        (Mode::Columns | Mode::Edit | Mode::Confirm | Mode::ConfirmAgain, Some(nome)) => {
            format!("dados · {} linhas · col: {nome}", data.rows.len())
        }
        _ => format!("dados · {} linhas", data.rows.len()),
    };
    if janela.inicio > 0 || janela.fim() < data.columns.len() {
        // Nem todas as colunas estão na tela: diz quais estão.
        title.push_str(&format!(
            " · colunas {}-{}/{}",
            janela.inicio + 1,
            janela.fim(),
            data.columns.len()
        ));
    }

    let header = Row::new(header_cells(data, &janela)).style(theme::header());
    // Só as linhas que cabem: montar `Row`s das milhares de linhas carregadas a
    // cada quadro custa mais que desenhar.
    let altura = area.height.saturating_sub(BORDAS + HEADER) as usize;
    let (primeira, ultima) =
        linhas_visiveis(state.offset(), state.selected(), altura, data.rows.len());
    let rows: Vec<Row> = data.rows[primeira..ultima]
        .iter()
        .map(|row| Row::new(cells(row, &janela)))
        .collect();

    let table = Table::new(rows, constraints(&janela.larguras))
        .header(header)
        .block(panel(title, focused))
        .column_spacing(0)
        .highlight_symbol("❯ ")
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(theme::selected())
        .cell_highlight_style(theme::cell_selected());
    // O widget recebe um estado descartável: ele escreve nele a rolagem e a
    // coluna em relação às linhas e colunas que vê, que começam na janela. O
    // estado de verdade guarda só a linha e a coluna selecionadas — e a rolagem
    // vertical, que é nossa. O índice da coluna é dobrado porque o widget vê uma
    // divisória entre cada par de colunas.
    let mut grade = TableState::default();
    grade.select(state.selected().map(|linha| linha.saturating_sub(primeira)));
    grade.select_column(
        column
            .and_then(|index| index.checked_sub(janela.inicio))
            .filter(|&visivel| visivel < janela.larguras.len())
            .map(table_column),
    );
    frame.render_stateful_widget(table, area, &mut grade);
    *state.offset_mut() = primeira;
}

/// A linha vertical que separa duas colunas.
fn rule() -> Cell<'static> {
    Cell::new(RULE).style(theme::muted())
}

/// Cabeçalho com a divisória entre as colunas.
fn header_cells<'a>(data: &'a TableData, janela: &Janela) -> Vec<Cell<'a>> {
    let mut cells = Vec::with_capacity(janela.larguras.len() * RULE_STRIDE);
    for (visivel, largura) in janela.larguras.iter().enumerate() {
        if visivel > 0 {
            cells.push(rule());
        }
        let name = &data.columns[janela.inicio + visivel];
        cells.push(match fit(name, *largura) {
            Some(cortado) => Cell::new(cortado),
            None => Cell::new(name.as_str()),
        });
    }
    cells
}

/// Linha de dados com a divisória entre as colunas; `NULL` sai em cinza.
fn cells<'a>(row: &'a [String], janela: &Janela) -> Vec<Cell<'a>> {
    let mut cells = Vec::with_capacity(janela.larguras.len() * RULE_STRIDE);
    for (visivel, largura) in janela.larguras.iter().enumerate() {
        if visivel > 0 {
            cells.push(rule());
        }
        let value = row.get(janela.inicio + visivel).map_or("", String::as_str);
        let cell = match fit(value, *largura) {
            Some(cortado) => Cell::new(cortado),
            None => Cell::new(value),
        };
        cells.push(if value == "NULL" {
            cell.style(theme::muted())
        } else {
            cell
        });
    }
    cells
}

/// O recorte de colunas que a grade desenha.
struct Janela {
    /// Índice da primeira coluna de dados visível.
    inicio: usize,
    /// Largura de cada coluna visível, da esquerda para a direita.
    larguras: Vec<usize>,
}

impl Janela {
    /// Índice (exclusivo) da coluna seguinte à última visível.
    fn fim(&self) -> usize {
        self.inicio + self.larguras.len()
    }
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

/// A largura mínima de cada coluna: o nome do cabeçalho inteiro.
fn minimas(data: &TableData) -> Vec<usize> {
    data.columns.iter().map(|name| largura(name)).collect()
}

/// Espaço que sobra para as colunas de dados: da área saem as bordas do painel,
/// o `❯ ` da seleção, o respiro e as divisórias entre as colunas.
fn orcamento(area: u16, colunas: usize) -> usize {
    let divisoes = colunas.saturating_sub(1);
    (area as usize).saturating_sub(2 + 2 + colunas * GAP as usize + divisoes * RULE_WIDTH as usize)
}

/// Quantas colunas de `larguras` entram em `area` a partir de `inicio`.
///
/// Uma coluna sozinha é sempre aceita, mesmo larga demais: quem a corta depois
/// é [`encaixar`].
fn cabem(larguras: &[usize], area: u16, inicio: usize) -> usize {
    let disponivel = (area as usize).saturating_sub(2 + 2);
    let mut usadas = 0;
    for (quantas, largura) in larguras[inicio..].iter().enumerate() {
        let total =
            usadas + largura + GAP as usize + if quantas == 0 { 0 } else { RULE_WIDTH as usize };
        if quantas > 0 && total > disponivel {
            return quantas;
        }
        usadas = total;
    }
    larguras.len() - inicio
}

/// O maior recorte que termina em `fim` (exclusive) e ainda cabe em `area`.
fn recorte_ate(larguras: &[usize], area: u16, fim: usize) -> usize {
    let disponivel = (area as usize).saturating_sub(2 + 2);
    let mut usadas = 0;
    let mut inicio = fim;
    for (index, largura) in larguras[..fim].iter().enumerate().rev() {
        // `inicio == fim` só na primeira volta: a coluna mais à direita sempre
        // entra, mesmo larga demais; quem a corta é [`encaixar`].
        let total = usadas
            + largura
            + GAP as usize
            + if inicio == fim {
                0
            } else {
                RULE_WIDTH as usize
            };
        if inicio < fim && total > disponivel {
            break;
        }
        usadas = total;
        inicio = index;
    }
    inicio
}

/// Encolhe as colunas mais largas até caberem em `area`, sem descer abaixo do
/// nome do cabeçalho. Só numa coluna maior que a tela inteira o nome é cortado
/// com `…`, porque aí não há para onde rolar.
fn encaixar(ideais: &[usize], minimas: &[usize], area: u16) -> Vec<usize> {
    let mut larguras = ideais.to_vec();
    let orcamento = orcamento(area, larguras.len());
    while larguras.iter().sum::<usize>() > orcamento {
        let cabe_cabecalho = (0..larguras.len())
            .filter(|&index| larguras[index] > minimas[index])
            .max_by_key(|&index| larguras[index]);
        let cabe_um = (0..larguras.len())
            .filter(|&index| larguras[index] > 1)
            .max_by_key(|&index| larguras[index]);
        let Some(index) = cabe_cabecalho.or(cabe_um) else {
            break;
        };
        larguras[index] -= 1;
    }
    larguras
}

/// O recorte de colunas da grade, com as larguras já ajustadas à `area`.
///
/// Quando os nomes dos cabeçalhos não cabem todos, a grade rola na horizontal —
/// `scroll` é a primeira coluna pedida — em vez de espremer o cabeçalho até uma
/// letra. A coluna `column`, quando há, sempre fica visível.
fn janela(
    data: &TableData,
    area: u16,
    scroll: usize,
    column: Option<usize>,
    ideais: &[usize],
) -> Janela {
    let total = data.columns.len();
    if total == 0 {
        return Janela {
            inicio: 0,
            larguras: Vec::new(),
        };
    }
    let minimas = minimas(data);
    // Todos os nomes cabem: basta cortar o conteúdo das colunas mais largas.
    if minimas.iter().sum::<usize>() <= orcamento(area, total) {
        return Janela {
            inicio: 0,
            larguras: encaixar(ideais, &minimas, area),
        };
    }

    let mut inicio = scroll.min(total - 1);
    if let Some(selecionada) = column {
        let visiveis = cabem(ideais, area, inicio);
        if selecionada < inicio {
            inicio = selecionada;
        } else if selecionada >= inicio + visiveis {
            // A seleção não coube à direita: ela vira a última coluna da tela.
            inicio = recorte_ate(ideais, area, selecionada + 1);
        }
    }
    let fim = inicio + cabem(ideais, area, inicio);
    Janela {
        inicio,
        larguras: encaixar(&ideais[inicio..fim], &minimas[inicio..fim], area),
    }
}

/// As linhas que a grade desenha: a janela de `altura` linhas em torno da
/// seleção, ancorada na `rolagem` que ficou do quadro anterior.
///
/// Mesma regra do widget (que só recebe a janela): a seleção nunca sai da tela —
/// vira a última linha quando passa do fim e a primeira quando fica acima.
pub(super) fn linhas_visiveis(
    rolagem: usize,
    selecionada: Option<usize>,
    altura: usize,
    total: usize,
) -> (usize, usize) {
    if total == 0 || altura == 0 {
        return (0, 0);
    }
    let ultima = total - 1;
    let mut primeira = rolagem.min(ultima);
    if let Some(selecionada) = selecionada {
        primeira = primeira.min(selecionada.min(ultima));
    }
    let mut fim = (primeira + altura).min(total);
    if let Some(selecionada) = selecionada {
        let selecionada = selecionada.min(ultima);
        if selecionada >= fim {
            primeira = (selecionada + 1).saturating_sub(altura);
            fim = (primeira + altura).min(total);
        }
    }
    (primeira, fim)
}

/// Constraints do `Table`: coluna (com o respiro antes da divisória), divisória,
/// coluna, ... A divisória fica colada no início da coluna seguinte.
fn constraints(colunas: &[usize]) -> Vec<Constraint> {
    let mut widths = Vec::with_capacity(colunas.len() * RULE_STRIDE);
    for (index, largura) in colunas.iter().enumerate() {
        if index > 0 {
            widths.push(Constraint::Length(RULE_WIDTH));
        }
        widths.push(Constraint::Length(*largura as u16 + GAP));
    }
    widths
}
