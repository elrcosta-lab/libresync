# Spec: Desktop Daemon — Tray, Keyring, Notificações, Error Handling

**Versão:** 1.0
**Status:** Rascunho
**Data:** 2026-07-26

---

## 1. Resumo

Transformar o binário terminal `libresync-core` em um **daemon desktop** com ícone na bandeja do sistema, armazenamento seguro de tokens via Linux Secret Service, notificações desktop de eventos de sync, e tratamento robusto de erros de rede com retry inteligente.

---

## 2. Contexto e Motivação

**Problema:** Hoje o LibreSync roda exclusivamente no terminal, requer env vars para credenciais, e não se recupera de falhas de rede. Um usuário comum não consegue usar o app sem abrir um terminal e configurar variáveis de ambiente.

**Por que agora:** O MVP terminal está funcional (216 testes, e2e passando). O próximo passo lógico é a experiência desktop — rodar em background, não exigir terminal, e ser resiliente a falhas.

---

## 3. Goals

- [ ] G-01: Binário com flag `--daemon` que entra em background e mostra ícone na bandeja
- [ ] G-02: Tokens armazenados no Linux Secret Service (GNOME Keyring / KWallet)
- [ ] G-03: Notificações desktop para sync, conflitos e erros
- [ ] G-04: Retry inteligente com backoff exponencial e recuperação de rede

---

## 4. Non-Goals

- NG-01: Janela GUI principal — apenas tray + janela de preferências (sem explorer de arquivos)
- NG-02: Suporte a macOS/Windows — Linux exclusivo nessa versão
- NG-03: Agendamento avançado (ex: sync apenas em horário comercial)
- NG-04: Múltiplas contas simultâneas no tray

---

## 5. Usuários e Personas

**Usuário primário:** Usuário Linux desktop (GNOME/KDE) que quer sincronizar arquivos com Google Drive sem abrir terminal. Confortável com instaladores .deb/.rpm mas não com CLI.

**Jornada atual:** Edita arquivos localmente, faz upload manual pelo navegador (drive.google.com). Não tem sync automático.

**Jornada futura:** Instala o LibreSync, faz login uma vez (browser OAuth2), choose pasta de sync, e o app roda em background sincronizando automaticamente.

```
libresync-core (binário)
  ├── Tauri App (tray + notificações)
  │     ├── SystemTray (ícone + menu)
  │     ├── Notifications (notify-rust + fallback stdout)
  │     └── Preferences window (Tauri WebView)
  ├── Auth
  │     └── KeyringStorage (secret-service)
  │         └── Fallback: encrypted file (AES-256-GCM)
  └── Sync Engine
        └── ErrorHandler (retry + backoff + recovery)
```

---

## 6. Requisitos Funcionais

### 6.1 Tray Icon e Daemon

| ID | Requisito | Prio | Critério de Aceite |
|----|-----------|------|-------------------|
| RF-01 | O binário deve aceitar `--daemon` para rodar em background | Must | `libresync-core --daemon` retorna imediatamente, processo continua rodando |
| RF-02 | O tray deve mostrar ícone indicando status (synced/syncing/error/paused/offline) | Must | Ícone muda conforme `SyncStatus` da `src/ui/state.rs` |
| RF-03 | O menu do tray deve ter: Status, Pause/Resume, Configurações, Sair | Must | Cada item executa ação correspondente |
| RF-04 | Clicar em "Configurações" abre janela Tauri minimalista | Should | Janela mostra status, botão de login/logout, pasta de sync |
| RF-05 | O daemon deve parar graciosamente com SIGTERM/SIGINT | Must | `kill <pid>` finaliza sem corromper estado |

### 6.2 Keyring Storage

| ID | Requisito | Prio | Critério de Aceite |
|----|-----------|------|-------------------|
| RF-06 | Tokens devem ser armazenados no Linux Secret Service via `secret-service` crate | Must | Após login, token persiste entre execuções sem env var |
| RF-07 | Se Secret Service estiver indisponível, usar fallback criptografado AES-256-GCM | Must | Rodar sem keyring (ex: container) usa `~/.config/libresync/tokens/*.enc` |
| RF-08 | O provider deve ler do keyring automaticamente no startup | Must | Ao iniciar, conta fica ativa sem pedir env vars |
| RF-09 | `libresync-core logout` deve remover token do keyring | Should | Token removido, conta marcada como revoked |

### 6.3 Notificações Desktop

| ID | Requisito | Prio | Critério de Aceite |
|----|-----------|------|-------------------|
| RF-10 | Notificação "Sync concluído" com número de arquivos | Must | `notify-send` ou `notify-rust` mostra: "LibreSync: 5 arquivos sincronizados" |
| RF-11 | Notificação "Conflito detectado" com nome do arquivo | Must | Notificação com ação "Resolver" (abre docs) |
| RF-12 | Notificação "Erro de autenticação" se token expirar | Must | Usuário é alertado para reautenticar |
| RF-13 | Notificações agrupadas: múltiplos eventos em 60s viram uma notificação | Should | Rate limiting via `NotificationManager` existente em `src/ui/` |

### 6.4 Error Handling Robusto

