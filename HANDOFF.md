# LibreSync — Sumário da Sessão

## 45 commits, 6 specs SDD, 51 testes unitários + 52 testes de integração

---

## O que foi construído

### Core (motor de sync)
- **Auth OAuth2** — PKCE, token exchange, refresh automático, callback server
- **Sync Engine** — state machine, job queue, detecção de mudanças
- **File Watcher** — monitoramento de arquivos com inotify
- **Transfer Managers** — upload/download com worker pool, retry, bandwidth limit
- **Conflict Resolution** — detecção e resolução automática de conflitos
- **Drive Api Client** — adapter completo para Google Drive API v3

### Persistência e segurança
- **SQLite** — accounts, sync_state, jobs persistidos
- **Keyring** — Linux Secret Service + AES-256-GCM fallback
- **Instance Lock** — flock em `/tmp/libresync.pid`

### Interface
- **System Tray** — ícone dinâmico (synced/syncing/error/paused/offline)
- **Notificações desktop** — sync, conflitos, erros via notify-rust
- **WebView Tauri** — 3 telas (Login, Dashboard, Configurações)
- **Tray menu** — Conectar conta Google, Configurar Client ID, Pause, Preferences, Quit
- **Zenity dialog** — entrada de Client ID via janela nativa

### DevOps
- **GitHub Actions** — CI (build, clippy, test) + Release (tag v*)
- **.deb package** — 6.3MB, script `build-deb.sh` reproduzível
- **Dockerfile** — teste em container Ubuntu 24.04
- **Documentação** — README, PRD v1.1, 6 specs

---

## Pendências Resolvidas (última sessão)

### ✅ Callback OAuth inicia antes de abrir navegador
**Problema:** O navegador era aberto antes do servidor de callback estar garantidamente escutando, causando `ERR_CONNECTION_REFUSED` em `localhost:65432/callback`.

**Correção:**
- O servidor de callback agora é iniciado em uma task separada antes de abrir o navegador.
- Adicionado pequeno delay (200ms) para garantir que o servidor esteja pronto.
- O callback é aguardado via `JoinHandle` em vez de chamar `wait_for_callback()` diretamente.

**Arquivos alterados:** `src/tray_app.rs`

### ✅ Panic de runtime aninhado no menu do tray
**Causa raiz:** O handler do menu "login" estava criando um novo runtime Tokio com `std::thread::spawn()` + `Runtime::new()` + `block_on()` dentro do contexto do Tauri. Isso causava o panic "Cannot start a runtime from within a runtime".

**Correção:**
- Substituído `std::thread::spawn()` + `block_on()` por `tauri::async_runtime::spawn()` no handler do menu "login".
- O Tauri fornece seu próprio runtime assíncrono que é seguro usar dentro dos handlers de eventos.

**Arquivos alterados:** `src/tray_app.rs`

### ✅ Panic de runtime aninhado no loop de sync
**Causa raiz:** O `main()` usa `#[tokio::main]` que cria um runtime Tokio. O loop de sync estava sendo spawnado com `tokio::spawn()` dentro do contexto do Tauri, mas o Tauri tem seu próprio runtime. Isso causava o panic "Cannot start a runtime from within a runtime".

**Correção:**
- Substituído `tokio::spawn()` por `tauri::async_runtime::spawn()` no `main.rs` para spawnar o loop de sync.
- O `tauri::async_runtime::spawn()` é seguro usar dentro do contexto do Tauri e usa o runtime do Tauri.
- O `engine` é compartilhado via `Arc<tokio::sync::Mutex<Option<SyncEngine>>>` (tokio::sync::Mutex em vez de std::sync::Mutex para ser Send+async).
- A função `do_oauth_flow()` foi atualizada para usar `.lock().await` em vez de `.lock().unwrap()`.

**Arquivos alterados:** `src/main.rs`, `src/tray_app.rs`

### ✅ Panic de runtime aninhado em chamadas síncronas
**Causa raiz:** Chamadas síncronas como `open::that()` e `std::process::Command::new("zenity")` estavam sendo executadas diretamente dentro de contextos assíncronos (handlers do Tauri e `do_oauth_flow`). Essas chamadas podem internamente usar `block_on` ou bloquear a thread, causando o panic "Cannot start a runtime from within a runtime".

**Correção:**
- Todas as chamadas `open::that()` foram envoltas em `tokio::task::spawn_blocking()` para executar em uma thread separada sem bloquear o runtime assíncrono.
- Todas as chamadas `std::process::Command::new("zenity")` foram envoltas em `tokio::task::spawn_blocking()` para o mesmo motivo.
- Adicionado tratamento de erro mais robusto para o erro 401 (token expirado), notificando o usuário para fazer login novamente.

**Arquivos alterados:** `src/tray_app.rs`, `src/main.rs`

### ✅ Panic de runtime aninhado após erro 401
**Causa raiz:** Após o erro `HTTP 401 Unauthorized`, o loop de sync chamava `notify_rust::Notification::show()` diretamente dentro de uma task async do Tokio. O `notify-rust` usa D-Bus internamente e pode bloquear/inicializar runtime, causando novamente o panic "Cannot start a runtime from within a runtime" no worker Tokio.

**Correção:**
- Notificações do loop de sync em `main.rs` agora rodam dentro de `tokio::task::spawn_blocking()`.
- Notificações chamadas por funções async em `tray_app.rs` também foram isoladas com `spawn_blocking()`.
- Build release e suíte completa de testes validados após a correção.

