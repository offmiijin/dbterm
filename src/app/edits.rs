//! Comportamento dos pop-ups de banco e tabela: campos, listas e `ALTER`.

use crate::config::{DatabaseField, TableField};
use crate::db::{Collation, ROW_FORMATS, TableAlter};

use super::state::{DatabaseEdit, TableEdit};

impl DatabaseEdit {
    pub fn new(info: crate::db::DatabaseInfo) -> Self {
        Self {
            original: info.clone(),
            values: info,
            field: DatabaseField::Charset,
            picker: None,
        }
    }

    /// Campos do pop-up na ordem de tabulação: os que o servidor informa.
    pub fn fields(&self) -> Vec<DatabaseField> {
        DatabaseField::ALL
            .into_iter()
            .filter(|field| self.available(*field))
            .collect()
    }

    /// O servidor informa este campo? A opção de somente leitura só existe da
    /// 8.0.22 em diante.
    pub fn available(&self, field: DatabaseField) -> bool {
        match field {
            DatabaseField::Charset | DatabaseField::Collation => true,
            DatabaseField::ReadOnly => self.values.read_only.is_some(),
        }
    }

    /// Valor do campo como aparece no pop-up.
    pub fn value(&self, field: DatabaseField) -> String {
        match field {
            DatabaseField::Charset => self.values.charset.clone(),
            DatabaseField::Collation => self.values.collation.clone(),
            DatabaseField::ReadOnly => sim_nao(self.values.read_only),
        }
    }

    /// Collations que combinam com o charset escolhido.
    pub fn collations(&self) -> impl Iterator<Item = &Collation> {
        self.values
            .collations
            .iter()
            .filter(|collation| collation.charset == self.values.charset)
    }

    /// Opções do campo, na ordem em que aparecem na lista.
    pub fn options(&self, field: DatabaseField) -> Vec<String> {
        match field {
            DatabaseField::Charset => self
                .values
                .charsets
                .iter()
                .map(|charset| charset.name.clone())
                .collect(),
            DatabaseField::Collation => self
                .collations()
                .map(|collation| collation.name.clone())
                .collect(),
            DatabaseField::ReadOnly => Vec::new(),
        }
    }

    /// Espaço no campo com foco: abre a lista do campo ou, no sim/não, alterna.
    pub fn activate(&mut self) {
        match self.field {
            DatabaseField::ReadOnly => {
                self.values.read_only = self.values.read_only.map(|valor| !valor);
            }
            campo => {
                let items = self.options(campo);
                if items.is_empty() {
                    return;
                }
                // A lista abre no valor atual, que pode estar em qualquer ponto.
                let atual = self.value(campo);
                let index = items.iter().position(|item| *item == atual).unwrap_or(0);
                self.picker = Some(super::state::Picker {
                    field: campo,
                    items,
                    index,
                });
            }
        }
    }

    /// Aplica na escolha da lista o valor do campo.
    pub fn choose(&mut self, field: DatabaseField, item: String) {
        match field {
            DatabaseField::Charset => {
                self.values.charset = item;
                // A collation acompanha o charset: vai para a padrão dele.
                if let Some(charset) = self
                    .values
                    .charsets
                    .iter()
                    .find(|charset| charset.name == self.values.charset)
                {
                    self.values.collation = charset.default_collation.clone();
                }
            }
            DatabaseField::Collation => self.values.collation = item,
            DatabaseField::ReadOnly => {}
        }
    }
}

/// "sim"/"não" das opções; "—" quando o servidor não informa.
fn sim_nao(valor: Option<bool>) -> String {
    match valor {
        Some(true) => String::from("sim"),
        Some(false) => String::from("não"),
        None => String::from("—"),
    }
}

impl TableEdit {
    pub fn new(info: crate::db::TableInfo) -> Self {
        let auto_increment = info
            .auto_increment
            .map(|valor| valor.to_string())
            .unwrap_or_default();
        Self {
            original: info.clone(),
            values: info,
            auto_increment,
            field: TableField::Name,
            picker: None,
        }
    }

    /// Campos do pop-up na ordem de tabulação: os que o servidor informa.
    pub fn fields(&self) -> Vec<TableField> {
        TableField::ALL
            .into_iter()
            .filter(|field| self.available(*field))
            .collect()
    }

    /// O servidor informa este campo? O auto-increment só existe quando a
    /// tabela tem uma coluna `AUTO_INCREMENT`.
    pub fn available(&self, field: TableField) -> bool {
        match field {
            TableField::AutoIncrement => self.original.auto_increment.is_some(),
            _ => true,
        }
    }

