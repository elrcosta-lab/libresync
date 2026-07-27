/// Monta a URL de autorização OAuth2 do Google.
/// Segue a especificação em RF-01 e RFC 7636.
pub fn build_authorization_url(
    client_id: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
) -> String {
    let params = [
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", "https://www.googleapis.com/auth/drive"),
        ("code_challenge", code_challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ];

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("https://accounts.google.com/o/oauth2/v2/auth?{}", query)
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".into(),
            other => format!("%{:02X}", other as u8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencode_ascii() {
        assert_eq!(urlencode("hello"), "hello");
    }

    #[test]
    fn test_urlencode_special_chars() {
        assert_eq!(urlencode("http://localhost:65432/callback"),
                   "http%3A%2F%2Flocalhost%3A65432%2Fcallback");
    }

    #[test]
    fn test_urlencode_space() {
        assert_eq!(urlencode("a b"), "a+b");
    }
}
