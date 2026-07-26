# Spec: System Tray e Interface com Usuário (Tauri Commands + Frontend)

**Versão:** 1.0
**Status:** Rascunho
**Autor:** Engineering Team
**Data:** 2026-07-26
**Reviewers:** TBD

---

## 1. Resumo

Este documento especifica os componentes **System Tray** (ícone na bandeja do sistema com menu contextual) e **Interface com Usuário** (conjunto de telas Svelte + TypeScript + Tauri IPC) do LibreSync. A tray reflete em tempo real o estado da sincronização (sincronizado, sincronizando, erro, pausado, offline) e oferece acesso rápido às funções principais. A interface cobre quatro telas: Login, Principal, Preferências (4 abas) e Onboarding (3 passos). A comunicação frontend↔backend ocorre exclusivamente via Tauri commands (invocação síncrona de funções Rust) e Tauri events (push assíncrono do backend).

---

## 2. Contexto e Motivação

**Problema:** O usuário Linux não possui um cliente Google Drive nativo. As soluções existentes são instáveis, não têm interface gráfica ou exigem configuração manual via terminal. Sem uma system tray, o app precisa ficar visível o tempo todo. Sem notificações, o usuário não sabe quando algo falha.

**Oportunidade:** LibreSync será um dos poucos clientes Google Drive nativos e open source para Linux. Uma tray bem projetada transmite confiabilidade profissional. Uma UI Svelte moderna com IPC limpo diferencia o produto de ferramentas alternativas.

**Por que agora:** O PRD já define os requisitos de tray, notificações e telas (RF-05, RF-06, RF-15, RF-23). A arquitetura Tauri foi decidida. Este documento detalha o contrato entre frontend e backend para que implementações paralelas sejam possíveis.

---

## 3. Goals (Objetivos)

- [ ] G-01: O usuário pode ver o estado da sincronização na system tray sem abrir a janela
- [ ] G-02: O usuário pode pausar/retomar/abrir/sair da aplicação via menu da tray
- [ ] G-03: A tray reflete 5 estados visuais distintos (verde, azul animado, vermelho, cinza, branco)
- [ ] G-04: O usuário recebe notificações desktop para eventos importantes (erro, conflito, conclusão)
- [ ] G-05: O usuário completa o onboarding em até 3 cliques e começa a sincronizar
- [ ] G-06: O usuário configura sincronização, contas, rede e logs via janela de preferências com 4 abas
- [ ] G-07: O frontend consome dados do backend exclusivamente via Tauri commands + events, sem exposição de API HTTP

**Métricas de sucesso:**
| Métrica | Baseline | Target | Prazo |
|---------|----------|--------|-------|
| Tempo entre instalação e primeira sincronização | N/A | < 2 minutos | MVP |
| Cliques para iniciar sincronização | N/A | ≤ 3 (onboarding) | MVP |
| Notificações por minuto em operação normal | N/A | ≤ 1 | MVP |
| Latência tray → mudança de estado | N/A | < 500ms | MVP |

---

## 4. Non-Goals (Fora do Escopo)

- NG-01: **Tray com progresso de sincronização embutido** (sugestão RF-22) — v1.0
- NG-02: **Tray com mini-lista de arquivos recentes** — v1.0
- NG-03: **Preferência "Iniciar com o sistema"** — será implementada via systemd user service, não na GUI
- NG-04: **Notificações com ação "Abrir pasta"** — v1.0 (requer suporte do protocolo de notificações)
- NG-05: **Tema escuro/claro customizável** — seguirá o tema do sistema (GTK)
- NG-06: **Internacionalização (i18n)** — MVP apenas em português/inglês
- NG-07: **Janela minimizada para a tray em vez de fechar** — comportamento padrão: fechar minimiza para tray

---

## 5. Usuários e Personas

**Usuário primário:** Maria (desenvolvedora Linux, 28-40 anos). Usa Ubuntu/Arch. Quer sync automático em background. Interage principalmente pela tray. Abre a janela apenas para ver logs ou configurar.
**Usuário secundário:** Carlos (usuário corporativo, 35-50 anos). Usa Fedora. Quer configuração simples e notificações claras quando algo errado acontece.

**Jornada atual (sem a feature):**
1. Maria abre o navegador, acessa drive.google.com
2. Faz upload/download manual de cada arquivo
3. Esquece de sincronizar → perde versões
4. Alterna entre soluções de terceiros que quebram com frequência

**Jornada futura (com a feature):**
1. Maria instala LibreSync, vê onboarding de 3 passos
2. Faz login com Google, escolhe pasta, tudo pronto
3. O ícone verde na tray confirma que está sincronizado
4. Edita código no VS Code → sincronização acontece automaticamente
5. Se algo falha, notificação + ícone vermelho avisam

---

## 6. Requisitos Funcionais

### 6.1 System Tray

| ID | Requisito | Prioridade | Critério de Aceite |
|----|-----------|-----------|-------------------|
| RF-TRAY-01 | A tray deve ser criada na inicialização do app e permanecer ativa mesmo com a janela fechada | Must | Ao iniciar o app, o ícone aparece na bandeja. Ao fechar a janela, o ícone permanece |
| RF-TRAY-02 | O ícone da tray deve mudar conforme o estado de sincronização | Must | Cada estado (`Synced`, `Syncing`, `Error`, `Paused`, `Offline`) exibe um ícone PNG diferente |
| RF-TRAY-03 | O estado `Syncing` deve exibir animação de rotação no ícone | Should | O ícone azul rotaciona continuamente enquanto há jobs em execução |
| RF-TRAY-04 | O menu contextual da tray deve conter: status, abrir app, pausar/retomar, preferências, sair | Must | Todos os 5 itens estão presentes no menu |
| RF-TRAY-05 | O item de status no menu deve mostrar o email da conta ativa e o texto descritivo do estado | Should | Ex: "maria@gmail.com — Sincronizado" |
| RF-TRAY-06 | Clicar no ícone da tray deve abrir/focar a janela principal | Must | Evento de clique (esquerdo) dispara `show_window` |
| RF-TRAY-07 | O item "Pausar/Retomar" deve alternar o texto conforme o estado atual | Must | Exibindo "Pausar" quando sincronizando, "Retomar" quando pausado |
| RF-TRAY-08 | O item "Sair" deve encerrar completamente o processo (janela + tray) | Must | O processo termina, ícone desaparece |
| RF-TRAY-09 | A tray deve suportar atalhos de teclado: nenhum no MVP, mas o menu pode ser acessado via botão direito do mouse | Could | Menu aparece ao clicar com botão direito |

### 6.2 Notificações Desktop