    /// Valor do campo como aparece no pop-up.
    pub fn value(&self, field: TableField) -> String {
        match field {
            TableField::Name => self.values.name.clone(),
            TableField::Comment => self.values.comment.clone(),
            TableField::Engine => self.values.engine.clone(),
            TableField::RowFormat => self.values.row_format.clone(),
            TableField::AutoIncrement => self.auto_increment.clone(),
            TableField::Charset => self.values.charset.clone(),
            TableField::Collation => self.values.collation.clone(),
        }
    }

    /// Collations que combinam com o charset escolhido.
    pub fn collations(&self) -> impl Iterator<Item = &Collation> {
        self.values
            .collations
            .iter()
            .filter(|collation| collation.charset == self.values.charset)
    }

    /// Opções do campo, na ordem em que aparecem na lista.
    pub fn options(&self, field: TableField) -> Vec<String> {
        match field {
            TableField::Engine => self.values.engines.clone(),
            TableField::RowFormat => ROW_FORMATS.iter().map(|nome| (*nome).to_owned()).collect(),
            TableField::Charset => self
                .values
                .charsets
                .iter()
                .map(|charset| charset.name.clone())
                .collect(),
            TableField::Collation => self
                .collations()
                .map(|collation| collation.name.clone())
                .collect(),
            TableField::Name | TableField::Comment | TableField::AutoIncrement => Vec::new(),
        }
    }

    /// Enter no campo com foco: abre a lista, que já sai no valor atual.
    pub fn activate(&mut self) {
        let campo = self.field;
        if !campo.dropdown() {
            return;
        }
        let items = self.options(campo);
        if items.is_empty() {
            return;
        }
        let atual = self.value(campo);
        let index = items.iter().position(|item| *item == atual).unwrap_or(0);
        self.picker = Some(super::state::Picker {
            field: campo,
            items,
            index,
        });
    }

    /// Aplica na escolha da lista o valor do campo.
    pub fn choose(&mut self, field: TableField, item: String) {
        match field {
            TableField::Engine => self.values.engine = item,
            TableField::RowFormat => self.values.row_format = item,
            TableField::Charset => {
                self.values.charset = item;
                // A collation acompanha o charset: vai para a padrão dele.
                if let Some(charset) = self
                    .values
                    .charsets
                    .iter()
                    .find(|charset| charset.name == self.values.charset)
                {
                    self.values.collation = charset.default_collation.clone();
                }
            }
            TableField::Collation => self.values.collation = item,
            TableField::Name | TableField::Comment | TableField::AutoIncrement => {}
        }
    }

    /// Digita no campo de texto com foco.
    pub fn type_char(&mut self, caractere: char) {
        match self.field {
            TableField::Name => self.values.name.push(caractere),
            TableField::Comment => self.values.comment.push(caractere),
            TableField::AutoIncrement => self.auto_increment.push(caractere),
            _ => {}
        }
    }

    /// Apaga o último caractere do campo de texto com foco.
    pub fn backspace(&mut self) {
        match self.field {
            TableField::Name => {
                self.values.name.pop();
            }
            TableField::Comment => {
                self.values.comment.pop();
            }
            TableField::AutoIncrement => {
                self.auto_increment.pop();
            }
            _ => {}
        }
    }

    /// Apaga a palavra anterior (`alt+backspace`) do campo de texto com foco.
    pub fn delete_word(&mut self) {
        use super::state::apagar_palavra_anterior;
        match self.field {
            TableField::Name => apagar_palavra_anterior(&mut self.values.name, 0),
            TableField::Comment => apagar_palavra_anterior(&mut self.values.comment, 0),
            TableField::AutoIncrement => apagar_palavra_anterior(&mut self.auto_increment, 0),
            _ => {}
        }
    }

    /// As opções que mudaram, a partir do que o pop-up tem agora.
    ///
    /// O auto-increment fica como está quando o campo é apagado; um valor
    /// digitado que não é número impede a gravação, assim como um nome vazio.
    pub fn alter(&self) -> Result<TableAlter, String> {
        if self.values.name.trim().is_empty() {
            return Err(String::from("informe o nome da tabela"));
        }
        let mut depois = self.values.clone();
        match self.auto_increment.trim() {
            "" => depois.auto_increment = self.original.auto_increment,
            texto => {
                let valor: u64 = texto
                    .parse()
                    .map_err(|_| format!("auto-increment inválido: {texto}"))?;
                if valor == 0 {
                    return Err(String::from("auto-increment precisa ser maior que zero"));
                }
                depois.auto_increment = Some(valor);
            }
        }
        Ok(TableAlter::between(&self.original, &depois))
    }
}
