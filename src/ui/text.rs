//! Corte de texto com `…` e realce de SQL; a largura de exibição mora em
//! `crate::view`.

use crate::db::SqlPart;
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

/// Palavras que saem na cor de declaração, como no pop-up "Execução".
const DECLARACOES: [&str; 26] = [
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "LIMIT", "OFFSET", "ORDER", "BY", "GROUP",
    "HAVING", "AS", "WITH", "DISTINCT", "ASC", "DESC", "IN", "IS", "LIKE", "BETWEEN", "JOIN",
    "LEFT", "RIGHT", "ON", "UNION",
];

/// Quebra o SQL em pedaços com o papel de cada um, nas mesmas cores do
/// pop-up "Execução": declaração, literal de texto, número, `NULL` e o
/// resto como texto comum. Aspas e crases valem até o fecho (com escape
/// dobrado); sem fecho, vão até o fim.
pub(super) fn realces(texto: &str) -> Vec<(String, SqlPart)> {
    let mut pedacos: Vec<(String, SqlPart)> = Vec::new();
    let mut plano = String::new();
    let despeja = |pedacos: &mut Vec<(String, SqlPart)>, plano: &mut String| {
        if !plano.is_empty() {
            pedacos.push((std::mem::take(plano), SqlPart::Plain));
        }
    };
    let caracteres: Vec<char> = texto.chars().collect();
    let mut i = 0;
    while i < caracteres.len() {
        let atual = caracteres[i];
        if atual == '\'' || atual == '"' || atual == '`' {
            let mut literal = String::from(atual);
            i += 1;
            while i < caracteres.len() {
                literal.push(caracteres[i]);
                if caracteres[i] == atual {
                    // `''` (e `""`) escapado segue dentro do literal.
                    if atual != '`' && caracteres.get(i + 1) == Some(&atual) {
                        literal.push(atual);
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            despeja(&mut pedacos, &mut plano);
            let papel = if atual == '`' {
                SqlPart::Plain
            } else {
                SqlPart::Text
            };
            pedacos.push((literal, papel));
            continue;
        }
        if atual.is_ascii_digit() {
            let mut numero = String::new();
            while i < caracteres.len() && (caracteres[i].is_ascii_digit() || caracteres[i] == '.') {
                numero.push(caracteres[i]);
                i += 1;
            }
            despeja(&mut pedacos, &mut plano);
            pedacos.push((numero, SqlPart::Number));
            continue;
        }
        if atual.is_ascii_alphabetic() || atual == '_' {
            let mut palavra = String::new();
            while i < caracteres.len()
                && (caracteres[i].is_ascii_alphanumeric()
                    || caracteres[i] == '_'
                    || caracteres[i] == '$')
            {
                palavra.push(caracteres[i]);
                i += 1;
            }
            despeja(&mut pedacos, &mut plano);
            let alta = palavra.to_ascii_uppercase();
            if alta == "NULL" {
                pedacos.push((palavra, SqlPart::Null));
            } else if DECLARACOES.contains(&alta.as_str()) {
                pedacos.push((palavra, SqlPart::Declaration));
            } else {
                pedacos.push((palavra, SqlPart::Plain));
            }
            continue;
        }
        plano.push(atual);
        i += 1;
    }
    despeja(&mut pedacos, &mut plano);
    pedacos
}

#[cfg(test)]
mod tests {
    use super::*;

    fn papeis(pedacos: &[(String, SqlPart)]) -> Vec<(&str, SqlPart)> {
        pedacos
            .iter()
            .map(|(texto, papel)| (texto.as_str(), *papel))
            .collect()
    }

    #[test]
    fn declaracoes_texto_numero_e_nulo_ganham_cor() {
        assert_eq!(
            papeis(&realces("SELECT * FROM `objetos` WHERE cod_empresa = 189")),
            [
                ("SELECT", SqlPart::Declaration),
                (" * ", SqlPart::Plain),
                ("FROM", SqlPart::Declaration),
                (" ", SqlPart::Plain),
                ("`objetos`", SqlPart::Plain),
                (" ", SqlPart::Plain),
                ("WHERE", SqlPart::Declaration),
                (" ", SqlPart::Plain),
                ("cod_empresa", SqlPart::Plain),
                (" = ", SqlPart::Plain),
                ("189", SqlPart::Number),
            ]
        );
    }

    #[test]
    fn literal_de_texto_e_nulo_saem_com_o_papel() {
        assert_eq!(
            papeis(&realces("WHERE nome = 'd''água' AND fim IS NULL")),
            [
                ("WHERE", SqlPart::Declaration),
                (" ", SqlPart::Plain),
                ("nome", SqlPart::Plain),
                (" = ", SqlPart::Plain),
                ("'d''água'", SqlPart::Text),
                (" ", SqlPart::Plain),
                ("AND", SqlPart::Declaration),
                (" ", SqlPart::Plain),
                ("fim", SqlPart::Plain),
                (" ", SqlPart::Plain),
                ("IS", SqlPart::Declaration),
                (" ", SqlPart::Plain),
                ("NULL", SqlPart::Null),
            ]
        );
    }
}
