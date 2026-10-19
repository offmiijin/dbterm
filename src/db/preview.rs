//! O texto do `UPDATE` que os pop-ups mostram, em pedaços já classificados
//! para o `ui` escolher a cor. Não fala com o banco.

use super::{ColumnKind, TableData};

/// Papel de um pedaço da query de preview, para a UI escolher a cor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlPart {
    /// Declaração: `UPDATE`, `SET`, `WHERE`.
    Declaration,
    /// Operador, pontuação e nomes de tabela/coluna.
    Plain,
    /// Literal de texto, já entre aspas.
    Text,
    /// Literal numérico, sem aspas.
    Number,
    /// `NULL`.
    Null,
}

/// `UPDATE` da edição em pedaços classificados: as linhas do padrão
/// `UPDATE` / `SET` / `WHERE ...;`, com os valores no lugar dos `?`, sem crases
/// nos identificadores e cada literal marcado pelo tipo da coluna.
pub fn update_preview(
    table: &str,
    data: &TableData,
    row: usize,
    column: usize,
    value: Option<&str>,
) -> Vec<Vec<(String, SqlPart)>> {
    let Some(name) = data.columns.get(column) else {
        return Vec::new();
    };
    let update = vec![
        (String::from("UPDATE"), SqlPart::Declaration),
        (format!(" {table}"), SqlPart::Plain),
    ];
    let set = vec![
        (String::from("SET"), SqlPart::Declaration),
        (format!(" {name} = "), SqlPart::Plain),
        literal(value, data.kind(column)),
    ];

    let mut where_ = vec![(String::from("WHERE"), SqlPart::Declaration)];
    for (index, name, value) in data.key_columns(row) {
        // A primeira coluna da PK ganha um espaço; as demais, um `AND`.
        let separator = if where_.len() == 1 { " " } else { " AND " };
        where_.push((format!("{separator}{name} = "), SqlPart::Plain));
        where_.push(literal(Some(value), data.kind(index)));
    }
    where_.push((String::from(";"), SqlPart::Plain));

    vec![update, set, where_]
}

/// Um literal do preview, no formato do tipo da coluna.
fn literal(value: Option<&str>, kind: ColumnKind) -> (String, SqlPart) {
    match value {
        None => (String::from("NULL"), SqlPart::Null),
        Some(value) if kind == ColumnKind::Number => (value.to_owned(), SqlPart::Number),
        Some(value) => (format!("'{}'", value.replace('\'', "''")), SqlPart::Text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A query de preview sem as marcações: o texto que aparece na tela.
    fn texto(linhas: &[Vec<(String, SqlPart)>]) -> String {
        linhas
            .iter()
            .map(|linha| {
                linha
                    .iter()
                    .map(|(texto, _)| texto.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn rascunho_do_update_mostra_valores_condicao_e_escapes() {
        let data = TableData {
            columns: vec![String::from("id"), String::from("name")],
            kinds: vec![ColumnKind::Number, ColumnKind::Text],
            rows: vec![vec![String::from("1"), String::from("ana")]],
            primary_key: vec![0],
        };
        assert_eq!(
            texto(&update_preview("users", &data, 0, 1, Some("bia"))),
            "UPDATE users\nSET name = 'bia'\nWHERE id = 1;"
        );
        assert_eq!(
            texto(&update_preview("users", &data, 0, 1, None)),
            "UPDATE users\nSET name = NULL\nWHERE id = 1;"
        );
        assert_eq!(
            texto(&update_preview("t", &data, 0, 1, Some("d'água"))),
            "UPDATE t\nSET name = 'd''água'\nWHERE id = 1;"
        );
        // Linha fora da página: sem chave, a condição fica vazia.
        assert!(data.key_of(9).is_empty());
    }

    #[test]
    fn literais_do_preview_seguem_o_tipo_da_coluna() {
        let data = TableData {
            columns: vec![
                String::from("id"),
                String::from("name"),
                String::from("price"),
            ],
            kinds: vec![ColumnKind::Number, ColumnKind::Text, ColumnKind::Number],
            rows: vec![vec![
                String::from("7"),
                String::from("teclado"),
                String::from("10.5"),
            ]],
            primary_key: vec![0],
        };
        let preview = update_preview("products", &data, 0, 1, Some("Teclado mecânico"));
        assert_eq!(
            preview[1],
            vec![
                (String::from("SET"), SqlPart::Declaration),
                (String::from(" name = "), SqlPart::Plain),
                (String::from("'Teclado mecânico'"), SqlPart::Text),
            ]
        );

        let numerico = update_preview("products", &data, 0, 2, Some("12.9"));
        assert_eq!(
            numerico[1],
            vec![
                (String::from("SET"), SqlPart::Declaration),
                (String::from(" price = "), SqlPart::Plain),
                (String::from("12.9"), SqlPart::Number),
            ]
        );
        // A chave primária numérica também sai sem aspas.
        assert_eq!(
            numerico[2],
            vec![
                (String::from("WHERE"), SqlPart::Declaration),
                (String::from(" id = "), SqlPart::Plain),
                (String::from("7"), SqlPart::Number),
                (String::from(";"), SqlPart::Plain),
            ]
        );
    }
}
