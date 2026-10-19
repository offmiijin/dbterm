# dbterm

Gerenciador de banco de dados via terminal.

## Onde a senha fica

As conexões rápidas são salvas em `$XDG_CONFIG_HOME/dbterm/conexoes.json`
(`~/.config/dbterm/conexoes.json`), com a senha cifrada (AES-256-GCM). A chave
dessa cifra mora no cofre do sistema — Keychain no macOS, Credential Manager no
Windows, Secret Service (GNOME Keyring/KWallet) no Linux.

Sem cofre disponível — SSH, contêiner, D-Bus fora do ar —, a chave cai para o
arquivo `chave.bin` ao lado do JSON, com permissão `0600`. Nesse caso a senha
continua fora do texto puro, mas quem lê a sua pasta pessoal lê a chave e
decifra a senha. O `chave.bin` de versões antigas é migrado para o cofre e
apagado na primeira execução.

`DBTERM_KEYRING=off` desliga o cofre e força o arquivo (útil em CI e em SSH).
Trocar de máquina não leva a chave: o JSON vai, o cofre não, então as senhas
salvas deixam de abrir e o app pede a senha de novo.
