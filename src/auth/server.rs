use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::auth::error::{AuthError, AuthResult};

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

#[derive(Debug, Clone)]
pub struct AuthorizationCode {
    pub code: String,
    pub state: String,
}

pub struct CallbackServer {
    port: u16,
    timeout: Duration,
}

impl Default for CallbackServer {
    fn default() -> Self {
        Self::new()
    }
}

impl CallbackServer {
    pub fn new() -> Self {
        Self {
            port: 65432,
            timeout: Duration::from_secs(300),
        }
    }

    pub fn with_port(port: u16) -> Self {
        Self {
            port,
            timeout: Duration::from_secs(300),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn wait_for_callback(&self, expected_state: &str) -> AuthResult<AuthorizationCode> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|_| AuthError::PortUnavailable)?;

        let accept_fut = async {
            let (mut stream, _) = listener
                .accept()
                .await
                .map_err(|_| AuthError::LoginTimeout)?;

            let mut reader = BufReader::new(&mut stream);
            let mut request_line = String::new();
            reader
                .read_line(&mut request_line)
                .await
                .map_err(|_| AuthError::LoginTimeout)?;

            let parts: Vec<&str> = request_line.split_whitespace().collect();
            let path = parts.get(1).unwrap_or(&"/");
            let query_params = parse_query(path);

            let mut header_buf = String::new();
            loop {
                header_buf.clear();
                if reader.read_line(&mut header_buf).await.map_err(|_| AuthError::LoginTimeout)? == 0
                    || header_buf.trim().is_empty()
                {
                    break;
                }
            }

            let error = query_params.iter().find(|(k, _)| k == "error");
            if let Some((_, desc)) = error {
                let body = format!(
                    "<html><body style='font-family:sans-serif;text-align:center;padding:40px'>\
                     <h1 style='color:#e74c3c'>Authorization denied</h1>\
                     <p>Error: {}</p></body></html>",
                    desc
                );
                let response = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\n\r\n{}",
                    body
                );
                stream.write_all(response.as_bytes()).await.ok();
                return Err(AuthError::LoginDenied);
            }

            let code = query_params
                .iter()
                .find(|(k, _)| k == "code")
                .map(|(_, v)| v.clone())
                .ok_or_else(|| AuthError::StateError("no code in callback".into()))?;

            let state = query_params
                .iter()
                .find(|(k, _)| k == "state")
                .map(|(_, v)| v.clone())
                .ok_or(AuthError::CsrfMismatch)?;

            if state != expected_state {
                return Err(AuthError::CsrfMismatch);
            }

            let body = "<html><body style='font-family:sans-serif;text-align:center;padding:40px'>\
                        <h1 style='color:#4CAF50'>Authorized!</h1>\
                        <p>Feche esta aba.</p></body></html>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.ok();

            Ok(AuthorizationCode { code, state })
        };

        tokio::time::timeout(self.timeout, accept_fut)
            .await
            .map_err(|_| AuthError::LoginTimeout)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_url() {
        let path = "/callback?code=abc123&state=xyz789";
        let params = parse_query(path);
        assert_eq!(params.len(), 2);
        assert_eq!(
            params.iter().find(|(k, _)| k == "code").map(|(_, v)| v.as_str()),
            Some("abc123")
        );
        assert_eq!(
            params.iter().find(|(k, _)| k == "state").map(|(_, v)| v.as_str()),
            Some("xyz789")
        );
    }

    #[test]
    fn test_parse_error_url() {
        let path = "/callback?error=access_denied&state=xyz";
        let params = parse_query(path);
        assert_eq!(
            params.iter().find(|(k, _)| k == "error").map(|(_, v)| v.as_str()),
            Some("access_denied")
        );
    }

    #[test]
    fn test_parse_url_decode_values() {
        let path = "/callback?code=a%20b%26c&state=%2Btest";
        let params = parse_query(path);
        assert_eq!(
            params.iter().find(|(k, _)| k == "code").map(|(_, v)| v.as_str()),
            Some("a b&c")
        );
        assert_eq!(
            params.iter().find(|(k, _)| k == "state").map(|(_, v)| v.as_str()),
            Some("+test")
        );
    }

    #[test]
    fn test_url_decode_plus() {
        assert_eq!(url_decode("a+b"), "a b");
    }

    #[test]
    fn test_url_decode_pct() {
        assert_eq!(url_decode("hello%20world"), "hello world");
    }

    #[test]
    fn test_url_decode_no_escape() {
        assert_eq!(url_decode("simple"), "simple");
    }

    #[test]
    fn test_server_timeout() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = CallbackServer::with_port(0).with_timeout(Duration::from_millis(1));
        let result = rt.block_on(server.wait_for_callback("test"));
        assert!(result.is_err());
        match result {
            Err(AuthError::PortUnavailable) | Err(AuthError::LoginTimeout) => (),
            _ => panic!("expected timeout or port unavailable error"),
        }
    }
}
