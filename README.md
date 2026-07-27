# LibreSync

![CI](https://github.com/elrcosta-lab/libresync/actions/workflows/ci.yml/badge.svg)

Cliente nativo de sincronização com Google Drive para Linux.

## Funcionalidades

- Sincronização bidirecional com Google Drive
- Interface gráfica completa (configuração, login, monitoramento)
- Autenticação OAuth2 com PKCE — sem senhas
- Sistema tray com ícone de status dinâmico
- Notificações desktop (sync, conflitos, erros)
- Armazenamento seguro de tokens (Linux Secret Service + AES-256-GCM)
- Resolução automática de conflitos
- Monitoramento de arquivos em tempo real (inotify)
- Limitação de largura de banda
- Múltiplas contas

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

1. Crie um **OAuth 2.0 Client ID** no [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
   - Tipo: "Desktop application"
   - Redirect URI: `http://localhost:65432/callback`
2. Ative a **Google Drive API** no mesmo projeto
3. Execute o LibreSync:

```bash
libresync-core --tray
```

4. Clique com direito no ícone da bandeja → **Preferences**
5. Vá em **Configurações** → cole seu **Google Client ID** → Salvar
6. Volte → clique **"Conectar conta Google"**
7. O navegador abre para autorização — faça login e autorize
8. Pronto! A sincronização começa automaticamente

> **Tudo pela interface gráfica** — nenhum terminal necessário após a instalação.

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
| **Login** | Conectar/conta Google, lista de contas |
| **Dashboard** | Status do sync, pause/resume, atividade recente, quota |
| **Configurações** | Google Client ID, pasta de sync, banda, auto-start, polling |

A janela de configuração abre pelo menu do tray (Preferences) ou clicando no ícone.

## Estrutura do projeto

```
src/
├── auth/          Autenticação OAuth2 + PKCE + callback server
├── config/        Configuração TOML
├── conflict/      Detecção e resolução de conflitos
├── db/            Persistência SQLite
├── drive/         Cliente Google Drive API
├── error_handler/ Retry e conectividade
├── instance/      Lock de instância única
├── keyring/       Armazenamento seguro de tokens
├── notification/  Notificações desktop
├── sync/          Motor de sincronização
├── transfer/      Gerenciamento de transferências
├── ui/            Modelos de estado e interface
├── watcher/       Monitoramento de arquivos
├── autostart.rs   Auto-start com o sistema
├── main.rs        Ponto de entrada
└── tray_app.rs    App Tray (Tauri)
gui/
├── index.html     Interface gráfica (Tauri WebView)
├── app.js         Lógica da interface
└── style.css      Tema escuro
```

## Variáveis de Ambiente

| Variável | Descrição |
|----------|-----------|
| `GOOGLE_CLIENT_ID` | ID do cliente OAuth2 (fallback se não configurado na GUI) |
| `GOOGLE_CLIENT_SECRET` | Segredo do cliente OAuth2 |
| `GOOGLE_REFRESH_TOKEN` | Token de refresh (gerado automaticamente pelo login) |
| `LIBRESYNC_BANDWIDTH_KBPS` | Limite de banda (0 = ilimitado) |

## Testes

```bash
# Testes unitários completos
cargo test

# Testes de integração (requer credenciais Google reais)
GOOGLE_CLIENT_ID=... GOOGLE_CLIENT_SECRET=... GOOGLE_REFRESH_TOKEN=... \
  cargo test --features integration-test

# Cobertura total: 51 testes unitários + 6 testes de integração
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

### Sincronização não está baixando arquivos

Se a autenticação funciona mas a pasta de sync não é populada:

1. **Verifique os logs:** execute `libresync-core` (modo terminal) para ver mensagens detalhadas
2. **Token expirado ou inválido:** faça login novamente pelo tray → "Conectar conta Google"
3. **Client ID incorreto:** verifique no tray → "Configurar Client ID" se o valor está correto
4. **Arquivos vazios no Drive:** `detect_changes()` lista arquivos com `trashed=false` — se sua pasta do Drive está vazia, não há nada para baixar
5. **Polling ativo:** o engine verifica mudanças remotas a cada 30s (padrão) — aguarde o ciclo

### WebView não abre / IPC não funciona

A interface gráfica (Preferences) depende do runtime Tauri. Se a janela não abrir:
- Verifique se `libwebkit2gtk-4.1-dev` e `libgtk-3-dev` estão instalados
- Execute `libresync-core --tray` diretamente no terminal para ver erros de inicialização
- As ações principais (login, configurar Client ID, pause) funcionam pelo menu do tray mesmo sem WebView

## Licença

MIT
