//! Histórico das conexões rápidas: o que fica salvo entre execuções.
//!
//! A senha é guardada, mas cifrada: em memória ela fica em claro para o app
//! usar; no disco vai só um blob do AES-256-GCM. A chave dessa cifra mora no
//! cofre do sistema (`keychain`), com o arquivo `chave.bin` como fallback. Veja
//! `crypto` para o limite dessa proteção.
//!
//! O modelo mora em `model`, o JSON e os caminhos em `storage` e a escolha e
//! migração da chave em `migration`.

mod migration;
mod model;
mod storage;

pub use migration::migrar_chave;
pub use model::Saved;

use std::fs;
use std::path::PathBuf;

use crate::crypto::{self, Key};
use crate::keychain::Keychain;

use migration::chave;
use storage::{Stored, default_key_path, default_path};

/// Quantas conexões a tela inicial mostra.
pub const RECENTES: usize = 3;

/// As conexões salvas, da mais recente para a mais antiga.
#[derive(Debug, Default)]
pub struct History {
    connections: Vec<Saved>,
    /// Onde ler/gravar; `None` quando a sessão não persiste (testes).
    path: Option<PathBuf>,
    /// Chave da cifra da senha; `None` quando não deu para obtê-la.
    key: Option<Key>,
}

impl History {
    /// Lê o arquivo do usuário; arquivo faltando ou inválido vira histórico
    /// vazio, nunca erro.
    pub fn load() -> Self {
        let path = default_path();
        let key = chave(&Keychain, default_key_path().as_deref());
        let connections = path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|json| serde_json::from_str::<Vec<Stored>>(&json).ok())
            .map(|stored| {
                stored
                    .into_iter()
                    .map(|entry| entry.abrir(key.as_ref()))
                    .collect()
            })
            .unwrap_or_default();
        let mut history = Self {
            connections,
            path,
            key,
        };
        history.sort();
        history
    }

    /// Um histórico só em memória, que não toca no disco.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Histórico em memória com entradas dadas, já ordenado.
    pub fn with(entries: Vec<Saved>) -> Self {
        let mut history = Self {
            connections: entries,
            ..Self::default()
        };
        history.sort();
        history
    }

    /// As conexões mais recentes, no máximo [`RECENTES`].
    pub fn recent(&self) -> &[Saved] {
        &self.connections[..self.connections.len().min(RECENTES)]
    }

    /// Todas as conexões salvas, para a navegação da tela de trabalho.
    pub fn all(&self) -> &[Saved] {
        &self.connections
    }

    /// Marca a conexão como a mais recente: sobe para o topo, sem duplicar.
    pub fn touch(&mut self, saved: Saved) {
        self.connections.retain(|entry| !entry.same(&saved));
        self.connections.push(saved);
        self.sort();
        self.save();
    }

    /// Troca a conexão editada pela que estava em `index`, no mesmo lugar da
    /// lista: a data de uso — e portanto a ordem — não muda.
    pub fn replace(&mut self, index: usize, mut saved: Saved) {
        let Some(atual) = self.connections.get(index) else {
            return;
        };
        saved.last_used = atual.last_used;
        self.connections[index] = saved;
        self.save();
    }

    fn sort(&mut self) {
        self.connections
            .sort_by(|a, b| b.last_used.cmp(&a.last_used));
    }

    /// Grava best-effort: falha de escrita não derruba a sessão.
    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        let stored: Vec<Stored> = self
            .connections
            .iter()
            .map(|entry| entry.guardar(self.key.as_ref()))
            .collect();
        if let Ok(json) = serde_json::to_string_pretty(&stored) {
            let _ = crypto::write_private(path, json.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(host: &str, last_used: u64) -> Saved {
        Saved {
            engine: String::from(Saved::MYSQL),
            host: String::from(host),
            port: 3306,
            user: String::from("root"),
            database: String::from("app"),
            password: None,
            last_used,
        }
    }

    #[test]
    fn recent_mostra_no_maximo_tres_e_ordena() {
        let history = History::with(vec![
            saved("a", 10),
            saved("b", 30),
            saved("c", 20),
            saved("d", 40),
        ]);
        let hosts: Vec<&str> = history
            .recent()
            .iter()
            .map(|entry| entry.host.as_str())
            .collect();
        assert_eq!(hosts, ["d", "b", "c"]);
    }

    #[test]
    fn touch_sobe_para_o_topo_sem_duplicar() {
        let mut history = History::with(vec![saved("a", 10), saved("b", 20)]);
        history.touch(saved("a", 30));
        let hosts: Vec<&str> = history
            .recent()
            .iter()
            .map(|entry| entry.host.as_str())
            .collect();
        assert_eq!(hosts, ["a", "b"]);
    }

    #[test]
    fn replace_edita_a_conexao_sem_mudar_o_lugar_na_lista() {
        let mut history = History::with(vec![saved("a", 10), saved("b", 20)]);
        let mut editada = saved("novo", 99);
        editada.port = 3307;

        history.replace(1, editada);

        let hosts: Vec<&str> = history
            .all()
            .iter()
            .map(|entry| entry.host.as_str())
            .collect();
        assert_eq!(hosts, ["b", "novo"], "a ordem é a mesma");
        assert_eq!(history.all()[1].port, 3307);
        assert_eq!(history.all()[1].last_used, 10, "a data de uso é preservada");
    }

    #[test]
    fn replace_ignora_indice_fora_da_lista() {
        let mut history = History::with(vec![saved("a", 10)]);
        history.replace(7, saved("novo", 99));
        assert_eq!(history.all()[0].host, "a");
    }
}
