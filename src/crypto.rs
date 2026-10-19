//! Cifra da senha salva: AES-256-GCM com uma chave local do app.
//!
//! A chave fica ao lado do arquivo de conexões, com permissão só do dono
//! (0600 no Unix). Isso tira a senha do texto puro do JSON — protege contra
//! leitura casual, backup, cópia do arquivo ou `git` — mas **não** protege
//! contra quem já consegue ler a sua pasta pessoal: quem lê a chave decifra a
//! senha. Segurança de verdade pediria a chave no keyring do sistema ou um
//! master password só seu.

use std::fs;
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit, OsRng, rand_core::RngCore};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use zeroize::Zeroize;

/// Tamanho do nonce do AES-GCM, em bytes.
const NONCE: usize = 12;
/// Tamanho da chave do AES-256, em bytes.
const KEY: usize = 32;

/// A chave da cifra, lida de um arquivo ou criada na primeira vez.
pub struct Key([u8; KEY]);

impl Key {
    /// Lê a chave de `path`; se não existir, sorteia uma nova e grava com
    /// permissão só do dono. `None` quando não deu nem para ler nem gravar —
    /// nesse caso a senha simplesmente não é salva.
    pub fn load_or_create(path: &Path) -> Option<Self> {
        if let Ok(bytes) = fs::read(path) {
            return Some(Self(<[u8; KEY]>::try_from(bytes).ok()?));
        }
        // Existe mas não deu para ler/usar: não sobrescreve a chave (poderia
        // apagar a única cópia que decifra as senhas já salvas).
        if path.exists() {
            return None;
        }
        let mut key = [0u8; KEY];
        OsRng.fill_bytes(&mut key);
        match write_private(path, &key) {
            Ok(()) => Some(Self(key)),
            Err(_) => {
                key.zeroize();
                None
            }
        }
    }

    /// Senha em claro vira `base64(nonce || cifra)`, ou `None` se a cifra falhar.
    pub fn seal(&self, senha: &str) -> Option<String> {
        let cifra = Aes256Gcm::new_from_slice(&self.0).ok()?;
        let mut nonce = [0u8; NONCE];
        OsRng.fill_bytes(&mut nonce);
        let selado = cifra
            .encrypt(Nonce::from_slice(&nonce), senha.as_bytes())
            .ok()?;
        let mut blob = nonce.to_vec();
        blob.extend(selado);
        Some(STANDARD.encode(blob))
    }

    /// Volta o texto em claro; `None` se o blob estiver corrompido ou tiver
    /// sido cifrado com outra chave.
    pub fn open(&self, blob: &str) -> Option<String> {
        let bytes = STANDARD.decode(blob).ok()?;
        if bytes.len() <= NONCE {
            return None;
        }
        let (nonce, cifra) = bytes.split_at(NONCE);
        let aes = Aes256Gcm::new_from_slice(&self.0).ok()?;
        let claro = aes.decrypt(Nonce::from_slice(nonce), cifra).ok()?;
        String::from_utf8(claro).ok()
    }

    /// Chave fixa para os testes que não precisam tocar no disco.
    #[cfg(test)]
    pub fn test() -> Self {
        Self([7u8; KEY])
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Nunca imprime os bytes da chave.
impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(..)")
    }
}

/// Grava um arquivo privado (0600 no Unix), criando o diretório se preciso.
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    options.open(path)?.write_all(bytes)?;
    #[cfg(unix)]
    {
        // Garante 0600 também quando o arquivo já existia.
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Caminho único no diretório temporário, já sem arquivo.
    fn caminho(nome: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dbterm-crypto-{}", std::process::id()));
        let path = dir.join(nome);
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn sela_e_abre_a_senha() {
        let key = Key::load_or_create(&caminho("roundtrip.bin")).expect("chave");
        let selado = key.seal("segredo").expect("sela");
        assert!(!selado.contains("segredo"), "{selado}");
        assert_eq!(key.open(&selado).as_deref(), Some("segredo"));
    }

    #[test]
    fn nonce_novo_a_cada_selo() {
        let key = Key::load_or_create(&caminho("nonce.bin")).expect("chave");
        assert_ne!(key.seal("x"), key.seal("x"));
    }

    #[test]
    fn outra_chave_nao_abre() {
        let path = caminho("troca.bin");
        let selado = Key::load_or_create(&path)
            .expect("chave")
            .seal("segredo")
            .expect("sela");
        let _ = fs::remove_file(&path);
        let outra = Key::load_or_create(&path).expect("outra chave");
        assert_eq!(outra.open(&selado), None);
    }

    #[test]
    fn blob_corrompido_nao_abre() {
        let key = Key::load_or_create(&caminho("corrompido.bin")).expect("chave");
        assert_eq!(key.open("nao é base64!!"), None);
        assert_eq!(key.open("YWJj"), None);
    }

    #[test]
    fn chave_existente_invalida_nao_e_sobrescrita() {
        let path = caminho("invalida.bin");
        fs::create_dir_all(path.parent().expect("pai")).expect("dir");
        fs::write(&path, b"abc").expect("grava");
        assert!(Key::load_or_create(&path).is_none());
        assert_eq!(fs::read(&path).expect("le"), b"abc");
    }

    #[cfg(unix)]
    #[test]
    fn chave_so_para_o_dono() {
        use std::os::unix::fs::PermissionsExt;

        let path = caminho("perm.bin");
        let _ = Key::load_or_create(&path).expect("chave");
        let modo = fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(modo, 0o600);
    }
}
