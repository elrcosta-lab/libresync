use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Gera um code_verifier de 128 bytes aleatórios, codificado em base64url sem padding.
/// RFC 7636 seção 4.1 recomenda entropia mínima de 128 bits.
pub fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 128];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Calcula o code_challenge = BASE64URL-ENCODE(SHA256(verifier)) sem padding.
/// RFC 7636 seção 4.2 — code_challenge_method=S256.
pub fn compute_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}
