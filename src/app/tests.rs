use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::input::{PAGE, step};
use super::*;
use crate::config::Field;
use crate::db::{Charset, Collation, ColumnKind};
use crate::history::{History, Saved};

fn app() -> App {
    let mut app = App::new(Some("mysql://user@localhost/app"));
    app.history = History::in_memory();
    app
}

/// App na tela inicial, sem histórico em disco.
fn app_quick() -> App {
    let mut app = App::new(None);
    app.history = History::in_memory();
    app.quick = None;
    app
}

/// App com conexões rápidas salvas, na ordem dada (a primeira é a mais recente).
fn app_com_rapidas(hosts: &[&str]) -> App {
    let mut app = app_quick();
    let total = hosts.len();
    let entries = hosts
        .iter()
        .enumerate()
        .map(|(i, host)| Saved {
            engine: String::from(Saved::MYSQL),
            host: String::from(*host),
            port: 3306,
            user: String::from("root"),
            database: String::from("app"),
            password: None,
            last_used: (total - i) as u64,
        })
        .collect();
    app.history = History::with(entries);
    app.quick = Some(0);
    app
}

/// App com o formulário aberto via `n`.
fn app_form() -> App {
    let mut app = app_quick();
    app.on_key(KeyEvent::from(KeyCode::Char('n')));
    app
}

fn app_com_grade() -> App {
    let mut app = app();
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.current = Some(String::from("users"));
    app.data = Some(TableData {
        columns: vec![String::from("id"), String::from("name")],
        kinds: vec![ColumnKind::Number, ColumnKind::Text],
        rows: vec![
            vec![String::from("1"), String::from("ana")],
            vec![String::from("2"), String::from("bia")],
        ],
        primary_key: vec![0],
    });
    app.focus = Focus::Data;
    app.table_state.select(Some(0));
    app.status = Status::Ready;
    app
}

#[test]
fn step_clampa_nos_limites() {
    assert_eq!(step(None, 1, 0), None);
    assert_eq!(step(None, -1, 3), Some(0));
    assert_eq!(step(Some(0), -1, 3), Some(0));
    assert_eq!(step(Some(2), 1, 3), Some(2));
    assert_eq!(step(Some(1), PAGE, 3), Some(2));
    assert_eq!(step(Some(1), 1, 3), Some(2));
}

#[test]
fn conectado_mostra_servidor_e_schema() {
    let mut app = app();
    app.on_response(Response::Connected {
        server: String::from("MySQL 8.0.35"),
        database: String::from("app"),
    });
    assert_eq!(app.server, "MySQL 8.0.35");
    assert_eq!(app.database, "app");
}

#[test]
fn lista_de_tabelas_seleciona_a_primeira() {
    let mut app = app();
    app.on_response(Response::Tables(vec![String::from("users")]));
    assert_eq!(app.list_state.selected(), Some(0));
    assert!(matches!(app.status, Status::Loading(_)));
}

#[test]
fn resposta_de_linhas_preenche_a_grade() {
    let mut app = app();
    app.on_response(Response::Rows {
        table: String::from("users"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("1")]],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.current.as_deref(), Some("users"));
    assert_eq!(app.row_count(), 1);
    assert_eq!(app.table_state.selected(), Some(0));
    assert!(matches!(app.status, Status::Ready));
}

#[test]
fn falha_vira_status_de_erro() {
    let mut app = app();
    app.on_response(Response::Failed(String::from("sem permissão")));
    assert!(matches!(app.status, Status::Failed(message) if message == "sem permissão"));
}

#[test]
fn teclas_navegam_selecionam_e_saem() {
    let mut app = app();
    app.focus = Focus::Tables;
    app.tables = vec![String::from("a"), String::from("b")];
    app.list_state.select(Some(0));

    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.list_state.selected(), Some(1));
    app.on_key(KeyEvent::from(KeyCode::Down));
    assert_eq!(app.list_state.selected(), Some(1), "clampa no fim");
    app.on_key(KeyEvent::from(KeyCode::Char('g')));
    assert_eq!(app.list_state.selected(), Some(0));
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert_eq!(app.list_state.selected(), Some(1));

    // `h`/`l` não trocam mais de painel: nas tabelas eles não fazem nada.
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert!(app.focus == Focus::Tables);

    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.should_quit);
}

#[test]
fn esc_sobe_um_nivel_por_vez_ate_pedir_confirmacao() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;

    // Nos registros, `esc` volta às tabelas em vez de sair.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Tables);
    assert_eq!(app.mode, Mode::Rows);
    assert!(!app.should_quit);

    // Nas tabelas, sobe para os bancos; e depois para as conexões.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Databases);
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Connections);

    // Na raiz da cascata, `esc` abre a confirmação em vez de sair na hora.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);

    // `esc` na confirmação cancela e volta às conexões.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Rows);
    assert!(app.focus == Focus::Connections);
    assert!(!app.should_quit);

    // Confirmar com `enter` sai da aplicação.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.should_quit);
}

