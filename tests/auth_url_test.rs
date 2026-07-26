use libresync_core::auth::url::build_authorization_url;

#[test]
fn test_build_authorization_url_basic() {
    let url = build_authorization_url(
        "my-client-id",
        "http://localhost:65432/callback",
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        "some-state-value",
    );
    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"), "URL deve começar com o endpoint Google");
    assert!(url.contains("client_id=my-client-id"), "deve conter client_id");
    assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A65432%2Fcallback"), "redirect_uri deve estar url-encoded");
    assert!(url.contains("response_type=code"), "deve conter response_type=code");
    assert!(url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.file"), "deve conter scope url-encoded");
    assert!(url.contains("code_challenge=dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "deve conter code_challenge");
    assert!(url.contains("code_challenge_method=S256"), "deve conter S256");
    assert!(url.contains("state=some-state-value"), "deve conter state");
    assert!(url.contains("access_type=offline"), "deve solicitar refresh token");
    assert!(url.contains("prompt=consent"), "deve forçar consentimento");
}
