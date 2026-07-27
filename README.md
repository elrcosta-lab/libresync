# LibreSync

![CI](https://github.com/elrcosta-lab/libresync/actions/workflows/ci.yml/badge.svg)

Cliente nativo de sincronização com Google Drive para Linux.

## Funcionalidades

- Sincronização bidirecional com Google Drive
- Interface gráfica Tauri WebView (Login, Dashboard, Configurações, Boas-vindas)
- Autenticação OAuth2 com PKCE + suporte a client_secret
- System tray com ícone de status dinâmico (synced/syncing/error/paused/offline)
- Notificações desktop (sync, conflitos, erros)
- Armazenamento seguro de tokens (Linux Secret Service + AES-256-GCM)
- Resolução automática de conflitos
- Monitoramento de arquivos em tempo real (inotify)
- Limitação de largura de banda
- Múltiplas contas Google
- Tela de boas-vindas na primeira execução com passo a passo

## Instalação

### Via .deb (Ubuntu/Debian)

```bash
# Build o pacote
./build-deb.sh

# Instale
sudo dpkg -i libresync_0.1.0_amd64.deb
sudo apt-get install -f  # instala dependências
```

### Compilando manualmente

```bash
# Requer: Rust 1.80+, libgtk-3-dev, libwebkit2gtk-4.1-dev
cargo build --release
```

## Primeiros passos

Na **primeira execução**, o LibreSync abre automaticamente uma janela de boas-vindas com o passo a passo:

1. Crie um **OAuth 2.0 Client ID** no [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
   - Tipo: **"Web application"** (o tipo Desktop app não exibe o campo de redirect URIs customizado)
   - Authorized redirect URIs: `http://localhost:65432/callback`
2. Ative a **Google Drive API** no mesmo projeto
3. Cole o **Client ID** (e opcionalmente o **Client Secret**) na tela de boas-vindas e clique em "Concluir configuração"
4. Clique com direito no ícone da bandeja → **Conectar conta Google**
5. O navegador abre para autorização — faça login e autorize
6. Pronto! A sincronização começa automaticamente

> **Tudo pela interface gráfica** — nenhum terminal necessário após a instalação.

Se precisar rever as instruções, clique no tray → **Boas-vindas**.

## Uso

```bash
# Modo tray (recomendado — ícone na bandeja)
libresync-core --tray

# Modo terminal
libresync-core

# Ajuda
libresync-core --help
```

| Comando | Descrição |
|---------|-----------|
| (sem argumentos) | Modo terminal com loop de sincronização |
| `--tray` | Inicia com ícone na bandeja do sistema |
| `--cli` | Força modo terminal (CLI puro) |

## Interface gráfica

| Tela | Descrição |
|------|-----------|
| **Boas-vindas** | Passo a passo para configurar credenciais Google (exibida na 1ª execução) |
| **Login** | Conectar conta Google, lista de contas |
| **Dashboard** | Status do sync, pause/resume, atividade recente, quota |
| **Configurações** | Client ID, Client Secret, pasta de sync, banda, auto-start, polling |

### Tray menu

| Item | Descrição |
|------|-----------|
| Conectar conta Google | Inicia fluxo OAuth2 |
| Configurar Client ID | Entrada via janela nativa (Zenity) |
| Configurar Client Secret | Entrada via janela nativa (Zenity) |
| **Boas-vindas** | Reabre o guia de configuração inicial |
| Pause Sync | Pausa/retoma a sincronização |
| Preferences | Abre a janela de configurações (WebView) |
| Quit | Sai do aplicativo |

O ícone do tray muda automaticamente conforme o estado:
- 🟢 Verde: sincronizado
- 🔵 Azul: sincronizando
- 🔴 Vermelho: erro
- ⚪ Cinza: pausado
- ⬜ Branco: offline

## Estrutura do projeto

```
src/
├── auth/          Autenticação OAuth2 + PKCE + callback server
├── config/        Configuração TOML (client_id, client_secret, first_run, etc.)
├── conflict/      Detecção e resolução de conflitos
├── db/            Persistência SQLite
├── drive/         Cliente Google Drive API
├── error_handler/ Retry e conectividade
├── instance/      Lock de instância única
├── keyring/       Armazenamento seguro de tokens
├── notification/  Notificações desktop
├── sync/          Motor de sincronização (state machine, job queue)
├── transfer/      Gerenciamento de transferências
├── ui/            Modelos de estado e interface (AppScreen, SyncStatus, UIConfig)
├── watcher/       Monitoramento de arquivos
├── autostart.rs   Auto-start com o sistema
├── main.rs        Ponto de entrada
└── tray_app.rs    App Tray (Tauri) — sync loop, OAuth, menu, ícone
gui/
├── index.html     Interface gráfica (Tauri WebView) — 4 telas
├── app.js         Lógica da interface (IPC, navegação, polling)
└── style.css      Tema escuro responsivo
icons/
├── icon.png       Ícone do aplicativo (512×512)
├── icon-256.png   Ícone (256×256)
├── icon-128.png   Ícone (128×128)
├── icon.ico       Ícone do aplicativo para Windows (multi-resolução)
├── tray.png       Ícone da bandeja (64×64)
└── tray-32.png    Ícone da bandeja (32×32)
resources/
└── icons/
    ├── icon.png            Ícone do app (256×256)
    ├── libresync-1024.png  Fonte do ícone (1024×1024)
    └── status/32x32/     Ícones de estado (synced, syncing, error, paused, offline)
```

## Configuração

O arquivo de configuração fica em `~/.config/libresync/config.toml`:

| Campo | Descrição |
|-------|-----------|
| `google.client_id` | Client ID do OAuth2 |
| `google.client_secret` | Client Secret (opcional, melhora confiabilidade do token) |
| `google.refresh_token` | Token de refresh (gerado automaticamente pelo login) |
| `sync.local_dir` | Pasta de sincronização (default: `~/LibreSync`) |
| `sync.poll_interval_secs` | Intervalo entre verificações remotas (default: 30s) |
| `sync.auto_start` | Iniciar sincronização automaticamente (default: true) |
| `first_run` | Flag de primeira execução (default: true) |

### Variáveis de Ambiente

| Variável | Descrição |
|----------|-----------|
| `GOOGLE_CLIENT_ID` | Fallback se não configurado na GUI |
| `GOOGLE_CLIENT_SECRET` | Fallback se não configurado na GUI |
| `GOOGLE_REFRESH_TOKEN` | Token de refresh (útil para testes headless) |
| `LIBRESYNC_BANDWIDTH_KBPS` | Limite de banda (0 = ilimitado) |

## Testes

```bash
# Testes unitários + integração
cargo test

# Cobertura total: 51 testes unitários + 52 testes de integração
```

## Pacote .deb

```bash
# Build do pacote
./build-deb.sh

# Instalação
sudo dpkg -i libresync_0.1.0_amd64.deb

# Teste em container
podman build -t libresync-test .
podman run --rm libresync-test libresync-core --help
```

## Troubleshooting

### A tela de boas-vindas não aparece

Se você já usou o LibreSync antes, o `first_run` no config.toml já está como `false`. Para testar novamente:

```bash
sed -i 's/first_run = false/first_run = true/' ~/.config/libresync/config.toml
```

Ou reabra pelo tray → **Boas-vindas**.

### Sincronização não está baixando arquivos

1. **Verifique os logs:** execute `libresync-core` (modo terminal) para mensagens detalhadas
2. **Token expirado:** faça login novamente pelo tray → "Conectar conta Google"
3. **Client ID incorreto:** verifique no tray → "Configurar Client ID" ou nas Configurações
4. **Pasta de sync:** o padrão é `~/LibreSync` (caminho absoluto)

### WebView não abre / IPC não funciona

A interface gráfica (Preferences, Boas-vindas) depende do runtime Tauri. Se a janela não abrir:
- Verifique se `libwebkit2gtk-4.1-dev` e `libgtk-3-dev` estão instalados
- Execute `libresync-core --tray` no terminal para ver erros de inicialização
- As ações principais (login, configurar credenciais, pause) funcionam pelo menu do tray mesmo sem WebView
- Fechar a janela com ✕ apenas a oculta — o app continua rodando no tray

## Licença

MIT