#[test]
fn teclas_c_b_t_levam_ao_nivel() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    app.column = Some(0);

    // Sair dos registros pelos chips fecha o modo coluna.
    app.on_key(KeyEvent::from(KeyCode::Char('c')));
    assert!(app.focus == Focus::Connections);
    assert_eq!(app.mode, Mode::Rows);
    assert_eq!(app.column, None);

    app.on_key(KeyEvent::from(KeyCode::Char('b')));
    assert!(app.focus == Focus::Databases);
    app.on_key(KeyEvent::from(KeyCode::Char('t')));
    assert!(app.focus == Focus::Tables);
}

#[test]
fn j_k_andam_nas_conexoes_e_nos_bancos() {
    let mut app = app_com_rapidas(&["a", "b"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.connections_state.selected(), Some(1));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.connections_state.selected(), Some(1), "clampa no fim");
    app.on_key(KeyEvent::from(KeyCode::Char('g')));
    assert_eq!(app.connections_state.selected(), Some(0));

    app.focus = Focus::Databases;
    app.databases = vec![String::from("app"), String::from("outro")];
    app.databases_state.select(Some(0));
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert_eq!(app.databases_state.selected(), Some(1));
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(app.databases_state.selected(), Some(0));
}

#[test]
fn enter_nas_conexoes_abre_a_conexao_selecionada() {
    let mut app = app_com_rapidas(&["db.local"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    app.databases = vec![String::from("app")];
    app.tables = vec![String::from("users")];

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(matches!(app.status, Status::Connecting));
    assert!(app.databases.is_empty(), "bancos da conexão anterior somem");
    assert!(app.tables.is_empty(), "tabelas da conexão anterior somem");
}

#[test]
fn conectado_pede_os_bancos_e_desce_para_eles() {
    let mut app = app_form();
    app.connecting = true;
    app.on_response(Response::Connected {
        server: String::from("MySQL 8.0.35"),
        database: String::from("app"),
    });
    assert!(app.focus == Focus::Databases);
    assert!(matches!(app.status, Status::Loading(ref o) if o == "bancos"));
}

#[test]
fn bancos_selecionam_o_schema_atual_e_trocam_por_ele() {
    let mut app = app();
    app.database = String::from("outro");
    app.on_response(Response::Databases(vec![
        String::from("app"),
        String::from("outro"),
    ]));
    assert_eq!(app.databases_state.selected(), Some(1));
    assert!(matches!(app.status, Status::Loading(ref o) if o == "outro"));
}

#[test]
fn sem_schema_atual_o_primeiro_banco_e_escolhido() {
    let mut app = app();
    app.database.clear();
    app.on_response(Response::Databases(vec![
        String::from("clientes"),
        String::from("outro"),
    ]));
    assert_eq!(app.databases_state.selected(), Some(0));
    assert!(matches!(app.status, Status::Loading(ref o) if o == "clientes"));
}

#[test]
fn sem_bancos_a_lista_fica_vazia_e_sem_selecao() {
    let mut app = app();
    app.on_response(Response::Databases(Vec::new()));
    assert_eq!(app.databases_state.selected(), None);
    assert!(matches!(app.status, Status::Ready));
}

#[test]
fn trocar_de_banco_pede_as_tabelas_do_novo() {
    let mut app = app_com_grade();
    app.on_response(Response::Database(String::from("outro")));
    assert_eq!(app.database, "outro");
    assert!(matches!(app.status, Status::Loading(ref o) if o == "tabelas"));
}

#[test]
fn enter_no_banco_atual_desce_para_as_tabelas() {
    let mut app = app_com_grade();
    app.database = String::from("app");
    app.focus = Focus::Databases;
    app.databases = vec![String::from("app")];
    app.databases_state.select(Some(0));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Tables);
}

#[test]
fn q_em_colunas_confirma_e_cancelar_volta_ao_modo_coluna() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);

    // `q` pede confirmação em vez de sair direto.
    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);

    // Cancelar restaura o modo de origem, preservando a coluna selecionada.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Columns);
    assert_eq!(app.column, Some(0));
    assert!(!app.should_quit);
}

#[test]
fn enter_troca_de_painel_e_h_l_nao() {
    let mut app = app_com_grade();
    app.focus = Focus::Tables;

    // `h`/`l` não trocam mais de painel; quem leva aos dados é o `enter`.
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Data);

    // De volta às tabelas, só com `esc`.
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.focus == Focus::Tables);

    // Tab foi removido da troca de foco entre os painéis.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert!(app.focus == Focus::Tables);
    app.on_key(KeyEvent::from(KeyCode::BackTab));
    assert!(app.focus == Focus::Tables);
}

