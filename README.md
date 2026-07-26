# LibreSync

Cliente de sincronização com Google Drive para Linux.

## Funcionalidades

- Sincronização bidirecional com Google Drive
- Autenticação OAuth2 com PKCE
- Sistema tray com ícone de status
- Notificações desktop
- Armazenamento seguro de tokens (Linux Secret Service + fallback criptografado)
- Resolução automática de conflitos
- Suporte a múltiplas contas
- Limitação de largura de banda

## Pré-requisitos

- Linux com systemd (GNOME, KDE ou qualquer WM)
- Rust 1.80+ (para compilar)
- Google Cloud Project com Drive API ativada

## Compilando

```bash
git clone <repo-url>
cd libresync
cargo build --release
```

## Configuração

1. Crie um OAuth 2.0 Client ID no Google Cloud Console (tipo "Desktop application")
2. Obtenha um refresh_token:
   ```bash
   GOOGLE_CLIENT_ID=seu_client_id cargo run --bin get_refresh_token
   ```
3. Configure o arquivo `~/.config/libresync/config.toml`:
   ```toml
   [google]
   client_id = "seu_client_id"
   client_secret = "seu_client_secret"
   refresh_token = "seu_refresh_token"

   [sync]
   local_dir = "~/LibreSync"
   ```

## Uso

```bash
# Modo terminal (padrão)
cargo run --bin libresync-core

# Modo tray (background com ícone)
cargo run --bin libresync-core -- --tray

# Modo CLI explícito
cargo run --bin libresync-core -- --cli

# Flags
#   --tray    Inicia com ícone na bandeja do sistema
#   --cli     Força modo terminal
#   --help    Mostra ajuda
```

## Variáveis de Ambiente

| Variável | Descrição |
|----------|-----------|
| `GOOGLE_CLIENT_ID` | ID do cliente OAuth2 |
| `GOOGLE_CLIENT_SECRET` | Segredo do cliente OAuth2 |
| `GOOGLE_REFRESH_TOKEN` | Token de refresh OAuth2 |
| `LIBRESYNC_BANDWIDTH_KBPS` | Limite de banda (0 = ilimitado) |

## Estrutura do Projeto

```
src/
├── auth/          Autenticação OAuth2 + PKCE
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
```

## Testes

```bash
# Testes unitários
cargo test --lib

# Testes de integração (requer credenciais Google)
GOOGLE_CLIENT_ID=... GOOGLE_REFRESH_TOKEN=... cargo test --features integration-test

# Testes completos
cargo test --features integration-test
```

## Licença

MIT
