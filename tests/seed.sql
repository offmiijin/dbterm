-- Fixture do dbterm: tabelas e dados para exercitar a TUI.
--
-- Carregado pelo entrypoint do MySQL (docker-entrypoint-initdb.d) apenas quando
-- o volume de dados está vazio. Depois de mexer aqui, derrube o container com
-- `down -v` para semear de novo.
--
-- Os valores foram escolhidos para bater nos casos chatos de desenho: NULL,
-- acentos, kanji, emoji, JSON, texto com quebra de linha, decimal, enum,
-- booleano e uma tabela grande o bastante para estourar o LIMIT 200 do dbterm.

-- O client do MySQL dentro do container tira o charset do locale, que ali é
-- POSIX — ou seja, latin1. Sem a linha abaixo, o arquivo (UTF-8) entra
-- corrompido e "Dora 石" é gravado como "Dora çŸ³".
SET NAMES utf8mb4;

USE dbterm;
CREATE TABLE users (
    id         INT AUTO_INCREMENT PRIMARY KEY,
    name       VARCHAR(80)  NOT NULL,
    email      VARCHAR(120) NOT NULL UNIQUE,
    active     BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at DATETIME     NOT NULL,
    last_login DATETIME     NULL,
    notes      TEXT         NULL,
    meta       JSON         NULL
);

CREATE TABLE products (
    id          INT AUTO_INCREMENT PRIMARY KEY,
    sku         CHAR(8)       NOT NULL UNIQUE,
    name        VARCHAR(80)   NOT NULL,
    price       DECIMAL(10,2) NOT NULL,
    stock       INT           NOT NULL DEFAULT 0,
    status      ENUM('draft', 'active', 'archived') NOT NULL DEFAULT 'draft',
    created_at  DATE          NOT NULL,
    description TEXT          NULL
);