#[test]
fn h_l_nos_registros_rolam_as_colunas() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    // `h`/`l` andam na horizontal pelas colunas, do mesmo jeito que as setas.
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert_eq!(app.hscroll, 1);
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert_eq!(app.hscroll, 1, "clampa na última coluna");
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert_eq!(app.hscroll, 0);
    app.on_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(app.hscroll, 0, "clampa na primeira coluna");
    // Rolar não mexe na seleção nem no modo.
    assert_eq!(app.column, None);
    assert_eq!(app.mode, Mode::Rows);
}

#[test]
fn enter_entra_no_modo_coluna_e_h_l_anda_na_horizontal() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);
    assert_eq!(app.column, Some(0));

    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    assert_eq!(app.column, Some(1));
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.column, Some(1), "clampa no fim");
    app.on_key(KeyEvent::from(KeyCode::Char('h')));
    assert_eq!(app.column, Some(0));

    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Rows);
    assert_eq!(app.column, None);
}

#[test]
fn edicao_passa_por_duas_confirmacoes() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Edit);
    let edit = app.edit.as_ref().expect("edição aberta");
    assert_eq!(edit.original, "1");
    assert_eq!(edit.input, "1");

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    app.on_key(KeyEvent::from(KeyCode::Char('9')));
    assert_eq!(app.edit.as_ref().unwrap().input, "9");

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Confirm);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::ConfirmAgain);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);
    assert!(app.edit.is_none());
}

#[test]
fn esc_na_confirmacao_volta_a_editar() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Confirm);

    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Edit);
    assert!(app.edit.is_some());
}

#[test]
fn tabela_sem_chave_primaria_nao_abre_edicao() {
    let mut app = app_com_grade();
    if let Some(data) = app.data.as_mut() {
        data.primary_key.clear();
    }
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Columns);
    assert!(app.edit.is_none());
    assert!(matches!(app.status, Status::Notice(_)));
}

#[test]
fn recarregar_a_mesma_tabela_preserva_linha_e_coluna() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Char('l')));
    app.table_state.select(Some(1));

    app.on_response(Response::Rows {
        table: String::from("users"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id"), String::from("name")],
            kinds: vec![ColumnKind::Number, ColumnKind::Text],
            rows: vec![
                vec![String::from("1"), String::from("ana")],
                vec![String::from("2"), String::from("bia")],
            ],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.table_state.selected(), Some(1));
    assert_eq!(app.column, Some(1));
    assert_eq!(app.mode, Mode::Columns);
}

#[test]
fn navegacao_horizontal_nao_para_nas_divisorias() {
    let mut app = app_com_grade();
    if let Some(data) = app.data.as_mut() {
        data.columns = ["id", "name", "email"]
            .iter()
            .map(|nome| nome.to_string())
            .collect();
        data.rows = vec![
            ["1", "ana", "ana@exemplo.com"]
                .iter()
                .map(|valor| valor.to_string())
                .collect(),
        ];
    }
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let mut vistos = Vec::new();
    for _ in 0..3 {
        app.on_key(KeyEvent::from(KeyCode::Char('l')));
        vistos.push(app.column.unwrap());
    }
    // 0 → coluna 1 → coluna 2 → clampa.
    assert_eq!(vistos, vec![1, 2, 2]);
}

#[test]
fn segundo_enter_na_tabela_aberta_vai_aos_registros() {
    let mut app = app();
    app.focus = Focus::Tables;
    app.tables = vec![String::from("users")];
    app.list_state.select(Some(0));
    app.status = Status::Ready;

    // Primeiro enter: começa a carregar e o foco continua na lista.
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Tables);
    assert!(matches!(app.status, Status::Loading(ref t) if t == "users"));

    // Resposta: os registros ficam prontos, mas o foco segue na lista.
    app.on_response(Response::Rows {
        table: String::from("users"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("1")]],
            primary_key: vec![0],
        },
    });
    assert!(app.focus == Focus::Tables);

    // Segundo enter: vai direto aos registros, sem precisar de `l`.
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Data);
    assert_eq!(app.mode, Mode::Rows);
}

#[test]
fn enter_em_outra_tabela_carrega_sem_pular_aos_registros() {
    // `users` está aberta, mas a lista aponta para `orders`: o enter deve
    // carregar a nova tabela e manter o foco na lista.
    let mut app = app_com_grade();
    app.focus = Focus::Tables;
    app.tables = vec![String::from("users"), String::from("orders")];
    app.list_state.select(Some(1));

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.focus == Focus::Tables);
    assert!(matches!(app.status, Status::Loading(ref t) if t == "orders"));
}

#[test]
fn abrir_outra_tabela_reseta_a_selecao_e_o_modo() {
    let mut app = app_com_grade();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Char('l')));

    app.on_response(Response::Rows {
        table: String::from("orders"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("7")]],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.table_state.selected(), Some(0));
    assert_eq!(app.column, None);
    assert_eq!(app.mode, Mode::Rows);
}

