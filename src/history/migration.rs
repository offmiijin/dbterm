//! A chave da cifra: cofre do sistema primeiro, arquivo como fallback.
//!
//! O `chave.bin` legado tem prioridade (é ele que decifra os dados atuais);
//! sem nenhum dos dois, sorteia uma nova e tenta o cofre antes de recorrer ao
//! arquivo. A migração leva a chave do arquivo para o cofre.

use std::fs;
use std::path::Path;

use crate::crypto::Key;
use crate::keychain::{Keychain, Store};

use super::storage::default_key_path;

/// Move a chave do `chave.bin` para o cofre do sistema e apaga o arquivo.
///
/// Chamado no início do binário. Só apaga depois que o cofre comprovadamente
/// tem essa chave: se a gravação falha, ou se o cofre já tem outra chave, o
/// arquivo fica onde está.
pub fn migrar_chave() {
    migrar(&Keychain, default_key_path().as_deref());
}

/// A chave da cifra: o `chave.bin` legado tem prioridade (é ele que decifra os
/// dados atuais), depois o cofre; sem nenhum dos dois, sorteia uma nova e tenta
/// o cofre antes de recorrer ao arquivo.
pub(super) fn chave(cofre: &impl Store, path: Option<&Path>) -> Option<Key> {
    if let Some(arquivo) = path.and_then(Key::read) {
        return Some(arquivo);
    }
    if let Some(bytes) = cofre.read() {
        return Some(Key::from_bytes(bytes));
    }
    let nova = Key::random();
    if cofre.write(nova.bytes()) {
        return Some(nova);
    }
    // Sem cofre, o arquivo 0600 continua sendo o lugar da chave.
    path.and_then(Key::load_or_create)
}

/// Leva a chave de `path` para o cofre e apaga o arquivo quando dá.
fn migrar(cofre: &impl Store, path: Option<&Path>) {
    let Some(do_arquivo) = path.and_then(Key::read) else {
        return;
    };
    let Some(path) = path else {
        return;
    };
    let no_cofre = match cofre.read() {
        Some(bytes) => &bytes == do_arquivo.bytes(),
        None => cofre.write(do_arquivo.bytes()),
    };
    if no_cofre {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::crypto;
    use crate::keychain::Store;

    use super::*;

    /// Cofre de mentira, para exercitar a escolha da chave sem tocar no cofre
    /// do sistema operacional.
    #[derive(Default)]
    struct CofreFalso {
        chave: std::cell::RefCell<Option<[u8; 32]>>,
        aceita: bool,
    }

    impl CofreFalso {
        /// Cofre que recusa gravação, como num SSH sem D-Bus.
        fn mudo() -> Self {
            Self::default()
        }

        fn com(chave: [u8; 32]) -> Self {
            Self {
                chave: std::cell::RefCell::new(Some(chave)),
                aceita: true,
            }
        }
    }

    impl Store for CofreFalso {
        fn read(&self) -> Option<[u8; 32]> {
            *self.chave.borrow()
        }

        fn write(&self, chave: &[u8; 32]) -> bool {
            if !self.aceita {
                return false;
            }
            *self.chave.borrow_mut() = Some(*chave);
            true
        }
    }

    /// Caminho único no diretório temporário, já sem arquivo.
    fn caminho(nome: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dbterm-history-{}", std::process::id()));
        let path = dir.join(nome);
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn chave_nova_vai_para_o_cofre_e_nao_cria_arquivo() {
        let path = caminho("nova.bin");
        let cofre = CofreFalso {
            aceita: true,
            ..CofreFalso::default()
        };

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert!(!path.exists());
        assert_eq!(cofre.read().as_ref(), Some(chave.bytes()));
    }

    #[test]
    fn sem_cofre_a_chave_nova_vai_para_o_arquivo() {
        let path = caminho("fallback.bin");
        let cofre = CofreFalso::mudo();

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert_eq!(
            Key::read(&path).as_ref().map(Key::bytes),
            Some(chave.bytes())
        );
        assert_eq!(cofre.read(), None);
    }

    #[test]
    fn chave_do_cofre_e_usada_sem_criar_arquivo() {
        let path = caminho("cofre.bin");
        let cofre = CofreFalso::com([9; 32]);

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert_eq!(chave.bytes(), &[9; 32]);
        assert!(!path.exists());
    }

    #[test]
    fn arquivo_legado_tem_prioridade_sobre_o_cofre() {
        let path = caminho("prioridade.bin");
        crypto::write_private(&path, &[1; 32]).expect("grava");
        let cofre = CofreFalso::com([2; 32]);

        let chave = chave(&cofre, Some(&path)).expect("chave");

        assert_eq!(chave.bytes(), &[1; 32]);
    }

    #[test]
    fn migrar_move_a_chave_para_o_cofre_e_apaga_o_arquivo() {
        let path = caminho("migra.bin");
        crypto::write_private(&path, &[3; 32]).expect("grava");
        let cofre = CofreFalso {
            aceita: true,
            ..CofreFalso::default()
        };

        migrar(&cofre, Some(&path));

        assert!(!path.exists());
        assert_eq!(cofre.read(), Some([3; 32]));
    }

    #[test]
    fn migrar_sem_cofre_mantem_o_arquivo() {
        let path = caminho("migra-sem-cofre.bin");
        crypto::write_private(&path, &[4; 32]).expect("grava");
        let cofre = CofreFalso::mudo();

        migrar(&cofre, Some(&path));

        assert!(path.exists());
        assert_eq!(cofre.read(), None);
    }

    #[test]
    fn migrar_nao_sobrescreve_cofre_com_outra_chave() {
        let path = caminho("migra-conflito.bin");
        crypto::write_private(&path, &[5; 32]).expect("grava");
        let cofre = CofreFalso::com([6; 32]);

        migrar(&cofre, Some(&path));

        assert_eq!(cofre.read(), Some([6; 32]));
        assert_eq!(Key::read(&path).as_ref().map(Key::bytes), Some(&[5; 32]));
    }
}