CREATE TABLE orders (
    id           BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id      INT           NOT NULL,
    total        DECIMAL(12,2) NOT NULL,
    status       ENUM('pending', 'paid', 'shipped', 'canceled') NOT NULL,
    placed_at    DATETIME      NOT NULL,
    delivered_at DATETIME      NULL,
    note         VARCHAR(200)  NULL,
    CONSTRAINT orders_user_fk FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE events (
    id      BIGINT AUTO_INCREMENT PRIMARY KEY,
    kind    VARCHAR(16)  NOT NULL,
    payload VARCHAR(120) NOT NULL,
    at      DATETIME     NOT NULL,
    INDEX events_at (at)
);

-- Cinco usuários escritos à mão: um sem last_login, um sem meta, um desativado,
-- um com kanji e um com emoji (a largura de exibição do emoji é um limite
-- conhecido da UI).
INSERT INTO users (name, email, active, created_at, last_login, notes, meta) VALUES
    ('Ana Prado',   'ana@exemplo.com',   TRUE,  '2024-01-05 09:12:00', '2026-10-17 08:00:00', 'cliente desde o lançamento',          JSON_OBJECT('plano', 'pro', 'seats', 3)),
    ('Bruno Sá',    'bruno@exemplo.com', TRUE,  '2024-02-11 14:30:00', '2026-09-30 21:45:00', 'prefere contato por e-mail',          JSON_OBJECT('plano', 'free')),
    ('Carla Miúda', 'carla@exemplo.com', FALSE, '2024-03-02 08:00:00', NULL,                  'conta desativada a pedido',           NULL),
    ('Dora 石',     'dora@exemplo.com',  TRUE,  '2024-04-19 11:11:11', '2026-10-01 12:34:56', 'nome com kanji',                      JSON_OBJECT('plano', 'pro')),
    ('Eva 👩🏽‍💻',   'eva@exemplo.com',   TRUE,  '2024-05-30 23:59:00', '2026-10-18 00:01:00', 'emoji no nome e quebra\nde linha',    JSON_OBJECT('plano', 'team'));

-- Mais 25 clientes gerados, para a lista ter rolagem.
INSERT INTO users (name, email, active, created_at, last_login, notes)
WITH RECURSIVE seq(n) AS (SELECT 6 UNION ALL SELECT n + 1 FROM seq WHERE n < 30)
SELECT CONCAT('Cliente ', n),
       CONCAT('cliente', n, '@exemplo.com'),
       n % 2,
       DATE_ADD('2024-06-01 10:00:00', INTERVAL n DAY),
       IF(n % 3 = 0, NULL, DATE_ADD('2026-01-01 09:00:00', INTERVAL n HOUR)),
       NULL
FROM seq;

INSERT INTO products (sku, name, price, stock, status, created_at, description) VALUES
    ('SKU-0001', 'Teclado mecânico',     349.90, 12, 'active',   '2025-02-01', 'switches marrons\nlayout ABNT2'),
    ('SKU-0002', 'Mouse sem fio',        129.00, 40, 'active',   '2025-02-14', NULL),
    ('SKU-0003', 'Monitor 27"',         1899.99,  3, 'active',   '2025-03-03', 'IPS, 144 Hz'),
    ('SKU-0004', 'Suporte de monitor',    89.50,  0, 'archived', '2024-11-20', 'descontinuado pelo fornecedor'),
    ('SKU-0005', 'Cabo USB-C 2 m',        39.90, 210, 'active',  '2025-06-06', NULL),
    ('SKU-0006', 'Hub USB-C 7 em 1',      249.00,  8, 'draft',   '2026-09-01', 'aguardando homologação'),
    ('SKU-0007', 'Cadeira ergonômica',   2390.00,  2, 'active',   '2025-01-09', 'garantia de 5 anos'),
    ('SKU-0008', 'Luminária de mesa',     159.90, 17, 'archived', '2024-08-15', NULL);

INSERT INTO orders (user_id, total, status, placed_at, delivered_at, note) VALUES
    (1,  129.90, 'paid',     '2026-09-01 10:00:00', '2026-09-04 16:20:00', NULL),
    (1, 1899.99, 'shipped',  '2026-09-20 18:30:00', NULL,                  'entregar depois das 14h'),
    (2,  349.90, 'pending',  '2026-10-02 09:05:00', NULL,                  NULL),
    (3,   39.90, 'canceled', '2026-10-03 12:00:00', NULL,                  'cliente desistiu'),
    (4,  249.00, 'paid',     '2026-10-05 15:45:00', '2026-10-08 11:10:00', NULL),
    (4, 2390.00, 'paid',     '2026-10-06 08:15:00', '2026-10-12 09:00:00', 'montagem inclusa'),
    (5,  159.90, 'pending',  '2026-10-10 20:20:00', NULL,                  'cupom de desconto de 10%'),
    (5, 1899.99, 'shipped',  '2026-10-11 07:00:00', NULL,                  NULL);

-- Mais 40 pedidos gerados, espalhados entre os usuários.
INSERT INTO orders (user_id, total, status, placed_at, delivered_at)
WITH RECURSIVE seq(n) AS (SELECT 9 UNION ALL SELECT n + 1 FROM seq WHERE n < 48)
SELECT 1 + (n % 30),
       ROUND(19.90 + n * 7.35, 2),
       ELT(1 + (n % 4), 'pending', 'paid', 'shipped', 'canceled'),
       DATE_ADD('2026-01-01 08:00:00', INTERVAL n DAY),
       IF(n % 4 = 1, NULL, DATE_ADD('2026-01-05 08:00:00', INTERVAL n DAY))
FROM seq;

-- 500 eventos: o dbterm busca no máximo 200 linhas por vez, então esta tabela
-- mostra o limite da grade (e serve para medir rolagem).
INSERT INTO events (kind, payload, at)
WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n < 500)
SELECT ELT(1 + (n % 4), 'click', 'view', 'purchase', 'error'),
       CONCAT('evento ', n),
       DATE_ADD('2026-08-01 00:00:00', INTERVAL n MINUTE)
FROM seq;