#[test]
fn sem_url_o_app_abre_nas_conexoes_rapidas() {
    let app = app_quick();
    assert!(app.screen == Screen::Quick);
    assert!(app.quick.is_none(), "sem histórico não há seleção");
}

#[test]
fn com_url_o_app_vai_direto_para_a_tela_de_trabalho() {
    assert!(app().screen == Screen::Browse);
}

#[test]
fn rapidas_selecionam_a_mais_recente() {
    let app = app_com_rapidas(&["a", "b"]);
    assert_eq!(app.quick, Some(0));
    assert_eq!(app.history.recent()[0].host, "a");
}

#[test]
fn n_abre_o_formulario_vazio() {
    let mut app = app_com_rapidas(&["a"]);
    app.on_key(KeyEvent::from(KeyCode::Char('n')));
    assert!(app.screen == Screen::Connect);
    assert_eq!(app.field, Field::Host);
    assert_eq!(app.connection.value(Field::Host), "127.0.0.1");
    assert_eq!(app.connection.value(Field::Database), "");
}

#[test]
fn enter_abre_a_rapida_preenchida_com_foco_na_senha() {
    let mut app = app_com_rapidas(&["db.local"]);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.screen == Screen::Connect);
    assert_eq!(app.field, Field::Password);
    assert_eq!(app.connection.value(Field::Host), "db.local");
    assert_eq!(app.connection.value(Field::Port), "3306");
    assert_eq!(app.connection.value(Field::User), "root");
    assert_eq!(app.connection.value(Field::Database), "app");
    assert_eq!(app.connection.value(Field::Password), "");
}

#[test]
fn j_k_anda_pelas_rapidas() {
    let mut app = app_com_rapidas(&["a", "b"]);
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.quick, Some(1));
    app.on_key(KeyEvent::from(KeyCode::Down));
    assert_eq!(app.quick, Some(1), "clampa no fim");
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(app.quick, Some(0));
}

#[test]
fn esc_nas_rapidas_sai() {
    let mut app = app_quick();
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.should_quit);
}

#[test]
fn esc_no_formulario_volta_para_as_rapidas() {
    let mut app = app_form();
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.screen == Screen::Quick);
    assert!(!app.should_quit);
}

#[test]
fn tab_anda_entre_os_campos_dando_a_volta() {
    let mut app = app_form();
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(app.field, Field::Port);
    app.on_key(KeyEvent::from(KeyCode::Down));
    assert_eq!(app.field, Field::User);
    app.on_key(KeyEvent::from(KeyCode::BackTab));
    assert_eq!(app.field, Field::Port);

    // Voltar do primeiro campo cai no último.
    app.field = Field::Host;
    app.on_key(KeyEvent::from(KeyCode::Up));
    assert_eq!(app.field, Field::Database);
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(app.field, Field::Host);
}

#[test]
fn digitar_edita_o_campo_com_foco_sem_sair() {
    let mut app = app_form();
    app.field = Field::Database;
    // `q` e `j` são texto na tela de conexão.
    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(app.connection.value(Field::Database), "qj");
    assert!(!app.should_quit);

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(app.connection.value(Field::Database), "q");
}

#[test]
fn formulario_incompleto_mostra_o_erro_e_nao_conecta() {
    let mut app = app_form();
    app.connection.value_mut(Field::User).clear();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.connect_error.as_deref(), Some("informe o usuário"));
    assert!(app.screen == Screen::Connect);
    assert!(!app.connecting);
}

#[test]
fn formulario_sem_schema_conecta_e_escolhe_o_banco_depois() {
    let mut app = app_form();
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.connect_error.is_none());
    assert!(app.connecting);
}

#[test]
fn formulario_valido_pede_a_conexao() {
    let mut app = app_form();
    app.field = Field::Database;
    app.on_key(KeyEvent::from(KeyCode::Char('a')));
    app.on_key(KeyEvent::from(KeyCode::Char('p')));
    app.on_key(KeyEvent::from(KeyCode::Char('p')));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert!(app.connect_error.is_none());
    assert!(app.connecting);
}

#[test]
fn enquanto_conecta_o_teclado_nao_edita() {
    let mut app = app_form();
    app.connecting = true;
    app.field = Field::Host;
    app.on_key(KeyEvent::from(KeyCode::Char('x')));
    assert_eq!(app.connection.value(Field::Host), "127.0.0.1");
}

#[test]
fn esc_durante_a_conexao_abandona_e_volta() {
    let mut app = app_form();
    app.connecting = true;
    app.on_key(KeyEvent::from(KeyCode::Esc));
    assert!(app.screen == Screen::Quick);
    assert!(!app.connecting);
}

