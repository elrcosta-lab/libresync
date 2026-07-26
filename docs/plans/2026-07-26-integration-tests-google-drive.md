# Google Drive Integration Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement tasks sequentially.

**Goal:** Implementar testes de integração reais contra a API Google Drive usando a conta de teste `elrcostadev@gmail.com`.

**Architecture:** Camada `test_helpers` com `GoogleDriveTestClient` que abstrai autenticação OAuth2 + chamadas REST à Drive API v3. Testes criam/limpam pasta `.libresync-test` no Drive. Gatilhados por env var `LIBRESYNC_INTEGRATION_TEST=1`.

**Tech Stack:** Rust + reqwest (já no projeto), serde_json, env vars para credenciais, Google Drive API v3 REST.

---

### Pré-requisito (Google Cloud)

Antes de rodar os testes, o usuário precisa:

1. Criar um projeto no [Google Cloud Console](https://console.cloud.google.com)
2. Habilitar **Google Drive API**
3. Criar **OAuth 2.0 Client ID** (tipo "Desktop application")
4. Adicionar `http://localhost:65432/callback` como redirect URI autorizado
5. Anotar **Client ID** e **Client Secret**
6. Fazer o fluxo OAuth2 manual uma vez para obter **refresh_token** inicial

Os seguintes env vars precisam estar definidos:
```
GOOGLE_CLIENT_ID=xxxxx.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=GOCSPX-xxxxx
GOOGLE_REFRESH_TOKEN=1//xxxxx
```

---

### Task 1: Criar test helper `GoogleDriveTestClient`

**Files:**
- Create: `tests/common/drive_test_client.rs`
- Modify: `tests/common/mod.rs`
- Create: `Cargo.toml` (add feature flag)

**Step 1.1: Add feature flag to Cargo.toml**

Add to `Cargo.toml`:
```toml
[features]
integration-test = []
```

**Step 1.2: Write `tests/common/mod.rs`**

```rust
pub mod drive_test_client;
```

**Step 1.3: Create `tests/common/drive_test_client.rs`**

Struct `GoogleDriveTestClient` que:
- Lê `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_REFRESH_TOKEN` de env vars
- Faz refresh do token via `POST https://oauth2.googleapis.com/token`
- Mantém token em cache, refresh automático se expirado
- Expõe métodos HTTP para Drive API v3:
  - `list_files(parent_id) -> Vec<File>`
  - `upload_file(parent_id, name, content) -> File`
  - `download_file(file_id) -> Vec<u8>`
  - `get_metadata(file_id) -> File`
  - `delete_file(file_id)`
  - `create_folder(parent_id, name) -> File`
  - `find_test_folder() -> String` (cria `.libresync-test` se não existir)
  - `cleanup()` (remove `.libresync-test`)

Modelos:
```rust
struct DriveFile {
    id: String,
    name: String,
    mime_type: String,
    size: Option<i64>,
    created_time: String,
    modified_time: String,
    md5_checksum: Option<String>,
    parents: Option<Vec<String>>,
}

struct TokenResponse {
    access_token: String,
    expires_in: i64,
    scope: String,
    token_type: String,
}
```

Erros:
```rust
enum DriveTestError {
    MissingEnvVar(String),
    TokenRefreshFailed(String),
    ApiError { status: u16, body: String },
    NetworkError(String),
    CleanupError(String),
}
```

**Step 1.4: Test manually**

Run: `LIBRESYNC_INTEGRATION_TEST=1 cargo test --features integration-test -p libresync-core --test common`
Expected: compile (sem testes ainda)

**Step 1.5: Commit**

```
git add Cargo.toml tests/common/
git commit -m "test: add GoogleDriveTestClient helper for integration tests"
```

---

### Task 2: Auth integration tests

**Files:**
- Create: `tests/integration/auth_integration_test.rs`

**Step 2.1: Write the test file**

```rust
#[cfg(feature = "integration-test")]
mod tests {
    // Testes reais contra Google OAuth2
}
```

Tests:

**Test 2.1: `test_token_refresh_works`**
- Usa `GoogleDriveTestClient`
- Força refresh do access_token
- Verifica que retorna token não vazio
- Verifica que `expires_in` é ~3600

**Test 2.2: `test_token_is_valid_for_drive_api`**
- Faz refresh do token
- Usa token para chamar `GET https://www.googleapis.com/drive/v3/files?pageSize=1`
- Verifica HTTP 200
- Verifica que resposta contém `files` (array, pode ser vazio)

**Test 2.3: `test_invalid_token_returns_401`**
- Chama Drive API com token inválido `"Bearer invalid_token_here"`
- Verifica HTTP 401

**Step 2.2: Run tests**

Run: `LIBRESYNC_INTEGRATION_TEST=1 cargo test --features integration-test --test auth_integration_test 2>&1`
Expected: all pass

**Step 2.3: Commit**

```
git add tests/integration/auth_integration_test.rs
git commit -m "test: add Google OAuth2 integration tests"
```

---

### Task 3: Drive API CRUD integration tests

**Files:**
- Create: `tests/integration/drive_crud_integration_test.rs`

**Step 3.1: Write test file**

**Test 3.1: `test_list_root_files`**
- Lista arquivos na raiz do Drive
- Verifica HTTP 200
- Verifica que `files` é um array (pode ser vazio)

**Test 3.2: `test_create_and_delete_folder`**
- Cria pasta `_test_libresync_temp_folder_` na raiz
- Verifica retorna id, name, mimeType=application/vnd.google-apps.folder
- Apaga a pasta
- Verifica listagem não contém mais a pasta

**Test 3.3: `test_upload_and_download_text_file`**
- Upload de arquivo `.txt` com conteúdo "Hello from LibreSync integration test!"
- Verifica metadata (name, mimeType=text/plain)
- Download do arquivo
- Verifica conteúdo baixado é igual ao enviado
- Apaga arquivo

**Test 3.4: `test_upload_binary_file`**
- Upload de 100 bytes aleatórios
- Download e verifica integridade (exact match byte a byte)
- Apaga

**Test 3.5: `test_file_update`**
- Upload de arquivo
- Faz upload novamente com conteúdo diferente (mesmo nome)
- Verifica que o id é o mesmo (update em vez de create)
- Download verifica novo conteúdo
- Apaga

**Test 3.6: `test_get_metadata`**
- Upload de arquivo
- GET metadata por file_id
- Verifica campos esperados (name, mimeType, size, modifiedTime)
- Apaga

**Test 3.7: `test_nested_folder_operations`**
- Cria pasta `parent/`
- Cria pasta `parent/child/`
- Upload de arquivo em `parent/child/file.txt`
- Lista conteúdo de `parent/child/`
- Verifica arquivo está lá
- Remove recursivamente `parent/`

**Step 3.2: Run tests**

Run: `LIBRESYNC_INTEGRATION_TEST=1 cargo test --features integration-test --test drive_crud_integration_test 2>&1`
Expected: all pass

**Step 3.3: Commit**

```
git add tests/integration/drive_crud_integration_test.rs
git commit -m "test: add Google Drive CRUD integration tests"
```

---

### Task 4: Error handling integration tests

**Files:**
- Create: `tests/integration/error_handling_integration_test.rs`

**Step 4.1: Write test file**

**Test 4.1: `test_rate_limiting`**
- Faz 100 chamadas rápidas para listar arquivos
- Verifica que eventualmente recebe HTTP 429
- Verifica header `Retry-After` presente

**Test 4.2: `test_not_found`**
- Tenta baixar file_id inexistente (`"fake_id_12345"`)
- Verifica HTTP 404

**Test 4.3: `test_permission_error`**
- Tenta acessar file_id de um arquivo que o app não tem acesso
- Verifica HTTP 403 ou 404

**Test 4.4: `test_invalid_request`**
- Tenta criar arquivo sem nome (payload vazio)
- Verifica HTTP 400

**Step 4.2: Run tests**

Run: `LIBRESYNC_INTEGRATION_TEST=1 cargo test --features integration-test --test error_handling_integration_test 2>&1`
Expected: all pass (rate_limit pode ser flaky)

**Step 4.3: Commit**

```
git add tests/integration/error_handling_integration_test.rs
git commit -m "test: add Google Drive error handling integration tests"
```

---

### Task 5: Run full suite and verify

**Step 5.1: Run all integration tests**

Run: `LIBRESYNC_INTEGRATION_TEST=1 cargo test --features integration-test 2>&1`
Expected: all pass, clean exit

**Step 5.2: Run clippy**

Run: `cargo clippy --features integration-test -- -D warnings 2>&1`
Expected: clean

**Step 5.3: Verify unit tests still pass without feature flag**

Run: `cargo test 2>&1 | tail -3`
Expected: 199 passed, no integration tests run

**Step 5.4: Commit**

```
git add -A && git commit -m "test: complete Google Drive integration test suite"
git push
```

---

## Files Changed Summary

| File | Action |
|------|--------|
| `Cargo.toml` | Add `integration-test` feature |
| `tests/common/mod.rs` | Create (or update) with `pub mod drive_test_client;` |
| `tests/common/drive_test_client.rs` | Create - `GoogleDriveTestClient` |
| `tests/integration/auth_integration_test.rs` | Create - OAuth2 tests |
| `tests/integration/drive_crud_integration_test.rs` | Create - CRUD tests |
| `tests/integration/error_handling_integration_test.rs` | Create - error handling tests |
