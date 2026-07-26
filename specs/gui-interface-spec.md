# Spec: Interface Gráfica de Configuração (Tauri WebView)

**Versão:** 1.0
**Status:** Rascunho
**Data:** 2026-07-26

---

## 1. Resumo

Interface gráfica completa para configuração e monitoramento do LibreSync, acessível via janela Tauri WebView. Todas as operações do app (login, sync, configurações) devem ser possíveis através da GUI.

---

## 2. Telas

### 2.1 Login
- Botão "Conectar conta Google" que abre o fluxo OAuth2
- Lista de contas conectadas (se houver)
- Botão "Remover conta"

### 2.2 Dashboard
- Status atual do sync (Synced/Syncing/Paused/Error/Offline)
- Botão Pause/Resume
- Atividade recente (últimos eventos de sync)
- Conta ativa com email e quota

### 2.3 Configurações
- Pasta de sync (seletor de diretório + input manual)
- Limite de banda (KBPS, 0 = ilimitado)
- Auto-start toggle
- Polling interval (segundos)
- Botão "Salvar"

---

## 3. IPC Commands (Tauri)

A comunicação frontend-backend usa `tauri::command`:

| Comando | Parâmetros | Retorno | Descrição |
|---------|-----------|---------|-----------|
| `get_state` | — | `AppUiState` | Estado completo da UI |
| `login` | — | `bool` | Inicia fluxo OAuth2 |
| `logout` | `account_id` | `bool` | Remove conta |
| `toggle_pause` | — | `bool` | Pause/Resume sync |
| `get_activity` | `limit` | `Vec<SyncActivity>` | Eventos recentes |
| `update_settings` | `Settings` | `bool` | Salva configurações |
| `get_settings` | — | `Settings` | Carrega configurações |
| `select_folder` | — | `String` | Abre seletor de pasta |

---

## 4. Frontend

### Arquivos
- `gui/index.html` — Estrutura SPA com navegação entre telas
- `gui/style.css` — Tema escuro moderno, responsivo
- `gui/app.js` — Lógica JS, chamadas Tauri IPC, state management

### Design
- Tema escuro (#1a1a2e background, #0078d4 accent)
- Fonte system-ui sans-serif
- Layout responsivo (480x320 mínimo, adaptável)
- Transições suaves entre telas
- Loading states e feedback de erro

### Comportamento
- SPA sem recarregamento de página (todas as telas em um HTML)
- Comunicação via `window.__TAURI__.invoke()`
- Atualização periódica do estado via `get_state()` a cada 5s
- Navegação via tabs ou botões de ação

---

## 5. Backend (Tauri Commands)

### `src/tray_app.rs`
Adicionar os comandos Tauri:

```rust
#[tauri::command]
fn get_state() -> AppUiState { ... }

#[tauri::command]
fn toggle_pause() -> bool { ... }

#[tauri::command]
fn get_activity(limit: usize) -> Vec<SyncActivity> { ... }
```

### `src/main.rs`
O modo `--tray` deve iniciar o Tauri com a GUI funcional.

---

## 6. Segurança

- CSP configurado para permitir apenas scripts do próprio app
- Navegação restrita a recursos locais
- Comandos IPC validam entrada do frontend

---

## 7. Testes

- Testar que os comandos IPC retornam dados corretos
- Testar que a UI reflete mudanças de estado
- Testar que o seletor de pasta abre