#[test]
fn conexao_aceita_vai_para_a_grade_e_entra_nas_rapidas() {
    let mut app = app_form();
    app.connecting = true;
    app.on_response(Response::Connected {
        server: String::from("MySQL 8.0.35"),
        database: String::from("app"),
    });
    assert!(app.screen == Screen::Browse);
    assert!(!app.connecting);
    assert_eq!(app.server, "MySQL 8.0.35");
    assert_eq!(app.database, "app");
    assert_eq!(app.history.recent().len(), 1);
    assert_eq!(app.history.recent()[0].host, "127.0.0.1");
}

#[test]
fn conexao_recusada_volta_ao_formulario_com_o_motivo() {
    let mut app = app_form();
    app.connecting = true;
    app.on_response(Response::Failed(String::from("falha ao conectar")));
    assert!(app.screen == Screen::Connect);
    assert!(!app.connecting);
    assert_eq!(app.connect_error.as_deref(), Some("falha ao conectar"));
    assert!(app.history.recent().is_empty(), "falha não salva conexão");
}

#[test]
fn enter_abre_a_rapida_com_a_senha_salva() {
    let mut app = app_quick();
    app.history = History::with(vec![Saved {
        engine: String::from(Saved::MYSQL),
        host: String::from("db.local"),
        port: 3306,
        user: String::from("root"),
        database: String::from("app"),
        password: Some(String::from("segredo")),
        last_used: 1,
    }]);
    app.quick = Some(0);
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(app.connection.value(Field::Password), "segredo");
}

#[test]
fn setas_horizontais_rolam_as_colunas_da_grade() {
    let mut app = app_com_grade();
    app.focus = Focus::Data;
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.hscroll, 1);
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.hscroll, 1, "clampa na última coluna");
    app.on_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(app.hscroll, 0);
    app.on_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(app.hscroll, 0, "clampa na primeira coluna");

    // Na lista de tabelas as setas horizontais não rolam a grade.
    app.focus = Focus::Tables;
    app.on_key(KeyEvent::from(KeyCode::Right));
    assert_eq!(app.hscroll, 0);
}

#[test]
fn tabela_nova_volta_a_rolagem_horizontal_ao_inicio() {
    let mut app = app_com_grade();
    app.hscroll = 1;
    app.on_response(Response::Rows {
        table: String::from("outra"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id"), String::from("name")],
            kinds: vec![ColumnKind::Number, ColumnKind::Text],
            rows: vec![vec![String::from("1"), String::from("ana")]],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.hscroll, 0);
}

/// Grade com `linhas` registros de uma coluna só, já focada nos dados.
fn grade_com(table: &str, linhas: usize) -> App {
    let mut app = app();
    app.tables = vec![table.to_string()];
    app.list_state.select(Some(0));
    app.current = Some(table.to_string());
    app.data = Some(TableData {
        columns: vec![String::from("id")],
        kinds: vec![ColumnKind::Number],
        rows: (1..=linhas).map(|n| vec![n.to_string()]).collect(),
        primary_key: vec![0],
    });
    app.focus = Focus::Data;
    app.table_state.select(Some(0));
    app.status = Status::Ready;
    app
}

/// Uma página de `linhas` registros a partir de `primeira`.
fn pagina(tabela: &str, primeira: usize, linhas: usize) -> Response {
    Response::Rows {
        table: tabela.to_string(),
        offset: primeira - 1,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: (primeira..primeira + linhas)
                .map(|n| vec![n.to_string()])
                .collect(),
            primary_key: vec![0],
        },
    }
}

#[test]
fn encostar_na_ultima_linha_pede_e_anexa_a_pagina_seguinte() {
    let mut app = grade_com("events", ROW_LIMIT);
    // Enquanto a seleção não chega no fim, nada é pedido.
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(!app.loading_more);
    assert!(!app.end_reached);

    // Chegou no registro 200: pede mais 200.
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert!(app.loading_more);
    assert!(matches!(app.status, Status::Loading(ref tabela) if tabela == "events"));

    app.on_response(pagina("events", ROW_LIMIT + 1, ROW_LIMIT));
    assert_eq!(app.row_count(), 2 * ROW_LIMIT);
    assert!(!app.loading_more);
    assert!(!app.end_reached);
    assert!(matches!(app.status, Status::Ready));
    // A seleção fica onde estava: a página entra no fim, não no lugar do cursor.
    assert_eq!(app.table_state.selected(), Some(ROW_LIMIT - 1));

    // Chegou no registro 400: mais 200.
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert!(app.loading_more);
    app.on_response(pagina("events", 2 * ROW_LIMIT + 1, ROW_LIMIT));
    assert_eq!(app.row_count(), 3 * ROW_LIMIT);
}

#[test]
fn pagina_curta_marca_o_fim_da_tabela() {
    let mut app = grade_com("events", ROW_LIMIT);
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert!(app.loading_more);

    app.on_response(pagina("events", ROW_LIMIT + 1, 12));
    assert!(app.end_reached);
    assert_eq!(app.row_count(), ROW_LIMIT + 12);

    // No fim da tabela não há mais o que pedir.
    app.on_key(KeyEvent::from(KeyCode::Char('G')));
    assert_eq!(app.table_state.selected(), Some(ROW_LIMIT + 11));
    assert!(!app.loading_more);
}

#[test]
fn pagina_que_chega_depois_de_trocar_de_tabela_e_descartada() {
    let mut app = grade_com("events", 10);
    app.table_state.select(Some(8));
    app.on_key(KeyEvent::from(KeyCode::Char('j'))); // chegou no fim: pede a próxima
    assert!(app.loading_more);

    // A resposta da primeira página de outra tabela chega antes.
    app.on_response(Response::Rows {
        table: String::from("users"),
        offset: 0,
        limit: ROW_LIMIT,
        data: TableData {
            columns: vec![String::from("id")],
            kinds: vec![ColumnKind::Number],
            rows: vec![vec![String::from("1")]],
            primary_key: vec![0],
        },
    });
    assert_eq!(app.current.as_deref(), Some("users"));

    // A página atrasada de `events` não pode entrar na grade de `users`.
    app.on_response(pagina("events", 11, 5));
    assert_eq!(app.current.as_deref(), Some("users"));
    assert_eq!(app.row_count(), 1);
}

#[test]
fn recarregar_a_mesma_tabela_pede_as_linhas_ja_carregadas() {
    let app = grade_com("events", 2 * ROW_LIMIT);
    assert_eq!(app.page_limit("events"), 2 * ROW_LIMIT);
    // Outra tabela começa do zero.
    assert_eq!(app.page_limit("users"), ROW_LIMIT);
    // Uma tabela com menos de uma página também pede ao menos uma página.
    let app = grade_com("events", 3);
    assert_eq!(app.page_limit("events"), ROW_LIMIT);
}

#[test]
fn falha_na_pagina_libera_a_proxima_tentativa() {
    let mut app = grade_com("events", 10);
    app.table_state.select(Some(8));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(app.loading_more);

    app.on_response(Response::Failed(String::from("conexão caiu")));
    assert!(!app.loading_more);
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "conexão caiu"));

    // A tabela não acabou: subir e descer de novo ao fim tenta outra página.
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert!(!app.loading_more);
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(app.loading_more);
}

