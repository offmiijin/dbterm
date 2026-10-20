# dbterm

Gerenciador de banco de dados via terminal (MySQL).

## Uso

```sh
cargo run -- mysql://usuario:senha@host:porta/schema
```

Sem argumento, o dbterm abre nas conexões rápidas. A URL também pode vir de
`DATABASE_URL` ou `MYSQL_URL`. O schema é opcional: sem ele, a lista de bancos
escolhe onde trabalhar.

## Desenvolvimento

```sh
cargo test        # unitários + integração (a integração pula sem container)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

O teste de integração sobe um MySQL descartável (veja `tests/README.md`):

```sh
cargo test --test mysql
```

## Arquitetura

- `app`: estado e regras — telas (`state`), pop-ups (`edits`), teclado
  (`input`), pedidos ao banco (`ops`) e respostas (`responses`).
- `db`: contrato (`Request`/`Response`), MySQL (`mysql`: catálogo, DDL,
  linhas, valores) e a thread de trabalho (`worker`). A UI nunca fala com o
  banco direto.
- `ui`: desenho puro sobre o estado — telas, grade e pop-ups.
- `view`: medidas neutras da grade, usadas por `app` e `ui` sem acoplá-los.
- `history` + `crypto` + `keychain`: conexões rápidas com a senha cifrada em
  repouso (AES-256-GCM, chave no cofre do SO).
- `config`: URL de conexão e campos dos formulários. `theme`: paleta.
