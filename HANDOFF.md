# LibreSync — Sumário da Sessão

## 74 commits, 8 specs SDD, 51 testes unitários + 52 testes de integração

---

## O que foi construído

### Core (motor de sync)
- **Auth OAuth2** — PKCE, token exchange, refresh automático, callback server, suporte a client_secret
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
- **System Tray** — ícone dinâmico (synced/syncing/error/paused/offline) — agora reflete o estado real do sync
- **Notificações desktop** — sync, conflitos, erros via notify-rust
- **WebView Tauri** — 4 telas (Boas-vindas, Login, Dashboard, Configurações)
- **Tela de boas-vindas** — exibida automaticamente na 1ª execução, com passo a passo e campos para Client ID/Secret
- **Tray menu** — Conectar conta Google, Configurar Client ID, Configurar Client Secret, Boas-vindas, Pause, Preferences, Quit
- **Zenity dialog** — entrada de Client ID/Secret via janela nativa

### DevOps
- **GitHub Actions** — CI (build, clippy, test) + Release (tag v*)
- **.deb package** — 6.4MB, script `build-deb.sh` reproduzível
- **Dockerfile** — teste em container Ubuntu 24.04
- **Documentação** — README, PRD v1.1, 6 specs

### Ícone personalizado
- **Fonte** — `resources/icons/libresync-1024.png` (1024×1024), Tux + triângulo estilo Google Drive + setas de sync
- **Ícone principal** — `icons/icon.png` (512×512) e variações 256×256, 128×128
- **Ícone tray** — 64×64 e 32×32
- **Status icons** — 5 ícones de estado (synced/syncing/error/paused/offline) em `resources/icons/status/32x32/`

### Segurança
- **LibreSync/ adicionado ao .gitignore** — evita novos vazamentos
- **git-filter-repo** — `LibreSync/` removido de todo o histórico git (73 commits reescritos)
- **Force push** — histórico limpo no GitHub (remote sobrescrito)
- **Debug log sanitizado** — `src/drive/client.rs` não loga mais refresh_token (mesmo truncado)

---

## Pendências Resolvidas

### ✅ Sync dir relativo → absoluto
**Problema:** `local_dir` padronizava como `"LibreSync"` (relativo), arquivos iam parar no CWD em vez de `~/LibreSync`.

**Correção:** `default_sync_dir()` usa `$HOME/LibreSync`. Config existente atualizado.

### ✅ Tray icon não refletia estado do sync
**Problema:** O sync loop rodava em `main.rs` sem acesso ao `AppUiState` ou `TrayIcon`. Ícone sempre verde.

**Correção:** Sync loop movido para dentro do `setup` hook do Tauri em `tray_app.rs`, com acesso ao `AppHandle`. Agora chama `update_tray()` a cada ciclo (syncing antes, synced/error depois).

### ✅ Tela de boas-vindas na primeira execução
**Problema:** Não havia onboarding — o usuário precisava saber como configurar sem instruções.

**Implementação:**
- `first_run: bool` no `LibreSyncConfig` (default `true`)
- Tela `#screen-welcome` no WebView com 4 passos numerados
- Campos para Client ID e Client Secret
- Botão "Concluir configuração" salva no config.toml e marca `first_run = false`
- Auto-exibe a janela na 1ª execução via `setup` hook
- Item "Boas-vindas" no tray menu para reabrir
- Fechar a janela com ✕ apenas oculta (não fecha o app)

### ✅ Campo Client Secret na tela de Configurações
**Problema:** Settings só tinha campo para Client ID, não para Client Secret.

**Implementação:**
- `client_secret: Option<String>` em `UIConfig`
- Campo `<input id="settings-client-secret">` no HTML
- `loadSettings()` e `saveSettings()` no JS
- `update_settings` salva `client_secret` no `config.toml`

### ✅ Tray icon agora reflete estado do sync
**Problema:** O ícone do tray permanecia verde independente do estado do engine.

