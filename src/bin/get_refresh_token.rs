use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::Command;

use libresync_core::auth::error::{AuthError, AuthResult};
use libresync_core::auth::session::PkceSession;

const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const LOCAL_PORT: u16 = 65432;

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '+' => result.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                }
            }
            _ => result.push(c),
        }
    }
    result
}

fn parse_query(path: &str) -> Vec<(String, String)> {
    if let Some(query_start) = path.find('?') {
        let query = &path[query_start + 1..];
        query
            .split('&')
            .filter_map(|pair| {
                let mut split = pair.splitn(2, '=');
                match (split.next(), split.next()) {
                    (Some(k), Some(v)) => Some((url_decode(k), url_decode(v))),
                    _ => None,
                }
            })
            .collect()
    } else {
        vec![]
    }
}

async fn exchange_code(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> AuthResult<TokenResponse> {
    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("code_verifier", code_verifier),
    ];

    if let Some(secret) = client_secret {
        params.push(("client_secret", secret));
    }

    let resp = client
        .post(OAUTH_TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| AuthError::NetworkError(e.to_string()))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if status.is_success() {
        serde_json::from_str::<TokenResponse>(&body)
            .map_err(|e| AuthError::StateError(format!("json: {}", e)))
    } else {
        Err(AuthError::StateError(format!("HTTP {}: {}", status, body)))
    }
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    scope: String,
    token_type: String,
    id_token: Option<String>,
}

#[tokio::main]
async fn main() -> AuthResult<()> {
    let client_id =
        env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID env var required");
    let client_secret = env::var("GOOGLE_CLIENT_SECRET").ok();
    let redirect_uri = format!("http://localhost:{}/callback", LOCAL_PORT);

    let session = PkceSession::new(&client_id);
    let auth_url = session.authorization_url(&redirect_uri);

    // 1. Start local TCP server
    let addr = format!("127.0.0.1:{}", LOCAL_PORT);
    let listener =
        TcpListener::bind(&addr).map_err(|e| AuthError::StateError(format!("bind: {}", e)))?;

    println!("✅ Local server listening on http://127.0.0.1:{}", LOCAL_PORT);
    println!();

    // 2. Open browser
    println!("🌐 Opening browser for Google authorization...");
    if Command::new("xdg-open").arg(&auth_url).spawn().is_err() {
        println!("Could not open browser automatically.");
        println!("Open this URL manually:");
        println!();
        println!("  {}", auth_url);
    } else {
        println!("  {}", auth_url);
    }
    println!();
    println!("⏳ Waiting for callback... (timeout: 5 minutes)");

    // 3. Accept callback connection with timeout
    let (mut stream, _peer) = listener
        .accept()
        .map_err(|_| AuthError::LoginTimeout)?;

    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(300)))
        .ok();

    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|_| AuthError::LoginTimeout)?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let path = parts.get(1).unwrap_or(&"/");
    let query_params = parse_query(path);

    // Drain remaining headers
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok() != Some(0) && line.trim().is_empty() {
            break;
        }
    }

    // 4. Extract code and state
    let code = query_params
        .iter()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.clone())
        .ok_or_else(|| AuthError::StateError("no code in callback".into()))?;

    let error = query_params.iter().find(|(k, _)| k == "error");
    if let Some((_, desc)) = error {
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n\
             <html><body><h1>❌ Authorization denied</h1>\
             <p>Error: {}</p></body></html>",
            desc
        );
        stream.write_all(response.as_bytes()).ok();
        return Err(AuthError::LoginDenied);
    }

    let state = query_params
        .iter()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.clone())
        .ok_or(AuthError::CsrfMismatch)?;

    session.validate_state(&state)?;

    // 5. Exchange code for tokens
    let client = reqwest::Client::new();
    let token_response = exchange_code(
        &client,
        &client_id,
        client_secret.as_deref(),
        &code,
        &session.code_verifier,
        &redirect_uri,
    )
    .await?;

    // 6. Send success response
    let html = format!(
        "<html><body style='font-family:sans-serif;text-align:center;padding:40px'>\
         <h1 style='color:#4CAF50'>✅ Authorization successful!</h1>\
         <p>Tokens valid for <strong>{}</strong> seconds.</p>\
         <p>You can close this tab and return to the terminal.</p></body></html>",
        token_response.expires_in
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(response.as_bytes()).ok();

    // 7. Print tokens
    println!("\n=== TOKENS ===");
    println!("Access Token:  {}", token_response.access_token);

    if let Some(rt) = token_response.refresh_token.as_deref() {
        println!("Refresh Token: {}", rt);
        println!();
        println!("=== EXPORT (adicione ao seu shell) ===");
        println!("export GOOGLE_CLIENT_ID='{}'", client_id);
        if let Some(ref secret) = client_secret {
            println!("export GOOGLE_CLIENT_SECRET='{}'", secret);
        }
        println!("export GOOGLE_REFRESH_TOKEN='{}'", rt);
    } else {
        println!();
        println!("⚠️  No refresh_token received.");
        println!("Make sure your OAuth client is configured as 'Desktop application'");
        println!("type in Google Cloud Console, or the user may need to re-authorize");
        println!("after revoking access at https://myaccount.google.com/permissions.");
    }

    println!("\nExpires In:    {} seconds", token_response.expires_in);
    println!("Scope:         {}", token_response.scope);

    Ok(())
}