| ID | Requisito | Prioridade | Critério de Aceite |
|----|-----------|-----------|-------------------|
| RF-NOTIF-01 | O sistema deve enviar notificação ao completar uma sincronização com arquivos novos | Should | Notificação "Sincronização concluída (N arquivos)" |
| RF-NOTIF-02 | O sistema deve enviar notificação ao detectar conflito | Must | Notificação "Conflito em [arquivo]" |
| RF-NOTIF-03 | O sistema deve enviar notificação em caso de erro de autenticação | Must | Notificação "Erro de autenticação — faça login novamente" |
| RF-NOTIF-04 | O sistema deve enviar notificação ao ficar offline | Should | Notificação "Conexão perdida — sincronização pausada" |
| RF-NOTIF-05 | O sistema deve enviar notificação ao restaurar conexão | Should | Notificação "Conexão restaurada — retomando sincronização" |
| RF-NOTIF-06 | O sistema NÃO deve emitir mais de 1 notificação por minuto em operação normal | Must | Rate limiter de notificações: máximo 1/min com throttle |
| RF-NOTIF-07 | Notificações de erro devem ter prioridade visual (urgência crítica) | Must | `Urgency::Critical` para erros de autenticação e conflitos |
| RF-NOTIF-08 | Notificações de info devem ter urgência normal | Should | `Urgency::Normal` para conclusão de sync |
| RF-NOTIF-09 | O sistema deve agrupar notificações do mesmo tipo (evitar repetição) | Should | Se 5 conflitos forem detectados em 1 minuto, notificar apenas "5 conflitos detectados" |
| RF-NOTIF-10 | Notificações não devem ser emitidas se a janela principal estiver visível e focada | Could | Supressão de notificação quando usuário já está vendo o app |

### 6.3 Tela de Login

| ID | Requisito | Prioridade | Critério de Aceite |
|----|-----------|-----------|-------------------|
| RF-LOGIN-01 | A tela de login deve ser exibida na primeira execução ou quando não há token válido | Must | App inicia sem token → mostra tela de login |
| RF-LOGIN-02 | Deve conter botão "Fazer login com Google" que dispara OAuth | Must | Clique abre navegador com URL de autorização |
| RF-LOGIN-03 | Deve mostrar estado de carregamento durante autenticação | Must | Spinner + "Aguardando autenticação..." |
| RF-LOGIN-04 | Deve mostrar mensagem de erro clara se OAuth falhar | Must | "Não foi possível autenticar: [motivo]" com botão "Tentar novamente" |
| RF-LOGIN-05 | Se token válido existir, deve redirecionar automaticamente para tela principal | Must | Ao iniciar, verifica token → se válido, pula login |

### 6.4 Tela Principal

| ID | Requisito | Prioridade | Critério de Aceite |
|----|-----------|-----------|-------------------|
| RF-MAIN-01 | A tela principal deve mostrar o email da conta ativa e estado geral | Must | Header com email, avatar (se disponível), status badge |
| RF-MAIN-02 | Deve listar as pastas em sincronização com seu status individual | Must | Cada pasta: nome, path local, status (sincronizando, pausado, erro) |
| RF-MAIN-03 | Deve mostrar uma lista de atividade recente (últimos eventos) | Must | Tabela com timestamp, tipo (↑ ↓ ✕), nome do arquivo, tamanho, resultado |
| RF-MAIN-04 | Deve ter botão para abrir Preferências | Must | Ícone de engrenagem no header |
| RF-MAIN-05 | Deve ter botão para pausar/retomar sincronização | Must | Botão no header que alterna entre "Pausar" e "Retomar" |
| RF-MAIN-06 | A lista de atividade deve atualizar em tempo real via eventos Tauri | Must | Ao receber `sync-event`, a lista adiciona o novo evento |
| RF-MAIN-07 | A tela deve suportar scroll para atividades extensas (limite: 100 eventos visíveis) | Should | Scroll infinito com paginação via comando `get_recent_events` |
| RF-MAIN-08 | Deve mostrar indicador de progresso quando há sincronização ativa | Must | Barra sutil no topo ou spinner com contagem "N arquivos restantes" |

### 6.5 Preferências (4 Abas)

| ID | Requisito | Prioridade | Critério de Aceite |
|----|-----------|-----------|-------------------|
| RF-PREF-01 | A janela de preferências deve ter abas: Geral, Contas, Sincronização, Rede, Logs | Must | 5 abas visíveis e navegáveis por clique |
| RF-PREF-02 | **Aba Geral:** O usuário pode configurar iniciar sync automático ao logar, notificar apenas erros | Should | Toggles/checkboxes salvos em `app_config` |
| RF-PREF-03 | **Aba Contas:** O usuário pode ver contas conectadas, remover conta, adicionar nova conta | Must | Lista de contas com botão "Adicionar conta" e "Remover" |
| RF-PREF-04 | **Aba Sincronização:** O usuário pode ver pastas sincronizadas, adicionar/remover pastas | Must | Lista com path local, status, botões "Adicionar pasta" e "Remover" |
| RF-PREF-05 | **Aba Rede:** O usuário pode limitar banda de upload/download e número de uploads paralelos | Should | Sliders para limite de banda, spinner para paralelismo |
| RF-PREF-06 | **Aba Logs:** O usuário pode visualizar logs em tempo real, filtrados por nível | Should | Últimas N linhas com filtro por nível (debug, info, warn, error) |
| RF-PREF-07 | Toda alteração em preferências deve ser persistida imediatamente (ou com botão "Salvar") | Must | Ao alterar toggle, muda imediatamente. Sliders têm botão "Aplicar" |
| RF-PREF-08 | Validação de entrada: path local deve existir e ser gravável; banda deve ser número positivo | Must | Erro inline no campo inválido |
| RF-PREF-09 | Preferências devem carregar o estado atual do backend ao abrir | Must | Ao abrir a janela, todos os campos refletem a configuração atual |

### 6.6 Onboarding (3 Passos)

| ID | Requisito | Prioridade | Critério de Aceite |
|----|-----------|-----------|-------------------|
| RF-ONB-01 | Na primeira execução, o app deve exibir o onboarding em vez da tela de login padrão | Must | Detecta `first_run = true` no config → exibe onboarding |
| RF-ONB-02 | Passo 1 — Boas-vindas + "Fazer login com Google" | Must | Tela com ilustração, texto explicativo, botão de login |
| RF-ONB-03 | Passo 2 — Após login, "Escolha a pasta de sincronização" com seletor de diretório | Must | Input + botão "Procurar" que abre diálogo nativo do FS |
| RF-ONB-04 | Passo 3 — "Tudo pronto!" com resumo do que foi configurado | Must | Email, path da pasta, botão "Começar a sincronizar" |
| RF-ONB-05 | O progresso do onboarding deve ser visual (barra de passos ou indicador 1/3, 2/3, 3/3) | Should | Indicador de progresso visível em cada passo |
| RF-ONB-06 | Ao completar o onboarding, `first_run` deve ser marcado como `false` | Must | Persistência imediata no `app_config` |
| RF-ONB-07 | O usuário pode fechar o onboarding e retomar depois? **Não** — deve completar para usar o app | Must | Onboarding é bloqueante, app não funciona sem concluir |
| RF-ONB-08 | Se o OAuth falhar no passo 1, o usuário pode tentar novamente sem perder progresso | Must | Botão "Tentar novamente" mantém o passo 1 |

