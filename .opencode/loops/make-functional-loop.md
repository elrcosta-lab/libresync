---
nome: make-functional
categoria: Implementação
gatilho: manual
plataforma: opencode
base-teorica: TDD (RED-GREEN-REFACTOR) + integração contínua com verificação externa
---

# Loop: tornar o LibreSync funcional

## Descrição
Transforma a biblioteca `libresync-core` em um binário executável que sincroniza uma pasta local com o Google Drive, passando um teste de integração de ciclo completo.

## Use quando
A biblioteca tem todos os componentes implementados e testados, mas o sistema ainda não é um binário executável — faltam integração real com Drive API, persistência, e o ponto de entrada.

## Meta
`cargo test --features integration-test --test end_to_end_sync_test` passa do início ao fim: autentica → sobe arquivo local para o Drive → verifica consistência → limpa. Verificável via comando.

## Verificação (o check que manda)
A cada volta, rode:

```bash
cargo build 2>&1 && cargo test --features integration-test 2>&1 | tail -20
```

Ou, quando o teste alvo existir:

```bash
cargo test --features integration-test --test end_to_end_sync_test 2>&1
```

- **Check:** saída do comando — 0 falhas e o teste alvo passando.
- **Pronto =** `end_to_end_sync_test` aparece como `ok` no output.

## Passos da volta

0. **Setup (1ª volta):** pergunte ao usuário as entradas (nenhuma — loop autônomo). Crie `.opencode/loops/make-functional/estado.json` com as tarefas abaixo se não existir.

1. **Fotografe o estado atual:** rode `cargo test --features integration-test 2>&1 | grep 'test result:'` e registre quantos passam/falham.

2. **Ranqueie o alvo de maior impacto (pior primeiro):** leia `estado.json`, escolha a tarefa pendente de maior prioridade.

3. **Faça UMA mudança:** implemente a tarefa escolhida, chamando skills ou subagentes conforme necessário.

4. **Rode o check** com `bash` e mostre a saída na íntegra.

5. **Mantenha a mudança** só se nada regrediu (nenhum teste existente quebrou); senão reverta com `git checkout -- <arquivos>`.

6. **Registre o progresso** em `.opencode/loops/make-functional/estado.json` — marque a tarefa como concluída ou anote o bloqueio.

## Lista de tarefas (ordem de implementação)

Prioridade decrescente — implementar uma por volta, nesta ordem:

| # | Tarefa | Depende de |
|---|--------|-----------|
| 1 | **DriveApiClient** — adapter real para Google Drive API v3 usando `GoogleDriveTestClient` como base. Métodos: `list`, `get`, `upload`, `download`, `delete`. | — |
| 2 | **SyncEngine + DriveApiClient** — conectar `SyncEngine` ao `DriveApiClient` real (remover mocks). `detect_changes()` chama `list` remoto. | #1 |
| 3 | **TransferManager real** — `UploadManager`/`DownloadManager` usam `DriveApiClient` em vez de simulação com sleep. | #1 |
| 4 | **Config file** — ler `~/.config/libresync/config.toml` com pasta local, client_id, etc. | — |
| 5 | **Binário principal** — `src/main.rs` que lê config, inicializa engine, roda loop de sync. | #2, #3, #4 |
| 6 | **Persistence (SQLite)** — contas, estado de sync, job queue persistente via `rusqlite`. | — |
| 7 | **Keyring token storage** — `secret-service` crate para armazenar tokens no Linux Secret Service. | — |
| 8 | **HTTP callback server** — integrar servidor `localhost:65432` no fluxo de login (hoje só no `get_refresh_token`). | — |
| 9 | **FileWatcher real (polling)** — watcher que de fato escuta o sistema de arquivos (inotify via `notify` crate ou polling). | — |
| 10 | **end_to_end_sync_test** — teste de integração que cria pasta local, sobe arquivo, verifica no Drive, limpa. | #1–#9 |

## Estados de parada

- **sucesso:** `end_to_end_sync_test` passa (`ok` no output).
- **sem-progresso:** 3 voltas sem nenhuma tarefa marcada como concluída (nenhum teste passou a mais, nenhum código novo).
- **bloqueado:** o check mostra o mesmo erro 3 vezes consecutivas (ex: mesma falha de compilação). Pergunte ao usuário com `question` tool.
- **esgotado:** 20 voltas atingidas.

## Guardrails

- **Teto:** 20 voltas. Use `todowrite` para rastrear voltas restantes.
- **Push force:** proibido sem aprovação (`question` tool).
- **CI/GitHub Actions:** alterar só com pergunta ao usuário.
- **Commit e push:** liberados sem permissão extra, mas apenas após check passar.

## Memória / estado

Arquivo: `.opencode/loops/make-functional/estado.json`

Formato:
```json
{
  "volta": 0,
  "tarefas": {
    "1_drive_api_client": "pending",
    "2_sync_engine_real": "pending",
    "3_transfer_real": "pending",
    "4_config_file": "pending",
    "5_main_binary": "pending",
    "6_persistence": "pending",
    "7_keyring": "pending",
    "8_callback_server": "pending",
    "9_watcher_real": "pending",
    "10_e2e_test": "pending"
  },
  "tarefa_atual": null,
  "ultimo_check": null,
  "voltas_sem_progresso": 0,
  "erro_repetido": null
}
```

Estados possíveis: `pending`, `in_progress`, `done`, `blocked`, `skipped`.

## Sub-loops (não se aplica)

Este loop não aninha sub-loops. Cada volta implementa exatamente uma tarefa linear.

## Subagentes opencode utilizados

- **`general`** — implementação de cada tarefa. Invocado via `task` com descrição detalhada + contexto.
- **`explore`** (ocasional) — busca em código para entender padrões existentes antes de implementar.

## Skills invocadas

- **`sdd-spec`** — quando uma tarefa exigir spec nova antes de implementar.
- **`software-architecture`** — para decisões arquiteturais (ex: como estruturar o binário).

## Por que funciona

- **Check externo (`cargo test`)** garante que a parada não é baseada em auto-avaliação do modelo.
- **Fotografar o estado antes** (passo 1) impede regressão silenciosa.
- **Pior primeiro** (passo 2) maximiza o valor de cada volta: o item mais crítico é atacado antes.
- **Uma mudança por volta** isola falhas — se algo quebra, sabemos exatamente o que foi.
- **Sem-progresso e bloqueio** detectam loops inúteis cedo (3 voltas sem ganho ou 3 erros iguais).
- **Memória em disco** permite retomar após contexto estourado ou sessão interrompida.

## Como acionar

### Curto (via comando no TUI — recomendado)

Abra o opencode na raiz do LibreSync e carregue este documento manualmente. O agente segue os passos da volta iterativamente.

### Médio (via shell script)

```bash
for i in $(seq 1 20); do
  opencode run --continue "Execute a volta $i do loop make-functional, lendo estado de .opencode/loops/make-functional/estado.json"
done
```

## Métrica de saúde

custo por mudança aceita = tokens gastos / tarefas concluídas (em `estado.json`). Monitore com `opencode stats`.