| ID | Requisito | Prio | Critério de Aceite |
|----|-----------|------|-------------------|
| RF-14 | Retry com backoff exponencial (1s, 2s, 4s, 8s, 16s, max 30s) para erros de rede | Must | Após queda de rede, sync retoma automaticamente |
| RF-15 | Detecção de conectividade: ping periódico ao Google Drive | Must | Se 3 tentativas seguidas falharem, estado → Offline |
| RF-16 | Reconexão automática: quando conectividade volta, estado → Idle e sync retoma | Must | Após Offline detectar回复, sync continua |
| RF-17 | Timeout configurável por requisição (default 30s, max 120s) | Should | Requisições lentas não bloqueiam o engine |
| RF-18 | Graceful degradation: se Drive API retorna 503, retry sem notify | Must | Erro transitório não assusta o usuário |

### 6.5 Fluxo Principal (Startup)

1. Usuário executa `libresync-core` (ou `--daemon`)
2. Sistema tenta carregar token do keyring
3. Se token existe e é válido → inicia sync
4. Se token não existe → abre janela de login (browser OAuth2)
5. Após login, token salvo no keyring
6. Tray icon aparece com status "Synced" ou "Syncing"
7. Loop de sync roda em background, notificando conforme necessário
8. Em caso de erro de rede → retry → offline → recovery

---

## 7. Requisitos Não-Funcionais

| ID | Requisito | Alvo | Observação |
|----|-----------|------|------------|
| RNF-01 | Memória idle | < 50 MB RSS | Sem vazamento em execução de 24h |
| RNF-02 | CPU idle | < 1% | Pooling 30s sem carga |
| RNF-03 | Latência de notificação | < 2s entre evento e notificação | |
| RNF-04 | Startup time | < 3s até tray visível | Inclui carregar token do keyring |
| RNF-05 | Tamanho do binário | < 25 MB | Tauri adiciona ~10MB |

---

## 8. Dependências Novas

| Crate | Versão | Uso |
|-------|--------|-----|
| `tauri` | 2.x | App shell, tray, notificações, WebView |
| `secret-service` | 5.x | Linux Secret Service (GNOME Keyring) |
| `notify-rust` | 4.x | Desktop notifications (fallback: `notify-send`) |
| `aes-gcm` | 0.10 | AES-256-GCM para fallback criptografado |
| `hkdf` | 0.12 | Derivação de chave para fallback |

---

## 9. Edge Cases

| Cenário | Trigger | Comportamento |
|---------|---------|---------------|
| EC-01: Keyring desabilitado | DBus off, container, SSH | Fallback AES-256-GCM ativado automaticamente |
| EC-02: Token expirado durante sync | `ensure_valid_token` falha | → Offline → notificação → reautenticação |
| EC-03: Rede cai durante upload | reqwest timeout | Retry 5x com backoff → Offline → recovery |
| EC-04: Dois cliques em "Sair" | SIGTERM repetido | Segundo sinal ignorado, força kill após 5s |
| EC-05: Tray já rodando | Segundo `libresync-core` | Lock file em `/tmp/libresync.lock` → erro |
| EC-06: Notificação ignorada | Usuário não clica | Notificação some após timeout (padrão do SO) |
| EC-07: Fallback corrompido | Arquivo `.enc` editado | `AeadError` → marca conta como `requires_reauth` |

---

## 10. Segurança

- Tokens nunca em disco não criptografado (keyring primário, AES-256-GCM fallback)
- Chave do fallback derivada de HKDF(machine-id + salt)
- Nonce de 12 bytes aleatório para cada ciphertext (via OsRng)
- Logs nunca contêm tokens — apenas email e tipo de erro
- Lock file previne duas instâncias simultâneas

---

## 11. Decisões Técnicas

| Decisão | Opção Escolhida | Alternativas | Motivação |
|---------|----------------|--------------|-----------|
| Tray framework | Tauri 2.x | `gtk-rs`, `libappindicator` | Tauri já estava na arquitetura original; WebView para config |
| Keyring crate | `secret-service` | `keyring` crate, `libsecret` sys | Rust nativo, async, suporte a GNOME + KDE |
| Notificações | `notify-rust` | Tauri notification API | Funciona sem Tauri window (modo terminal) |
| Lock file | `/tmp/libresync.lock` (PID file) | SQLite lock, socket | Simples, confiável, sem dependência extra |

---

## 12. Open Questions

| # | Pergunta | Impacto |
|---|---------|---------|
| OQ-01 | Tauri 2.x tem suporte estável a system tray no Linux? | Arquitetura |
| OQ-02 | `secret-service` funciona em KDE Plasma e GNOME? | Testabilidade |
| OQ-03 | Precisamos de fallback para `libappindicator` se Tauri tray não funcionar? | Portabilidade |

---

## Apêndice

### Estrutura de diretórios esperada

```
src/
├── main.rs              # Ponto de entrada com --daemon
├── lib.rs               # Biblioteca (tudo que existe)
├── tray/                # NOVO
│   ├── mod.rs
│   ├── daemon.rs        # Daemonização + lock file
│   └── builder.rs       # Construção do tray Tauri
├── keyring/             # NOVO
│   ├── mod.rs
│   ├── keyring_storage.rs   # secret-service wrapper
│   └── encrypted_fallback.rs # AES-256-GCM
└── error_handler/       # NOVO
    ├── mod.rs
    ├── retry.rs         # Backoff + retry policy
    └── connectivity.rs  # Ping + estado de rede
```