### 6.7 Comunicação Frontend↔Backend (Tauri IPC)

| ID | Requisito | Prioridade | Critério de Aceite |
|----|-----------|-----------|-------------------|
| RF-IPC-01 | Todos os dados da UI devem vir de Tauri commands (Rust → frontend) | Must | Nenhum dado é exposto via HTTP, socket ou endpoint externo |
| RF-IPC-02 | Eventos assíncronos do backend (mudança de estado, progresso, erro) devem ser emitidos via Tauri events | Must | Frontend escuta eventos via `listen()` |
| RF-IPC-03 | Commands que modificam estado devem ser idempotentes quando possível | Should | Repetir o mesmo comando não causa efeito colateral duplicado |
| RF-IPC-04 | Commands devem retornar Result<Payload, AppError> com erros serializáveis | Must | Erros têm código, mensagem amigável, mensagem técnica e ação sugerida |

---

## 7. Requisitos Não-Funcionais

| ID | Requisito | Valor alvo | Observação |
|----|-----------|-----------|------------|
| RNF-01 | Latência de mudança de estado na tray | < 500ms entre mudança interna e atualização do ícone | Evento emitido síncrono dentro do runtime Tauri |
| RNF-02 | Consumo de RAM da tray | < 2 MB adicional ao core | Ícones PNG carregados em memória |
| RNF-03 | Consumo de RAM da janela (webview) | < 80 MB em idle | Webview do sistema não carrega runtime JS pesado |
| RNF-04 | Tamanho dos ícones da tray | 22×22 px (tamanho padrão da bandeja) | Monocromáticos com canal alpha |
| RNF-05 | Throttle de notificações | Máximo 1 notificação/min em operação normal | Janela deslizante de 60 segundos |
| RNF-06 | Disponibilidade da tray | 100% enquanto o processo estiver vivo | Tray nunca falha independentemente do estado do sync |
| RNF-07 | Acessibilidade | Navegação por Tab entre campos | Seguir WAI-ARIA para componentes Svelte |
| RNF-08 | Compatibilidade de distribuições | Ubuntu 22.04+, Fedora 38+, Debian 12+, Arch | Ícones .png com fallback para ícone padrão |
| RNF-09 | Responsividade | UI adaptável entre 800×600 e 1920×1080 | Layout flexível, sem scroll horizontal |

---

## 8. Design e Interface

### 8.1 Arquitetura Tauri (Commands, Eventos, App State)

```
┌─────────────────────────────────────────────────────┐
│  Frontend (Svelte + TypeScript)                     │
│  ┌───────────┐  ┌──────────┐  ┌──────────────────┐ │
│  │ App.svelte │  │ Tray.svel│  │ Preferences.svel │ │
│  │            │  │ (n/a)    │  │                  │ │
│  └─────┬─────┘  └──────────┘  └────────┬─────────┘ │
│        │                                │           │
│  ┌─────▼────────────────────────────────▼─────────┐ │
│  │            Tauri IPC Bridge                     │ │
│  │  invoke('cmd')  +  listen('event')              │ │
│  └────────────────────┬───────────────────────────┘ │
└───────────────────────┼─────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────┐
│  Backend (Rust Core)                                │
│  ┌──────────────────────────────────────────────┐   │
│  │  Tauri App State (State<AppState>)           │   │
│  │  - sync_engine: Arc<SyncEngine>              │   │
│  │  - config: Arc<RwLock<ConfigManager>>        │   │
│  │  - accounts: Arc<RwLock<AccountManager>>     │   │
│  │  - notifier: Arc<NotificationManager>        │   │
│  │  - tray_handle: TrayHandle                   │   │
│  └──────────────────────────────────────────────┘   │
│                                                      │
│  ┌────────────┐  ┌────────────┐  ┌───────────────┐  │
│  │ Tauri Cmds  │  │ Event      │  │ SyncEngine     │  │
│  │ (invoke)    │  │ Emitter    │  │ State Machine  │  │
│  └────────────┘  └────────────┘  └───────────────┘  │
└──────────────────────────────────────────────────────┘
```

**App State (Rust):**

```rust
pub struct AppState {
    pub sync_engine: Arc<SyncEngine>,
    pub config_manager: Arc<RwLock<ConfigManager>>,
    pub account_manager: Arc<RwLock<AccountManager>>,
    pub notification_manager: Arc<NotificationManager>,
    pub tray_handle: Arc<Mutex<Option<TrayHandle>>>,
    pub event_emitter: Arc<EventEmitter>,
    pub app_handle: Arc<Mutex<Option<AppHandle>>>,
}
```

### 8.2 System Tray: Criação, Menu, Ícones Dinâmicos

**Criação (Rust, `setup` hook):**

```rust
// src-tauri/src/tray/mod.rs
pub fn create_tray(app: &AppHandle, state: &AppState) -> TrayHandle {
    let icon = load_icon_for_state(SyncState::Idle); // ícone branco (offline inicial)

    let mut menu = Menu::new();
    let status_item = MenuItemBuilder::new("LibreSync — Inicializando...")
        .disabled(true)
        .build();
    let open_item = MenuItemBuilder::new("Abrir LibreSync")
        .accelerator("CmdOrCtrl+Shift+L")
        .build();
    let pause_item = MenuItemBuilder::new("Pausar sincronização")
        .accelerator("CmdOrCtrl+Shift+P")
        .build();
    let prefs_item = MenuItemBuilder::new("Preferências")
        .accelerator("CmdOrCtrl+,")
        .build();
    let quit_item = MenuItemBuilder::new("Sair")
        .accelerator("CmdOrCtrl+Q")
        .build();

    let menu = Menu::with_items([
        &status_item,
        &NativeMenuItem::separator(),
        &open_item,
        &pause_item,
        &NativeMenuItem::separator(),
        &prefs_item,
        &quit_item,
    ]);

    let tray = app.tray_by_id("main").unwrap();
    tray.set_menu(Some(menu)).unwrap();

    // Event handlers
    tray.on_menu_event(move |app, event| {
        match event.id.as_ref() {
            "open" => show_main_window(app),
            "pause" => toggle_pause(app, state),
            "prefs" => show_preferences_window(app),
            "quit" => quit_app(app),
            _ => {}
        }
    });

    tray.on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click { button: Left, .. } = event {
            show_main_window(tray.app_handle());
        }
    });

    tray
}
```

