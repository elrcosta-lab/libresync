---
nome: tdd-implement
categoria: Implementação
gatilho: manual
plataforma: opencode
base-teorica: "TDD (Test-Driven Development): RED → GREEN → REFACTOR. Spec-Driven Development: spec antes de código. SDD skill references/sdd_guide.md. 2305.19118 (Self-Refine), 2502.19559 (fail-stop checks)"
---

# TDD Implement — Implementação orientada a specs

## Descrição
Pega a próxima spec não implementada da fila (definida no PRD), implementa seguindo TDD estrito (RED → GREEN → REFACTOR), e só avança quando `cargo test && cargo clippy -- -D warnings` passa. Cada spec tem limite de 5 tentativas antes de declarar bloqueado.

## Use quando
Houver specs aprovadas (formato SDD) no diretório `specs/` que ainda não foram implementadas, e o projeto estiver em fase de codificação. O loop garante que cada spec é implementada com teste antes do código, sem regredir o que já funciona.

## Meta
Todas as 6 specs em `specs/` implementadas com testes passando e lint limpo. Verificável por comando.

Ordem de implementação (respeitando dependências):
1. `specs/auth-oauth2-spec.md` — dependência zero
2. `specs/sync-engine-spec.md` — depende de auth
3. `specs/file-watcher-spec.md` — independente, integra com engine
4. `specs/transfer-managers-spec.md` — depende de engine + auth
5. `specs/conflict-resolution-spec.md` — depende de engine
6. `specs/system-tray-ui-spec.md` — depende de todos os anteriores

## Verificação (o check que manda)
A cada volta, RODE o check com `bash` e MOSTRE a saída.
- **Check:** `cargo test && cargo clippy -- -D warnings`
- **Pronto =** exit code 0 para ambos (testes verdes + lint limpo).
- O agente lê a saída do terminal para decidir se continua. Se o exit code for 0, a volta foi bem-sucedida. Se não, diagnostica e tenta novamente (até 5 tentativas por spec).

## Passos da volta

0. **Setup (1ª volta apenas):** Leia o arquivo `estado.json` do estado. Se não existir, crie com `specs_pendentes: [auth-oauth2, sync-engine, file-watcher, transfer-managers, conflict-resolution, system-tray-ui]`, `spec_atual: null`, `tentativas: 0`, `voltas_sem_ganho: 0`, `historico: []`.

1. **Fotografe o estado atual:** Rode `cargo test 2>&1 | tail -5` e `cargo clippy -- -D warnings 2>&1 | tail -5` para estabelecer linha de base. Salve no estado como `baseline`.

2. **Selecione a próxima spec:** Leia `estado.json`. Se `spec_atual` for null ou se a spec atual já foi concluída, pegue a primeira de `specs_pendentes`. Carregue a skill `test-driven-development` para guiar a implementação. Leia o arquivo da spec em `specs/<spec>-spec.md` com `read`.

3. **Implemente seguindo TDD estrito (RED → GREEN → REFACTOR):**
   - **RED:** Escreva um teste que falhe para a próxima unidade não implementada da spec. Rode `cargo test` e prove que falha (exit code != 0). Cole a saída como evidência.
   - **GREEN:** Implemente o código mínimo para fazer o teste passar. Rode `cargo test` e prove que passa.
   - **REFACTOR:** Limpe o código (nomes, extração, remoção de duplicação) mantendo testes verdes.
   - Se a especificação exigir múltiplos ciclos TDD dentro de uma spec (ex: várias RFs), repita RED→GREEN→REFACTOR até a spec inteira estar implementada.

4. **Rode o check:** Execute `cargo test && cargo clippy -- -D warnings` com `bash`. Cole a saída completa.

5. **Mantenha ou reverta:**
   - Se check passou → mantenha as mudanças. Marque a spec como concluída em `estado.json` (move de `specs_pendentes` para `historico` com status `concluida`). Resete `tentativas` e `voltas_sem_ganho`.
   - Se check falhou → incremente `tentativas`. Se `tentativas < 5`, corrija o erro e volte ao passo 4. Se `tentativas >= 5`, reverta as mudanças (use `git checkout` nos arquivos alterados, ou `edit` para desfazer), marque a spec como `bloqueada` no histórico com o erro, e passe para a próxima spec.

6. **Registre o progresso:** Atualize `estado.json` via `write`:
   ```json
   {
     "specs_pendentes": [...],
     "spec_atual": "...",
     "tentativas": 0,
     "voltas_sem_ganho": 0,
     "historico": [
       {"spec": "auth-oauth2", "status": "concluida", "tentativas_usadas": 3, "voltas": 12}
     ],
     "ultima_volta": "2026-07-26T12:30:00Z",
     "baseline": "..."
   }
   ```