**Arquivos alterados:** `src/main.rs`, `src/tray_app.rs`

### ✅ State machine presa em Scanning após erro de listagem
**Causa raiz:** Quando `detect_changes()` falhava em `drive_client.list_files()` (ex: `HTTP 401 Unauthorized`), a função retornava erro imediatamente e deixava a state machine em `Scanning`. No ciclo seguinte, o engine tentava iniciar outro scan com a transição `Scanning -> Scanning`, que é inválida.

**Correção:**
- `detect_changes()` agora transiciona `Scanning -> Error -> Idle` antes de retornar erro de listagem.
- Adicionado teste de regressão `test_detect_changes_failure_returns_to_idle`.
- Novo pacote `.deb` gerado após build release e suíte completa de testes.

**Arquivos alterados:** `src/sync/engine.rs`, `tests/sync_engine_test.rs`

### ✅ O sync não está populando a pasta
**Causa raiz (fase 1):** `SyncEngine::handle_download_job()` chamava `drive_client.download()` mas descartava os bytes retornados — nunca escrevia no arquivo local.

**Causa raiz (fase 2):** Os handlers (`handle_download_job`, `handle_upload_job`, `handle_delete_job`) usavam o *nome humano* do arquivo (ex: `"arquivo.txt"`) como parâmetro para `get_metadata()` e `download()`. A API do Google Drive espera um `file_id` (UUID), não um nome. Isso causava 404 Not Found em todas as chamadas — o engine "completava" os jobs sem nunca transferir dados. O mock de teste aceitava nome como key no HashMap, mascarando o bug.

**Correção:**
- Adicionado `remote_file_id: Option<String>` ao `SyncJob` e ao schema SQLite (`jobs` table).
- `detect_changes()` e `on_remote_change()` agora guardam o `file_id` do Google Drive no job via `with_remote_file_id()`.
- Todos os handlers usam `job.remote_file_id` nas chamadas de API (`get_metadata`, `download`, `delete`). O nome humano é usado apenas para construir o caminho local.
- Adicionado método `write_downloaded_file()` que salva o conteúdo baixado no disco local.
- Teste TDD: `test_download_job_writes_file_to_disk` em `tests/sync_engine_test.rs`.

**Arquivos alterados:** `src/sync/job.rs`, `src/sync/engine.rs`, `src/db/connection.rs`, `src/db/jobs.rs`, `tests/sync_engine_test.rs`, `tests/ui_commands_test.rs` (fix de compilação)

### ✅ WebView Tauri IPC
**Causa raiz:** O frontend usava `window.__TAURI_INTERNALS__` — um detalhe de implementação interno do Tauri não garantido estar disponível. No Tauri v2, a API pública é exposta via `window.__TAURI__.core.invoke()` quando `withGlobalTauri: true`.

**Correção:**
- `tauri.conf.json`: adicionado `"withGlobalTauri": true` na seção `app`.
- `gui/app.js`: função `invoke()` atualizada para tentar primeiro `window.__TAURI__.core.invoke()` (API pública v2) e manter `__TAURI_INTERNALS__` como fallback.

**Arquivos alterados:** `tauri.conf.json`, `gui/app.js`

### ✅ Logging estruturado para diagnóstico
**Problema:** Os erros no loop de background do tray estavam sendo engolidos (`let _ = ...`), tornando impossível diagnosticar problemas em runtime.

**Correção:**
- Adicionados logs no loop de background (`tray_app.rs`) para rastrear `detect_changes()` e `process_queue()`.
- Adicionados logs em `detect_changes()` para mostrar quantos arquivos foram listados.
- Adicionados logs em `handle_download_job()` e `write_downloaded_file()` para rastrear downloads.
- Adicionados logs em `list_files()` do DriveApiClient para ver respostas da API.
- Adicionadas notificações desktop para erros críticos no sync.

**Arquivos alterados:** `src/tray_app.rs`, `src/sync/engine.rs`, `src/drive/client.rs`

---

## Pendências Ativas

### 🔍 Token OAuth não é atualizado após login (investigação em andamento)
**Problema:** O usuário faz login pelo tray, o OAuth parece funcionar (aparece "autorizado"), mas o engine continua usando o token antigo/expirado e retornando `HTTP 401 Unauthorized`.

**Logs adicionados para diagnóstico:**
- `[oauth]` — mostra quando token é obtido, salvo e engine é substituído
- `[sync]` — mostra quando engine é atualizado (via pointer)
- `[DriveApiClient]` — mostra client_id e refresh_token (primeiros 10 chars) usado na criação

**Próximos passos:**
1. Instalar novo .deb com logs
2. Executar `libresync-core --tray`
3. Fazer login pelo tray
4. Observar logs para identificar:
   - Se `refresh_token` está sendo retornado pelo Google
   - Se tokens estão sendo salvos no config.toml
   - Se novo engine está sendo criado com credenciais corretas
   - Se engine está sendo substituído no estado global
   - Se loop de sync está usando o engine atualizado

**Arquivos alterados:** `src/tray_app.rs`, `src/main.rs`, `src/drive/client.rs`

---

## Como usar

```bash
# Build + instalar
./build-deb.sh
sudo dpkg -i libresync_0.1.0_amd64.deb

# Executar (modo tray)
libresync-core --tray

# Configurar Client ID
# Clique direito no tray → Configurar Client ID

# Autenticar
# Clique direito no tray → Conectar conta Google
```