**Correção:** Sync loop movido para o `setup` hook do Tauri, com acesso ao `AppHandle`. A cada ciclo define `SyncStatus` e chama `update_tray()`.

### ✅ State machine presa em Scanning/Queuing
**Problema:** Transições inválidas quando `detect_changes()` falhava ou ciclo completava sem reset.

**Correção:** Transição `Scanning → Error → Idle` em erro; transição `Queuing → Idle` adicionada.

### ✅ Sync não populava a pasta
**Problema:** `handle_download_job()` descartava bytes e usava nome em vez de `file_id`.

**Correção:** `remote_file_id` no `SyncJob`, `write_downloaded_file()` implementado.

### ✅ Runtime panics
**Problema:** Múltiplos panics "Cannot start a runtime from within a runtime" no Tauri + Tokio.

**Correção:** Uso de `tauri::async_runtime::spawn()`, `spawn_blocking()` para chamadas síncronas, isolamento de `notify-rust`.

### ✅ Estrutura de pastas ao baixar do Google Drive
**Problema:** Arquivos baixados iam todos para a raiz de `~/LibreSync`, ignorando a estrutura de pastas da nuvem. Ex: `Documentos/Trabalho/relatorio.pdf` baixava como `~/LibreSync/relatorio.pdf`.

**Causa raiz:** `detect_changes()` criava `SyncJob::new(&f.name, ...)` — apenas o nome do arquivo, ignorando o campo `parents` (IDs das pastas) retornado pela API.

**Correção:**
- `resolve_remote_path()` — função recursiva que rastreia a cadeia de `parents` folder IDs até a raiz para reconstruir o caminho completo (`Documentos/Trabalho/relatorio.pdf`)
- `detect_changes()` constrói um `folder_map` (HashMap id → nome+parents) a partir de todas as pastas listadas
- `on_remote_change()` usa `resolve_remote_path_for_file()` que resolve o caminho via chamadas `get_metadata` nos parent folders
- `write_downloaded_file()` já criava subdiretórios com `create_dir_all` — agora recebe o path completo

### ✅ Paginação no list_files (mais de 200 arquivos)
**Problema:** `list_files()` usava `pageSize=200` sem paginação. Contas com mais de 200 arquivos+pastas nunca listavam o restante, então o `folder_map` ficava incompleto e arquivos nem eram detectados.

**Correção:**
- `pageSize` aumentado de 200 para **1000** (máximo da API)
- Loop implementado sobre `nextPageToken` até listar todos os arquivos do Drive
- Logs informativos por página e total

### ✅ Tray abrindo tela errada (Boas-vindas → Configurações)
**Problema:** Clicar no tray → **Boas-vindas** abria a tela de **Configurações** em vez da boas-vindas. O handler de preferências nem sequer setava a tela correta.

**Causa raiz:** O frontend só lia `state.screen` uma vez durante `DOMContentLoaded`. Como a janela é criada oculta e reutilizada, `DOMContentLoaded` não disparava novamente ao reabrir; o frontend continuava na última tela exibida.

**Correção:**
- Handler "welcome" do tray já setava `AppScreen::Onboarding` — backend estava correto
- Handler "preferences" do tray agora seta `AppScreen::Preferences`
- `gui/app.js`: adicionado listener `visibilitychange` que chama `syncScreenFromBackend()` toda vez que a janela se torna visível
- `syncScreenFromBackend()` reconsulta o backend e navega para a tela correta (Onboarding, Preferences, Main/Login)

---

## Pendências Ativas

### Nenhuma

---

## Como usar

```bash
# Build + instalar
./build-deb.sh
sudo dpkg -i libresync_0.1.0_amd64.deb

# Executar (modo tray)
libresync-core --tray

# Na primeira execução a tela de boas-vindas abre automaticamente
# Para reabrir: clique direito no tray → Boas-vindas
```