**Ícones por estado:**

| Estado | Arquivo | Cor | Descrição |
|--------|---------|-----|-----------|
| `Synced` | `tray-synced.png` | Verde (#27AE60) | Check simples ou pasta com check |
| `Syncing` | `tray-syncing.png` | Azul (#3498DB) | Seta circular — animação de rotação via `set_icon` alternado a cada frame |
| `Error` | `tray-error.png` | Vermelho (#E74C3C) | Ponto de exclamação ou X |
| `Paused` | `tray-paused.png` | Cinza (#95A5A6) | Símbolo de pausa (∥) |
| `Offline` | `tray-offline.png` | Branco (#FFFFFF) | Nuvem com risco ou círculo vazado |

**Animação no estado Syncing:** Alternar entre 4-8 frames de ícone (rotação) via set_icon a cada 200ms enquanto houver pelo menos 1 job rodando.

**Atualização do menu:**
```rust
pub fn update_tray_status(tray: &TrayHandle, state: &SyncState, email: &str, is_paused: bool) {
    let status_text = match state {
        SyncState::Idle | SyncState::Synced => format!("{} — Sincronizado", email),
        SyncState::Scanning | SyncState::Queuing | SyncState::Uploading | SyncState::Downloading => {
            format!("{} — Sincronizando...", email)
        }
        SyncState::Error => format!("{} — Erro na sincronização", email),
        SyncState::Paused => format!("{} — Pausado", email),
        SyncState::Offline => format!("{} — Offline", email),
        SyncState::Conflict => format!("{} — Conflito detectado", email),
    };
    // Atualiza MenuItem "status"
    // Atualiza pause_item label entre "Pausar" / "Retomar"
    // Atualiza ícone
}
```

### 8.3 Notificações: Tipos, Agrupamento, Rate Limit

```rust
// src-tauri/src/notifications/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    SyncCompleted { file_count: u32 },
    Conflict { file_name: String, folder: String },
    AuthError { message: String },
    ConnectionLost,
    ConnectionRestored,
    Error { file_name: String, message: String },
    Warning { message: String },
    Info { message: String },
}

pub struct NotificationManager {
    rate_limiter: RateLimiter,        // Token bucket: 1 per 60s
    last_notifications: VecDeque<(Instant, NotificationType)>,
    group_window: Duration,           // 10s para agrupar
}
```

**Rate Limiting:** Algoritmo token bucket com refil de 1 token a cada 60 segundos. Notificações de erro têm prioridade e consomem 0.5 token (permitindo 2 erros/min). Notificações info consomem 1 token.

**Agrupamento:** Em uma janela de 10s, notificações do mesmo tipo são agregadas:
- Múltiplos `Conflict` → "N conflitos detectados no minuto"
- Múltiplos `Error` → "N erros — verifique os logs"

**Implementação (Rust, `notify-rust` crate):**

```rust
pub fn send_notification(&self, notif_type: NotificationType) -> Result<(), AppError> {
    if !self.rate_limiter.allow(notif_type.priority()) {
        self.aggregate(notif_type);
        return Ok(());
    }

    let (title, body, urgency) = match &notif_type {
        NotificationType::SyncCompleted { file_count } => (
            "Sincronização concluída",
            &format!("{} arquivo(s) sincronizado(s)", file_count),
            Urgency::Normal,
        ),
        NotificationType::Conflict { file_name, folder } => (
            "Conflito detectado",
            &format!("{} em {}", file_name, folder),
            Urgency::Critical,
        ),
        NotificationType::AuthError { message } => (
            "Erro de autenticação",
            message,
            Urgency::Critical,
        ),
        NotificationType::ConnectionLost => (
            "Conexão perdida",
            "Sincronização pausada até conexão ser restaurada",
            Urgency::Normal,
        ),
        NotificationType::ConnectionRestored => (
            "Conexão restaurada",
            "Retomando sincronização",
            Urgency::Low,
        ),
        NotificationType::Error { file_name, message } => (
            &format!("Erro em {}", file_name),
            message,
            Urgency::Critical,
        ),
        NotificationType::Warning { message } => (
            "Aviso",
            message,
            Urgency::Normal,
        ),
        NotificationType::Info { message } => (
            "LibreSync",
            message,
            Urgency::Low,
        ),
    };

    Notification::new()
        .summary(title)
        .body(body)
        .icon("libresync")
        .urgency(urgency)
        .appname("LibreSync")
        .show()?;

    self.rate_limiter.consume();
    Ok(())
}
```

### 8.4 Telas Svelte

#### 8.4.1 Tela de Login

```
┌──────────────────────────────────────────────┐
│                                              │
│          ┌──────────────────────┐             │
│          │   LibreSync Logo     │             │
│          └──────────────────────┘             │
│                                              │
│           Sincronize seus arquivos            │
│           com Google Drive no Linux           │
│                                              │
│    ┌────────────────────────────────────┐     │
│    │  Fazer login com Google      ▶    │     │
│    └────────────────────────────────────┘     │
│                                              │
│       [loading: spinner + mensagem]           │
│       [erro: "Falha na autenticação: ..."]    │
│                                              │
└──────────────────────────────────────────────┘
```

**Estados:**
- **Idle:** Botão "Fazer login com Google" habilitado
- **Loading:** Botão desabilitado + spinner + "Aguardando autenticação no navegador..."
- **Error:** Mensagem de erro + botão "Tentar novamente"
- **Success:** Redireciona para Onboarding Passo 2 (ou tela principal se já configurado)

#### 8.4.2 Tela Principal

```
┌──────────────────────────────────────────────────────────────┐
│  LibreSync                                                    │
│  ──────────────────────────────────────────────────────────── │
│  [🟢 Sincronizado]         maria@gmail.com    [⏸] [⚙] [✕]  │
│                                                               │
│  Pastas em sincronização                                      │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ 📁 Meu Drive         /home/maria/Drive                  │ │
│  │    ✓ 1.234 arquivos · Última sync: há 2 min             │ │
│  │    ████████████░░░░ 3/5 arquivos (65%)                  │ │
│  ├─────────────────────────────────────────────────────────┤ │
│  │ 📁 Projetos          /home/maria/Projetos               │ │
│  │    ✓ 89 arquivos · Última sync: há 15 min               │ │
│  │    ⚠ Conflito: index.html                               │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  Atividade Recente                                            │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ ✅ 10:23  ↑  plano.pdf         1.2 MB   ✓ Concluído    │ │
│  │ ✅ 10:22  ↓  foto.png          3.5 MB   ✓ Concluído    │ │
│  │ ❌ 10:20  ↑  projeto.zip       10 MB    ⚠ Erro         │ │
│  │    [mais...]                                             │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

**Estados:**
- **Loading:** Skeleton screens para pastas e atividade
- **Empty (nenhuma pasta configurada):** "Nenhuma pasta em sincronização. Configure em Preferências."
- **Error:** Banner no topo "Erro de conexão — tentando reconectar..." com botão "Tentar agora"
- **Success:** Layout completo com dados

**Componentes:**
- `AppHeader.svelte` — logo, status badge, ações (pause, settings, close)
- `FolderList.svelte` — lista de pastas com status individual
- `FolderCard.svelte` — card individual com nome, path, status, contagem, barra de progresso
- `ActivityList.svelte` — tabela de eventos recentes com scroll infinito
- `ActivityRow.svelte` — linha individuall com ícone de tipo, nome, tamanho, status
- `StatusBadge.svelte` — badge colorido com texto do estado

#### 8.4.3 Preferências (5 Abas)

```
┌──────────────────────────────────────────────────────────────┐
│  Preferências                                        [✕]     │
│  ──────────────────────────────────────────────────────────── │
│                                                               │
│  [  Geral  ] [  Contas  ] [ Sincronização ] [  Rede  ] [ Logs ] │
│  ──────────────────────────────────────────────────────────── │
│                                                               │
│  [Conteúdo da aba ativa]                                      │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                                                         │ │
│  │   Exemplo: Aba Contas                                   │ │
│  │   ─────────────────────                                  │ │
│  │   ● maria@gmail.com                        [Remover]    │ │
│  │     Quota: 2.3 GB / 15 GB                               │ │
│  │                                                         │ │
│  │   [ + Adicionar conta ]                                 │ │
│  │                                                         │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  [Salvar preferências]                                        │
└──────────────────────────────────────────────────────────────┘
```

**Layout comum:** Sidebar vertical com abas à esquerda, conteúdo à direita. Botão "Salvar" no rodapé (exceto Logs, que é ao vivo).

**Aba Geral:**
- Toggle: "Iniciar sincronização automaticamente ao logar"
- Toggle: "Notificar apenas erros e conflitos" (suprime notificações info)
- Seletor de idioma (português/inglês)

**Aba Contas:**
- Lista de contas com email, nome, quota usada/total
- Botão "Adicionar conta" → dispara OAuth flow
- Botão "Remover" → confirmação "Tem certeza? Arquivos locais não serão removidos."
- Indicador de conta ativa (apenas uma conta como "principal")

**Aba Sincronização:**
- Lista de pastas configuradas com botão "Remover"
- Botão "Adicionar pasta" → diálogo nativo de seleção de diretório
- Por pasta: modo (bidirectional / upload_only / download_only)
- Toggle "Sincronizar subpastas"
- "Escolher pastas específicas para sincronizar" → lista de checkboxes com pastas do Drive (v1.0)

**Aba Rede:**
- Slider: "Limite de upload" (0 = ilimitado, 1–100000 KB/s)
- Slider: "Limite de download" (0 = ilimitado)
- Spinner: "Uploads paralelos" (1–8)
- Spinner: "Downloads paralelos" (1–8)
- Toggle: "Usar configurações de proxy do sistema" (v1.0, desabilitado no MVP)

**Aba Logs:**
- Caixa de texto rolável com logs em tempo real (máx 1000 linhas visíveis)
- Filtros: "Debug", "Info", "Warn", "Error" — combináveis
- Botão "Copiar logs" → copia para área de transferência
- Botão "Abrir arquivo de log" → abre diretório de logs no gerenciador de arquivos

#### 8.4.4 Onboarding (3 Passos)

**Passo 1 — Boas-vindas:**

```
┌──────────────────────────────────────────────┐
│                                              │
│     ┌─────────────────────────────┐          │
│     │   Ilustração (setup.svg)    │          │
│     └─────────────────────────────┘          │
│                                              │
│     Bem-vindo ao LibreSync!                  │
│     Sincronize seus arquivos com             │
│     Google Drive de forma simples            │
│     e segura.                                │
│                                              │
│     ● ● ○  1/3                              │
│                                              │
│     ┌──────────────────────────────┐         │
│     │  Fazer login com Google ▶   │         │
│     └──────────────────────────────┘         │
│                                              │
└──────────────────────────────────────────────┘
```

**Passo 2 — Escolher pasta:**

```
┌──────────────────────────────────────────────┐
│                                              │
│     Escolha onde sincronizar                 │
│                                              │
│     ┌────────────────────────────────┐       │
│     │  /home/maria/LibreSync        │ [📂]  │
│     └────────────────────────────────┘       │
│                                              │
│     A pasta selecionada será usada           │
│     para sincronizar com "Meu Drive"         │
│                                              │
│     ○ ● ○  2/3                              │
│                                              │
│          [Voltar]    [Continuar ▶]           │
│                                              │
└──────────────────────────────────────────────┘
```

**Passo 3 — Pronto:**

```
┌──────────────────────────────────────────────┐
│                                              │
│     ┌─────────────────────────────┐          │
│     │   Ilustração (check.svg)    │          │
│     └─────────────────────────────┘          │
│                                              │
│     Tudo pronto!                             │
│                                              │
│     Conta: maria@gmail.com                   │
│     Pasta: /home/maria/LibreSync             │
│                                              │
│     Seus arquivos serão sincronizados        │
│     automaticamente.                         │
│                                              │
│     ○ ○ ●  3/3                              │
│                                              │
│     ┌──────────────────────────────┐         │
│     │  Começar a sincronizar ▶    │         │
│     └──────────────────────────────┘         │
│                                              │
└──────────────────────────────────────────────┘
```

**Estados de UI por passo:**

| Estado | Comportamento |
|--------|---------------|
| Loading (passo 1) | Spinner no botão de login, "Abrindo navegador..." |
| Error (passo 1) | "Não foi possível autenticar. [mensagem]" + botão "Tentar novamente" |
| Loading (passo 2) | Validação da pasta (verificar permissão de escrita) |
| Error (passo 2) | "A pasta [path] não pode ser usada: [motivo]" |
| Success | Transição animada para próximo passo |

### 8.5 Diagrama de Navegação

```mermaid
graph TD
    START[App Inicia] --> CHECK{token válido?}
    CHECK -->|Não| ONB1[Onboarding 1/3: Login]
    CHECK -->|Sim| MAIN[Tela Principal]

    ONB1 -->|Login OK| ONB2[Onboarding 2/3: Pasta]
    ONB2 -->|Pasta OK| ONB3[Onboarding 3/3: Pronto]
    ONB3 -->|"Começar"| MAIN

    MAIN -->|"⚙ Prefs"| PREFS[Preferências]
    PREFS -->|Fechar| MAIN

    MAIN -->|Fechar janela| HIDE[App em background]
    HIDE -->|Click tray| MAIN
    HIDE -->|Menu tray "Abrir"| MAIN

    MAIN -->|Menu tray "Pausar"| PAUSED[Sync pausado]
    PAUSED -->|Menu tray "Retomar"| MAIN

    MAIN -->|"Sair"| EXIT[Processo encerra]
    HIDE -->|"Sair"| EXIT
```

---

## 9. Modelo de Dados (Persistência do Frontend)

O frontend não persiste dados diretamente. Toda persistência é feita via Tauri commands que escrevem no SQLite / config do backend. O modelo abaixo reflete os dados trafegados via IPC:

### 9.1 Tipos Compartilhados (Rust → TypeScript via Tauri)

```typescript
// src/lib/types.ts

type SyncState =
  | 'idle'
  | 'scanning'
  | 'syncing'     // uploading ou downloading
  | 'synced'
  | 'error'
  | 'paused'
  | 'offline'
  | 'conflict';

type SyncMode = 'bidirectional' | 'upload_only' | 'download_only';

type EventType =
  | 'sync_started'
  | 'sync_completed'
  | 'file_uploaded'
  | 'file_downloaded'
  | 'file_deleted'
  | 'conflict_detected'
  | 'error'
  | 'warning'
  | 'info'
  | 'paused'
  | 'resumed'
  | 'offline'
  | 'online';

type EventLevel = 'debug' | 'info' | 'warn' | 'error';

interface Account {
  id: string;
  email: string;
  display_name: string | null;
  avatar_url: string | null;
  is_active: boolean;
  quota_total: number;
  quota_used: number;
  last_sync_at: number | null;
}

interface SyncFolder {
  id: string;
  account_id: string;
  local_path: string;
  remote_path: string;
  sync_mode: SyncMode;
  is_enabled: boolean;
  file_count: number;
  status: SyncState;
  last_sync_at: number | null;
}

interface SyncEvent {
  id: number;
  event_type: EventType;
  file_path: string | null;
  file_size: number | null;
  message: string | null;
  level: EventLevel;
  created_at: number;
}

interface SyncProgress {
  total_jobs: number;
  completed_jobs: number;
  failed_jobs: number;
  current_file: string | null;
  progress_percent: number;
}

interface AppConfig {
  first_run_completed: boolean;
  poll_interval_active: number;
  poll_interval_idle: number;
  max_parallel_uploads: number;
  max_parallel_downloads: number;
  max_retries: number;
  log_level: string;
  auto_sync_on_login: boolean;
  notify_only_errors: boolean;
  bandwidth_upload_kbps: number | null;
  bandwidth_download_kbps: number | null;
  proxy_enabled: boolean;
  proxy_url: string | null;
  locale: string;
}

interface AppError {
  code: string;           // "AUTH_TOKEN_EXPIRED"
  message: string;        // "Sua sessão expirou"
  detail: string | null;  // "Token expirou em 2026-07-26T10:00:00Z"
  action: string | null;  // "Faça login novamente"
}
```

### 9.2 Estado Global do Frontend (Svelte Store)

```typescript
// src/lib/stores/app.ts
import { writable, derived } from 'svelte/store';

interface AppStore {
  // Auth
  isAuthenticated: boolean;
  activeAccount: Account | null;

  // Sync
  syncState: SyncState;
  syncProgress: SyncProgress | null;

  // Data
  folders: SyncFolder[];
  recentEvents: SyncEvent[];

  // Config
  config: AppConfig | null;

  // UI
  isPreferencesOpen: boolean;
  isOnboarding: boolean;
  onboardingStep: number;
  isOnline: boolean;
  error: AppError | null;
  isLoading: boolean;
}
```

---

## 10. Integrações e Dependências

### 10.1 Tauri Commands (Rust → Frontend)

Todos os commands são assíncronos (`#[tauri::command]`) e retornam `Result<T, AppError>`.

| Comando | Parâmetros | Retorno | Descrição |
|---------|-----------|---------|-----------|
| `get_sync_state` | — | `SyncState` | Estado atual da sincronização |
| `get_sync_progress` | — | `SyncProgress \| null` | Progresso detalhado (ou null se idle) |
| `get_active_account` | — | `Account \| null` | Conta ativa ou null |
| `get_accounts` | — | `Account[]` | Todas as contas configuradas |
| `add_account` | — | `Account` | Inicia OAuth flow, retorna conta adicionada |
| `remove_account` | `{ account_id: string }` | `void` | Remove conta e pastas associadas |
| `get_sync_folders` | — | `SyncFolder[]` | Todas as pastas em sincronização |
| `add_sync_folder` | `{ local_path: string, mode?: SyncMode }` | `SyncFolder` | Adiciona nova pasta para sincronizar |
| `remove_sync_folder` | `{ folder_id: string }` | `void` | Remove pasta da sincronização |
| `get_recent_events` | `{ limit?: number, offset?: number, level?: EventLevel }` | `SyncEvent[]` | Eventos de sincronização com paginação |
| `get_config` | — | `AppConfig` | Configuração atual |
| `update_config` | `{ config: Partial<AppConfig> }` | `AppConfig` | Atualiza configuração, retorna estado completo |
| `toggle_pause` | — | `SyncState` | Alterna entre pausado/ativo |
| `check_auth` | — | `boolean` | Verifica se token atual é válido |
| `open_select_folder_dialog` | — | `string \| null` | Abre diálogo nativo de seleção de diretório |
| `get_logs` | `{ level?: EventLevel, lines?: number }` | `string[]` | Últimas N linhas do arquivo de log |
| `set_log_level` | `{ level: EventLevel }` | `void` | Altera nível de log em tempo real |
| `get_storage_stats` | `{ account_id: string }` | `{ used: number, total: number }` | Quota de armazenamento |
| `retry_failed_jobs` | — | `number` | Re-enfileira jobs falhos, retorna quantidade |
| `resolve_conflict` | `{ conflict_id: string, resolution: string }` | `void` | Resolve conflito manualmente |
| `complete_onboarding` | `{ local_path: string }` | `void` | Finaliza onboarding (configura pasta + marca first_run) |

### 10.2 Tauri Events (Backend → Frontend Push)

Eventos emitidos via `app_handle.emit("event_name", payload)`.

| Evento | Payload | Disparo | Frequência |
|--------|---------|---------|-----------|
| `sync-state-changed` | `SyncState` | Estado muda | A cada transição de estado |
| `sync-progress` | `SyncProgress` | Progresso de jobs ativos | A cada 500ms enquanto sincronizando |
| `sync-event` | `SyncEvent` | Novo evento de sincronização | Por evento concluído |
| `folder-status-changed` | `{ folder_id: string, status: SyncState }` | Status de uma pasta muda | Por pasta |
| `auth-expired` | `{ account_id: string, email: string }` | Token expirou e refresh falhou | Raro |
| `account-added` | `Account` | Nova conta adicionada | Raro |
| `account-removed` | `{ account_id: string }` | Conta removida | Raro |
| `config-changed` | `AppConfig` | Configuração alterada | Por alteração |
| `connection-status` | `{ online: boolean }` | Status de rede mudou | A cada transição online/offline |
| `notification` | `NotificationType` | Notificação emitida (para debug UI) | Mesmo throttle que notificações |
| `error` | `AppError` | Erro não crítico ocorreu | Por erro |

### 10.3 Dependências

| Dependência | Tipo | Impacto se indisponível |
|-------------|------|------------------------|
| `notify-rust` (crate) | Obrigatória | Notificações desktop não funcionam |
| `tauri-plugin-notification` (plugin) | Obrigatória | Notificações alternativas via Tauri |
| `tauri` (2.x) | Obrigatória | App não funciona |
| `svelte` (4.x+) | Obrigatória | Frontend não compila |
| `typescript` | Obrigatória | Frontend não compila |
| WebKitGTK (webkit2gtk-4.1) | Obrigatória | Webview não abre |
| GTK3/4 tray (libappindicator / ayatana) | Obrigatória | Tray não aparece |
| SVG to PNG para ícones | Compilação | Ícones precisam ser .png na compilação |

---

## 11. Edge Cases e Tratamento de Erros

### 11.1 System Tray

| Cenário | Trigger | Comportamento esperado |
|---------|---------|----------------------|
| EC-TRAY-01: Tray não suportada | Ambiente sem systray (Wayland puro, alguns WMs) | App roda sem tray (apenas janela). Log warning na primeira inicialização |
| EC-TRAY-02: Múltiplos cliques rápidos | Usuário clica 5x no tray em 1s | Abre janela apenas 1x (debounce) |
| EC-TRAY-03: Menu travado | Estado inconsistente entre engine e menu | Menu sempre reflete o último evento recebido. Se event bus falhar, menu mostra "Estado desconhecido" |
| EC-TRAY-04: Ícone não carrega | Arquivo .png corrompido ou não encontrado | Fallback para ícone embutido como raw bytes no binário |
| EC-TRAY-05: Fechar menu sem ação | Usuário clica fora do menu | Nada. Menu fecha, app continua |
| EC-TRAY-06: "Sair" durante sincronização | Usuário clica Sair com jobs ativos | Confirmação: "Há N arquivos sendo sincronizados. Deseja sair mesmo assim?" — sim: cancela jobs e sai |

### 11.2 Notificações

| Cenário | Trigger | Comportamento esperado |
|---------|---------|----------------------|
| EC-NOTIF-01: Explosão de conflitos | 100 arquivos em conflito simultâneo | Agrupa: "100 conflitos detectados — verifique a pasta de conflitos". Não emite 100 notificações |
| EC-NOTIF-02: Notificação ignorada | Sistema DND (Não Perturbe) ativo | Notificação emitida mas não exibida pelo sistema. App não tem controle sobre isso |
| EC-NOTIF-03: Throttle ativo | 10 erros em 10 segundos | Apenas 1 notificação emitida. Internamente, contador de eventos suprimidos é logado |
| EC-NOTIF-04: Serviço de notificação morto | `notify-rust` retorna erro | Loga warning, não quebra o app |
| EC-NOTIF-05: App focado | Janela visível + notificação de info | Notificação suprimida (não enviada) |
| EC-NOTIF-06: Arquivo muito grande para título | Nome com 200 caracteres | Trunca com "..." no meio (ex: "projeto-final-v3...final.docx") |

### 11.3 Telas

| Cenário | Trigger | Comportamento esperado |
|---------|---------|----------------------|
| EC-UI-01: Token expira durante uso | Refresh token falha silenciosamente | Evento `auth-expired` → frontend exibe modal "Sessão expirada — faça login novamente" com botão de login |
| EC-UI-02: Pasta local removida externamente | Usuário deleta pasta de sincronização via file manager | Estado da pasta: `error` com mensagem "Pasta não encontrada". Oferece ação "Selecionar nova pasta" |
| EC-UI-03: Disco chego | Sem espaço para download | Notificação + pasta marca erro. Job pausa até espaço liberado |
| EC-UI-04: Onboarding incompleto | Fecha app no passo 2 | Ao reabrir, volta ao passo 2. `first_run` continua `true` |
| EC-UI-05: Múltiplos cliques em "Login" | Usuário clica 3x seguidas | Botão desabilitado durante OAuth flow. Segundo clique ignorado |
| EC-UI-06: Preferências com alterações não salvas | Usuário tenta fechar janela | Modal "Há alterações não salvas. Descartar?" |
| EC-UI-07: Seletor de pasta cancela | Usuário clica "Cancelar" no diálogo nativo | Nada — input mantém valor anterior, nenhum erro |
| EC-UI-08: Logs com 10MB de texto | Aba Logs carrega arquivo enorme | Virtual scrolling (renderiza apenas linhas visíveis). Filtro pré-aplicado |
| EC-UI-09: Conexão instável | Alterna online/offline 10x em 1 minuto | Throttle de evento `connection-status`: no máximo 1 evento/5s. UI não "pisca" |
| EC-UI-10: Conta removida com pastas ativas | Usuário remove conta | Pastas são removidas da lista com confirmação. Arquivos locais preservados |

### 11.4 Validação de Preferências

| Campo | Validação | Erro |
|-------|-----------|------|
| `local_path` (nova pasta) | Deve existir, ser diretório, ter permissão de escrita | "A pasta [path] não existe ou não pode ser escrita" |
| `local_path` (nova pasta) | Não pode ser subdiretório de outra pasta já configurada | "Esta pasta já está dentro de [path] que já está sendo sincronizada" |
| `bandwidth_upload_kbps` | Deve ser 0 (ilimitado) ou ≥ 1 e ≤ 1.000.000 | "Limite de upload deve ser 0 (ilimitado) ou entre 1 e 1.000.000 KB/s" |
| `bandwidth_download_kbps` | Mesmo que upload | "Limite de download deve ser 0 (ilimitado) ou entre 1 e 1.000.000 KB/s" |
| `max_parallel_uploads` | Inteiro entre 1 e 8 | "Uploads paralelos deve ser entre 1 e 8" |
| `max_parallel_downloads` | Inteiro entre 1 e 8 | "Downloads paralelos deve ser entre 1 e 8" |
| `locale` | Deve ser "pt-BR" ou "en-US" | "Idioma não suportado" |

---

## 12. Segurança e Privacidade

- **Autenticação:** O frontend nunca manipula tokens de acesso. O comando `check_auth` retorna apenas `boolean`. Tokens residem exclusivamente no Rust core e no Linux Secret Service
- **Autorização:** O frontend não tem acesso a dados do Google Drive sem permissão. Toda operação passa pelo sync engine que verifica a validade do token
- **Dados sensíveis:** Nenhum token, refresh_token ou secret trafega no IPC. O modelo `Account` retornado ao frontend nunca inclui campos de token
- **Auditoria:** Ações do usuário na UI (pause, resume, add_account, remove_account, change_config) são logadas no `sync_events` com event_type `info` e nível `info`. Erros de autenticação são logados com nível `error`
- **Comunicação IPC:** O protocolo Tauri IPC roda em canal interno (loopback). Não há exposição de rede. A superfície de ataque se limita ao que os commands expõem

---

## 13. Plano de Rollout

- **Estratégia:** Big bang no MVP. Feature flags não são necessárias para tray e UI, pois são a interface primária do app
- **Como reverter:** `git revert` do commit que introduziu os componentes. Como o app não funciona sem UI, a reversão impede o uso do app
- **Dependências pré-deploy:**
  - [ ] Sync engine em estado funcional (pelo menos state machine + jobs)
  - [ ] Auth OAuth flow implementado
  - [ ] SQLite com schema inicial
  - [ ] ConfigManager funcional
- **Monitoramento pós-deploy:**
  - Logs de erro no frontend (window.onerror capturado e enviado ao core)
  - Métrica de crashes na webview
  - Métrica de commands rejeitados (AppError retornado ao frontend)
  - Número de notificações emitidas por hora

---

## 14. Open Questions

| # | Pergunta | Impacto | Dono | Prazo |
|---|---------|---------|------|-------|
| OQ-01 | A animação de rotação no ícone da tray (Syncing) deve ser implementada com múltiplos PNGs ou com CSS/svg animado? Alternar PNGs a cada 200ms é suficiente? | Baixo — apenas estético | TBD | MVP |
| OQ-02 | O seletor de diretório nativo no onboarding (passo 2) deve ser o diálogo padrão do sistema ou uma árvore customizada Svelte? | Médio — UX no Linux varia por DE | TBD | MVP |
| OQ-03 | Deve haver um limite de linhas visíveis no visualizador de logs da aba Logs? 1000 linhas é suficiente ou precisa ser configurável? | Baixo | TBD | MVP |
| OQ-04 | No MVP, o "fechar janela" minimiza para tray ou encerra o app? Decidido: minimiza para tray. Mas precisa de confirmação? | Médio — comportamento padrão | Decidido: minimiza sem confirmação | MVP |
| OQ-05 | A janela de preferências deve ser uma janela separada (Tauri window) ou um modal dentro da janela principal? | Médio — arquitetura de UI | TBD | MVP |
| OQ-06 | O ícone da tray em Wayland puro (sem appindicator) funciona via StatusNotifierItem (SNI)? Precisamos de fallback? | Alto — afeta usabilidade em Fedora/Wayland | TBD | MVP |

---

## 15. Decisões Tomadas (Decision Log)

| Decisão | Alternativas consideradas | Racional |
|---------|--------------------------|---------|
| **Ícones tray em PNG (não SVG)** | SVG renderizado em runtime | Tray icons precisam ser PNG no tamanho fixo 22×22. SVG adicionaria complexidade de renderização |
| **notify-rust + tauri-plugin-notification** | Apenas notify-rust, apenas plugin Tauri | Dual: notify-rust para notificações nativas quando app está em background, plugin Tauri para notificações que podem ser acionadas a partir do frontend |
| **Menu tray via `tauri::menu`** | Menu custom via libappindicator | Tauri v2 tem API nativa de menu tray que funciona tanto com libappindicator quanto com StatusNotifierItem |
| **Rate limiter token bucket 1/min** | Count-based (max 10/h), sliding window | Token bucket é mais flexível: permite bursts pequenos (ex: 2 erros/min com custo 0.5 cada) |
| **Onboarding bloqueante** | Onboarding não-bloqueante (pode fechar e retomar) | App não faz sentido sem conta e pasta configuradas. Seria confuso para o usuário |
| **Evento `sync-progress` a cada 500ms** | A cada 1s, a cada 200ms | 500ms é suave para animações sem sobrecarregar IPC com centenas de eventos/min |
| **5 abas em Preferências (Geral, Contas, Sincronização, Rede, Logs)** | Abas separadas: Sincronização + Pastas, Aba única com tudo | 5 abas organiza bem os grupos lógicos. "Sincronização" combina opções de comportamento; "Rede" é distinto o suficiente para ser separado |

---

## Apêndice

### A. Glossário

| Termo | Definição |
|-------|-----------|
| **Tray** | Ícone na bandeja do sistema (system tray / notification area) |
| **Tauri Command** | Função Rust invocada pelo frontend via `invoke()` — mecanismo IPC síncrono/assíncrono |
| **Tauri Event** | Mensagem push do backend para o frontend — mecanismo IPC assíncrono |
| **AppState** | Estado global gerenciado pelo Tauri, injetado nos commands |
| **SyncState** | Enum que representa o estado atual da máquina de estados de sincronização |
| **Throttle** | Limitação de taxa de eventos (ex: máximo 1 notificação por minuto) |
| **Token Bucket** | Algoritmo de rate limiting com refil gradual |
| **SNI** | StatusNotifierItem — protocolo de tray para Wayland |
| **AppIndicator** | Biblioteca GTK para ícones na bandeja (fallback para X11) |

### B. Referências

- PRD: `../PRD.md`
- Tauri v2 Tray API: https://v2.tauri.app/reference/tray/
- Tauri v2 Commands: https://v2.tauri.app/develop/calling-rust/
- Tauri v2 Events: https://v2.tauri.app/develop/events/
- notify-rust crate: https://crates.io/crates/notify-rust

### C. Histórico de Revisões

| Versão | Data | Autor | Mudanças |
|--------|------|-------|---------|
| 1.0 | 2026-07-26 | Engineering Team | Criação inicial |
