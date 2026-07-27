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
