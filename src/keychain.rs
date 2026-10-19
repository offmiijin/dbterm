//! Chave da cifra no cofre do sistema operacional.
//!
//! O cofre — Keychain no macOS, Credential Manager no Windows, Secret Service
//! (GNOME Keyring/KWallet) no Linux — é a casa preferida da chave: ela sai do
//! diretório do usuário e passa a depender do login do SO. Sem cofre (SSH,
//! container, D-Bus fora do ar), o app volta para o arquivo `chave.bin`, como
//! era antes.

use keyring::Entry;
use zeroize::Zeroize;

/// Nome do app no cofre; é o que aparece no gerenciador de senhas do sistema.
const SERVICO: &str = "dbterm";
/// Conta onde a chave mora. Mudar este nome órfã a entrada antiga e deixa as
/// senhas já salvas ilegíveis.
const CONTA: &str = "chave-historico";
/// `DBTERM_KEYRING=off` desliga o cofre, para quando ele atrapalha.
const AMBIENTE: &str = "DBTERM_KEYRING";

/// De onde a chave sai e para onde ela vai.
///
/// Existe para o histórico ser testável sem tocar no cofre de verdade.
pub trait Store {
    /// Chave guardada, se houver e o cofre responder.
    fn read(&self) -> Option<[u8; 32]>;

    /// Guarda (ou atualiza) a chave; `false` quando o cofre não aceita.
    fn write(&self, chave: &[u8; 32]) -> bool;
}

/// O cofre do sistema operacional.
pub struct Keychain;

impl Store for Keychain {
    fn read(&self) -> Option<[u8; 32]> {
        if desligado() {
            return None;
        }
        // Entrada ausente, cofre trancado ou sem D-Bus: sem chave, sem erro.
        let mut bytes = Entry::new(SERVICO, CONTA).ok()?.get_secret().ok()?;
        let chave = <[u8; 32]>::try_from(bytes.as_slice()).ok();
        bytes.zeroize();
        chave
    }

    fn write(&self, chave: &[u8; 32]) -> bool {
        if desligado() {
            return false;
        }
        Entry::new(SERVICO, CONTA)
            .and_then(|entrada| entrada.set_secret(chave))
            .is_ok()
    }
}

/// Se o cofre foi desligado por `DBTERM_KEYRING`.
fn desligado() -> bool {
    std::env::var(AMBIENTE).is_ok_and(|valor| matches!(valor.trim(), "off" | "0" | "false"))
}