#[test]
fn segurar_a_tecla_no_fim_da_pagina_nao_pede_de_novo() {
    let mut app = grade_com("events", 10);
    app.table_state.select(Some(8));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert!(app.loading_more);
    // A resposta chega e a página nova entra: a seleção não se moveu.
    app.on_response(pagina("events", 11, ROW_LIMIT));
    assert!(!app.loading_more);
    assert_eq!(app.row_count(), 10 + ROW_LIMIT);

    // No fim da página carregada, repetir `j` não pede nada.
    app.table_state.select(Some(10 + ROW_LIMIT - 1));
    for _ in 0..5 {
        app.on_key(KeyEvent::from(KeyCode::Char('j')));
    }
    assert_eq!(app.table_state.selected(), Some(10 + ROW_LIMIT - 1));
    assert!(
        !app.loading_more,
        "tecla repetida no fim não pode pedir página"
    );

    // `G` já no fim também não.
    for _ in 0..5 {
        app.on_key(KeyEvent::from(KeyCode::Char('G')));
    }
    assert!(!app.loading_more);
}

/// App na tela de trabalho, com uma conexão salva selecionada.
fn app_nas_conexoes() -> App {
    let mut app = app_com_rapidas(&["db.local", "outro"]);
    app.screen = Screen::Browse;
    app.focus = Focus::Connections;
    app.connections_state.select(Some(0));
    app
}

#[test]
fn e_nas_conexoes_abre_o_popup_de_edicao_preenchido() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    let edicao = app.connection_edit.as_ref().expect("pop-up aberto");
    assert_eq!(edicao.saved, 0);
    assert_eq!(edicao.field, Field::Host);
    assert_eq!(edicao.connection.value(Field::Host), "db.local");
    assert_eq!(edicao.connection.value(Field::Port), "3306");
    assert_eq!(edicao.connection.value(Field::User), "root");
    assert_eq!(edicao.connection.value(Field::Database), "app");
}

#[test]
fn e_fora_das_conexoes_nao_abre_a_edicao() {
    let mut app = app_nas_conexoes();
    app.focus = Focus::Tables;
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    assert!(app.connection_edit.is_none());
    assert_eq!(app.mode, Mode::Rows);
}

