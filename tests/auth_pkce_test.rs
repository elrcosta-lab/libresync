use libresync_core::auth::pkce;

#[test]
fn test_generate_code_verifier_length() {
    let verifier = pkce::generate_code_verifier();
    assert_eq!(verifier.len(), 171, "code_verifier deve ter 171 caracteres base64url (128 bytes)");
}

#[test]
fn test_generate_code_verifier_is_base64url() {
    let verifier = pkce::generate_code_verifier();
    for c in verifier.chars() {
        assert!(
            c.is_ascii_alphanumeric() || c == '-' || c == '_',
            "caractere '{}' não é base64url válido", c
        );
    }
}

#[test]
fn test_generate_code_verifier_no_padding() {
    let verifier = pkce::generate_code_verifier();
    assert!(!verifier.contains('='), "code_verifier não deve ter padding");
}

#[test]
fn test_generate_code_verifier_unique() {
    let a = pkce::generate_code_verifier();
    let b = pkce::generate_code_verifier();
    assert_ne!(a, b, "duas chamadas devem gerar valores diferentes");
}

#[test]
fn test_compute_code_challenge_correctness() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = pkce::compute_code_challenge(verifier);
    assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM", "code_challenge deve calcular SHA256 correto");
}
