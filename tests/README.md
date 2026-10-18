# Testes

Fixture de desenvolvimento do dbterm: um MySQL descartável, com tabelas e dados
semeados, para exercitar a TUI de verdade.

| Arquivo | O que é |
|---|---|
| `compose.yaml` | Serviço `docker.io/library/mysql:8.4` na porta **13306** do host, banco `dbterm`, usuário `dbterm` |
| `seed.sql` | Schema e dados, carregado pelo entrypoint só na primeira inicialização do volume |
| `mysql.rs` | Teste de integração: sobe o container, espera, e confere o que a TUI vai ler |

## Subir o container e abrir a TUI

```sh
podman compose -f tests/compose.yaml up -d    # proveito de podman-compose ou docker-compose
cargo run -- mysql://dbterm:dbterm@127.0.0.1:13306/dbterm
```

Os três comandos equivalentes: `podman compose -f tests/compose.yaml up -d`,
`podman-compose -f tests/compose.yaml up -d` e
`docker compose -f tests/compose.yaml up -d`. O nome do projeto sai do
diretório do arquivo (`tests`), então o container se chama `tests_mysql_1`
qualquer que seja a ferramenta — use sempre o mesmo caminho relativo para não
subir uma segunda instância brigando pela porta 13306.

Para acompanhar a inicialização (a primeira subida baixa a imagem):

```sh
podman logs -f tests_mysql_1
```

## Derrubar

```sh
podman compose -f tests/compose.yaml down      # para o container, mantém os dados
podman compose -f tests/compose.yaml down -v   # e apaga os dados
```
O `seed.sql` só roda com o volume vazio: **depois de mexer nele, use `down -v`**
para semear de novo.

## Rodar o teste

```sh
cargo test --test mysql
```

O teste sobe o container (idempotente: se já estiver no ar, reaproveita), espera
o MySQL aceitar conexão, lista as tabelas e confere usuários, pedidos e eventos
— inclusive acento, kanji, emoji e `NULL`, que são os casos que costumam quebrar
o desenho. Não derruba o container ao final: ele fica no ar para você abrir a
TUI, e a URL aparece na saída do teste.

Sem `docker compose`, `podman compose` ou `podman-compose` no PATH o teste é
**pulado** (imprime `SKIP`) em vez de falhar.

## Dados

| Tabela | Linhas | Serve para |
|---|---|---|
| `users` | 30 | `NULL` (`last_login`, `meta`), acentos, kanji, emoji, JSON, booleano |
| `products` | 8 | `DECIMAL`, `ENUM`, `DATE`, `NULL`, texto com quebra de linha |
| `orders` | 48 | FK para `users`, `DATETIME` nulo e preenchido, `ENUM` com quatro estados |
| `events` | 500 | Passa do `LIMIT 200` do dbterm: mostra o corte da grade e a rolagem |

O usuário `Eva 👩🏽‍💻` existe de propósito: a UI mede a largura da coluna por
contagem de caracteres, não por *grapheme cluster*, então é ali que o
desalinhamento aparece.

## Avisos

- Credenciais fixas (`dbterm`/`dbterm`, root `dbterm-root`), sem TLS e dados
  descartáveis: é fixture de desenvolvimento, não use como modelo de produção.
- A porta 13306 foi escolhida para não brigar com um MySQL local na 3306.
- A imagem vai com o registry explícito (`docker.io/library/mysql:8.4`): no
  podman, um nome curto como `mysql:8.4` depende de
  `unqualified-search-registries` no `registries.conf` e pode pedir escolha de
  registry ou falhar.