#[test]
fn tab_anda_nos_campos_do_popup_na_ordem_da_tela() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));

    let mut ordem = vec![app.connection_edit.as_ref().expect("pop-up").field];
    for _ in 0..4 {
        app.on_key(KeyEvent::from(KeyCode::Tab));
        ordem.push(app.connection_edit.as_ref().expect("pop-up").field);
    }
    // Servidor (host, porta, banco) e depois autenticação (usuário, senha).
    assert_eq!(
        ordem,
        vec![
            Field::Host,
            Field::Port,
            Field::Database,
            Field::User,
            Field::Password
        ]
    );

    // Tab dá a volta; no pop-up, digitar texto anda com `tab` e setas.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.connection_edit.as_ref().expect("pop-up").field,
        Field::Host
    );
    app.on_key(KeyEvent::from(KeyCode::BackTab));
    assert_eq!(
        app.connection_edit.as_ref().expect("pop-up").field,
        Field::Password
    );
}

#[test]
fn digitar_no_popup_edita_o_campo_com_foco_sem_sair() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    // `q` e `j` são texto no pop-up.
    app.on_key(KeyEvent::from(KeyCode::Char('q')));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.connection_edit
            .as_ref()
            .expect("pop-up")
            .connection
            .value(Field::Host),
        "db.localqj"
    );
    assert!(!app.should_quit);
    assert!(matches!(app.mode, Mode::Rows));

    app.on_key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(
        app.connection_edit
            .as_ref()
            .expect("pop-up")
            .connection
            .value(Field::Host),
        "db.localq"
    );
}

#[test]
fn enter_grava_a_conexao_editada_no_mesmo_lugar_da_lista() {
    let mut app = app_nas_conexoes();
    app.focus = Focus::Connections;
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    for _ in 0.."db.local".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    for caractere in "novo.local".chars() {
        app.on_key(KeyEvent::from(KeyCode::Char(caractere)));
    }
    app.on_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(app.connection_edit.is_none(), "o pop-up fecha ao salvar");
    let hosts: Vec<&str> = app
        .history
        .all()
        .iter()
        .map(|saved| saved.host.as_str())
        .collect();
    assert_eq!(
        hosts,
        ["novo.local", "outro"],
        "a posição na lista não muda"
    );
}

#[test]
fn esc_no_popup_descarta_a_edicao() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    app.on_key(KeyEvent::from(KeyCode::Char('x')));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    assert!(app.connection_edit.is_none());
    assert!(!app.should_quit);
    assert!(app.screen == Screen::Browse);
    assert_eq!(app.history.all()[0].host, "db.local");
}

#[test]
fn porta_invalida_mantem_o_popup_aberto_com_o_motivo() {
    let mut app = app_nas_conexoes();
    app.on_key(KeyEvent::from(KeyCode::Char('e')));
    app.on_key(KeyEvent::from(KeyCode::Tab));
    for _ in 0.."3306".len() {
        app.on_key(KeyEvent::from(KeyCode::Backspace));
    }
    for caractere in "abc".chars() {
        app.on_key(KeyEvent::from(KeyCode::Char(caractere)));
    }
    app.on_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(app.connection_edit.is_some());
    assert!(matches!(app.status, Status::Failed(ref erro) if erro == "porta inválida: abc"));
    assert_eq!(app.history.all()[0].port, 3306);
}

/// App na tela de trabalho, com o banco `app` selecionado.
fn app_nos_bancos() -> App {
    let mut app = app();
    app.databases = vec![String::from("app"), String::from("outro")];
    app.databases_state.select(Some(0));
    app.focus = Focus::Databases;
    app
}

/// Propriedades de banco como o worker devolveria, com o catálogo do servidor.
fn info_banco(read_only: Option<bool>) -> DatabaseInfo {
    DatabaseInfo {
        name: String::from("app"),
        charset: String::from("utf8mb4"),
        collation: String::from("utf8mb4_0900_ai_ci"),
        read_only,
        size_bytes: 12 * 1024 * 1024 * 1024,
        charsets: vec![
            Charset {
                name: String::from("latin1"),
                default_collation: String::from("latin1_swedish_ci"),
            },
            Charset {
                name: String::from("utf8mb4"),
                default_collation: String::from("utf8mb4_0900_ai_ci"),
            },
        ],
        collations: vec![
            Collation {
                name: String::from("latin1_swedish_ci"),
                charset: String::from("latin1"),
            },
            Collation {
                name: String::from("utf8mb4_0900_ai_ci"),
                charset: String::from("utf8mb4"),
            },
            Collation {
                name: String::from("utf8mb4_bin"),
                charset: String::from("utf8mb4"),
            },
        ],
    }
}

