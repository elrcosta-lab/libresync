use libresync_core::auth::models::{Account, AccountStatus, TokenSet};

#[test]
fn test_account_creation() {
    let account = Account::new(
        "12345".into(),
        "maria@gmail.com".into(),
        "Maria Silva".into(),
    );
    assert_eq!(account.email, "maria@gmail.com");
    assert_eq!(account.display_name, "Maria Silva");
    assert_eq!(account.status, AccountStatus::Active);
    assert!(account.is_active);
    assert!(account.created_at > 0);
}

#[test]
fn test_account_default_status_is_active() {
    let account = Account::new("1".into(), "test@test.com".into(), "Test".into());
    assert_eq!(account.status, AccountStatus::Active);
}

#[test]
fn test_account_can_be_revoked() {
    let mut account = Account::new("1".into(), "test@test.com".into(), "Test".into());
    account.revoke();
    assert_eq!(account.status, AccountStatus::Revoked);
    assert!(!account.is_active);
}

#[test]
fn test_token_set_expiry_check() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut tokens = TokenSet {
        access_token: "ya29.token".into(),
        refresh_token: Some("1//token".into()),
        id_token: None,
        expires_at: now + 3600,
        token_type: "Bearer".into(),
        scope: "drive.file".into(),
    };

    assert!(!tokens.is_expired()); // Still valid
    assert!(!tokens.should_refresh(300)); // Margin of 300s, 3600 > 300

    tokens.expires_at = now - 60; // Expired 1 min ago
    assert!(tokens.is_expired());
}

#[test]
fn test_token_set_should_refresh() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let tokens = TokenSet {
        access_token: "ya29.token".into(),
        refresh_token: Some("1//token".into()),
        id_token: None,
        expires_at: now + 120, // Expires in 2 min
        token_type: "Bearer".into(),
        scope: "drive.file".into(),
    };

    assert!(tokens.should_refresh(300)); // 120 < 300, should refresh
}

#[test]
fn test_token_set_serialization() {
    let tokens = TokenSet {
        access_token: "ya29.secret".into(),
        refresh_token: Some("1//refresh".into()),
        id_token: Some("eyJ.eyJ9.sig".into()),
        expires_at: 1700000000,
        token_type: "Bearer".into(),
        scope: "drive.file".into(),
    };

    let json = serde_json::to_string(&tokens).unwrap();
    let deserialized: TokenSet = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.access_token, "ya29.secret");
    assert_eq!(deserialized.refresh_token, Some("1//refresh".into()));
    assert_eq!(deserialized.token_type, "Bearer");
}