## Estados de parada
- **sucesso:** `specs_pendentes` está vazia (todas as specs implementadas) e `cargo test && cargo clippy -- -D warnings` passou na última volta.
- **sem-progresso:** 5 voltas sem que `specs_pendentes` diminua ou `tentativas` avance (sem melhora mensurável no estado).
- **bloqueado:** alguma spec atingiu 5 tentativas sem sucesso. O loop pergunta com `question` tool: "A spec X está bloqueada após 5 tentativas. Deseja: (a) pular e continuar, (b) revisar a spec, (c) parar o loop?"
- **esgotado:** todas as specs tentadas, mas algumas permanecem bloqueadas.

## Guardrails
- Teto: 5 tentativas por spec. 5 voltas sem progresso = sem-progresso.
- Aprovação humana antes de: pular uma spec bloqueada, modificar o arquivo `estado.json` manualmente, ou alterar specs que já passaram no check.
- Use `question` tool para solicitar confirmação antes de ações destrutivas (ex: `git reset --hard`, deletar arquivos).
- Cada volta deve fazer UMA mudança por ciclo TDD (um teste → uma implementação). Múltiplas mudanças só dentro do mesmo ciclo RED→GREEN→REFACTOR de uma RF.

## Memória / estado
Arquivo `.opencode/loops/tdd-implement/estado.json`:
- `specs_pendentes`: array de specs ainda não iniciadas
- `spec_atual`: spec em andamento (null se entre specs)
- `tentativas`: contagem de tentativas na spec atual
- `voltas_sem_ganho`: contagem de voltas sem progresso
- `historico`: array de objetos `{ spec, status, tentativas_usadas, voltas, erro }`
- `ultima_volta`: timestamp ISO
- `baseline`: snapshot do último check

## Subagentes opencode utilizados
- `explore` (via `task`): para analisar a estrutura de código existente, encontrar arquivos relevantes, e verificar padrões antes de escrever código.
- `general` (via `task`): para executar ciclos TDD completos quando a RF é complexa e cabe em um subagente isolado — o subagente recebe a RF da spec, implementa RED→GREEN→REFACTOR, e retorna o diff.

## Skills carregadas
- `test-driven-development`: carregada no início da implementação de cada spec para guiar o ciclo RED→GREEN→REFACTOR.
- `sdd-spec`: carregada se for necessário reavaliar a spec (ex: ambiguidade detectada durante implementação).

## Por que funciona
- **Check externo:** `cargo test && cargo clippy` é determinístico — o agente não se auto-avalia, ele lê o exit code. Isso elimina reward hacking de "achei que ficou bom".
- **Evidência na conversa:** a saída do check é colada a cada volta. O agente vê o resultado real.
- **TDD estrito:** RED antes de GREEN garante que o teste realmente testa algo (evita falso positivo de teste que sempre passa).
- **Ordem de dependências:** specs com dependência zero primeiro evitam bloqueio por falta de estrutura.
- **Teto por spec:** 5 tentativas impedam loop infinito em uma spec específica. Bloqueado pede ajuda humana.
- **Memória em disco:** sobrevive a falhas de sessão, permite `opencode run --continue`, e dá rastreabilidade.
- **Fotografa o antes:** linha de base do estado atual permite detectar regressão.

## Como acionar
### Médio (via shell + opencode run — para 15-50 voltas)
> Script que usa `opencode run --continue` a cada volta, com contexto fresco. Recomendado para este loop dado que implementar 6 specs pode levar dezenas de voltas.

```bash
#!/bin/bash
SESSION_ID="tdd-implement-$(date +%Y%m%d)"
for i in $(seq 1 50); do
  opencode run --continue -s "$SESSION_ID" \
    --agent build \
    "Execute a volta $i do loop tdd-implement. Leia o estado em .opencode/loops/tdd-implement/estado.json. Siga os passos da volta exatamente como descritos em .opencode/loops/tdd-implement-loop.md. Rode o check com bash e mostre a saída."
  # Verifica se parou
  STATE=".opencode/loops/tdd-implement/estado.json"
  if [ -f "$STATE" ]; then
    PENDENTES=$(python3 -c "import json; d=json.load(open('$STATE')); print(len(d.get('specs_pendentes',[])))" 2>/dev/null || echo "?")
    STATUS=$(python3 -c "import json; d=json.load(open('$STATE')); print(d.get('status',''))" 2>/dev/null || echo "")
    if [ -n "$STATUS" ]; then
      echo "Loop encerrado: $STATUS"
      break
    fi
  fi
done
```

### Longo (via subagentes + task — para 50+ voltas ou paralelizável)
> Para situações onde múltiplas specs poderiam ser implementadas em paralelo (ex: file-watcher e transfer-managers são ortogonais), um coordenador lê o estado e delega specs a subagentes `general` via task, cada um executando um ciclo TDD completo de uma RF.

## Métrica de saúde
custo por mudança aceita = tokens (ou R$) / nº de commits que sobreviveram ao check.
Monitore com `opencode stats --days N` para ver custo acumulado.