#[test]
fn resposta_com_as_propriedades_abre_o_popup_do_banco() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    let edicao = app.database_edit.as_ref().expect("pop-up aberto");
    assert_eq!(edicao.field, DatabaseField::Charset);
    assert_eq!(edicao.values.name, "app");
    assert_eq!(edicao.values.charset, "utf8mb4");
    assert_eq!(edicao.values.collation, "utf8mb4_0900_ai_ci");
    assert_eq!(
        edicao.fields(),
        vec![
            DatabaseField::Charset,
            DatabaseField::Collation,
            DatabaseField::ReadOnly
        ]
    );
}

#[test]
fn servidor_sem_opcao_de_leitura_esconde_o_campo() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(None)));

    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").fields(),
        vec![DatabaseField::Charset, DatabaseField::Collation]
    );

    // Tab dá a volta só nos campos que o servidor informa.
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Collation
    );
    app.on_key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Charset
    );
}

#[test]
fn enter_abre_a_lista_do_charset_no_valor_atual() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    app.on_key(KeyEvent::from(KeyCode::Enter));

    let lista = app
        .database_edit
        .as_ref()
        .expect("pop-up")
        .picker
        .as_ref()
        .expect("lista aberta");
    assert_eq!(lista.field, DatabaseField::Charset);
    assert_eq!(lista.items, ["latin1", "utf8mb4"]);
    assert_eq!(lista.index, 1, "a lista abre no valor atual");
}

#[test]
fn escolher_charset_leva_a_collation_padrao_dele() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let edicao = app.database_edit.as_ref().expect("pop-up");
    assert!(edicao.picker.is_none(), "a lista fecha ao escolher");
    assert_eq!(edicao.values.charset, "latin1");
    assert_eq!(edicao.values.collation, "latin1_swedish_ci");
}

#[test]
fn lista_de_collation_traz_so_o_charset_escolhido() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    let lista = app
        .database_edit
        .as_ref()
        .expect("pop-up")
        .picker
        .as_ref()
        .expect("lista aberta");
    assert_eq!(lista.field, DatabaseField::Collation);
    assert_eq!(lista.items, ["utf8mb4_0900_ai_ci", "utf8mb4_bin"]);
    assert_eq!(lista.index, 0);

    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").values.collation,
        "utf8mb4_bin"
    );
}

#[test]
fn j_k_na_lista_param_nos_limites() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Enter));

    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit
            .as_ref()
            .expect("pop-up")
            .picker
            .as_ref()
            .expect("lista")
            .index,
        1,
        "clampa no fim"
    );
    for _ in 0..3 {
        app.on_key(KeyEvent::from(KeyCode::Char('k')));
    }
    assert_eq!(
        app.database_edit
            .as_ref()
            .expect("pop-up")
            .picker
            .as_ref()
            .expect("lista")
            .index,
        0,
        "clampa no começo"
    );
}

#[test]
fn esc_na_lista_volta_ao_popup_sem_escolher() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Enter));
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    let edicao = app.database_edit.as_ref().expect("pop-up continua aberto");
    assert!(edicao.picker.is_none());
    assert_eq!(edicao.values.charset, "utf8mb4");
}

#[test]
fn enter_na_opcao_de_somente_leitura_alterna() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    // `j` anda nos campos como no resto do app.
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::ReadOnly
    );

    app.on_key(KeyEvent::from(KeyCode::Enter));
    let edicao = app.database_edit.as_ref().expect("pop-up");
    assert!(edicao.picker.is_none(), "sim/não não abre lista");
    assert_eq!(edicao.values.read_only, Some(true));

    app.on_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").values.read_only,
        Some(false)
    );
}

#[test]
fn j_k_andam_nos_campos_do_popup_do_banco() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Collation
    );
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Charset
    );

    // `k` no primeiro campo dá a volta e cai no último.
    app.on_key(KeyEvent::from(KeyCode::Char('k')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::ReadOnly
    );
    app.on_key(KeyEvent::from(KeyCode::Char('j')));
    assert_eq!(
        app.database_edit.as_ref().expect("pop-up").field,
        DatabaseField::Charset
    );
}

#[test]
fn s_no_popup_do_banco_fecha_sem_mudanca() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));

    app.on_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(app.database_edit.is_none());
    assert!(!matches!(app.status, Status::Failed(_)));
}

#[test]
fn esc_no_popup_do_banco_descarta() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseInfo(info_banco(Some(false))));
    app.on_key(KeyEvent::from(KeyCode::Char('x')));
    app.on_key(KeyEvent::from(KeyCode::Esc));

    assert!(app.database_edit.is_none());
    assert!(!app.should_quit);
    assert!(app.screen == Screen::Browse);
    assert!(app.databases_state.selected() == Some(0));
}

#[test]
fn banco_atualizado_vira_aviso_no_rodape() {
    let mut app = app_nos_bancos();
    app.on_response(Response::DatabaseUpdated {
        database: String::from("app"),
    });
    assert!(matches!(app.status, Status::Notice(ref aviso) if aviso == "banco app atualizado"));
}
