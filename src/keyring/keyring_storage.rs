use crate::keyring::error::{KeyringError, KeyringResult};
use std::collections::HashMap;

pub struct KeyringStorage;

impl KeyringStorage {
    pub async fn store_token(account_email: &str, token_json: &str) -> KeyringResult<()> {
        let ss = secret_service::SecretService::connect(
            secret_service::EncryptionType::Dh,
        )
        .await
        .map_err(|_| KeyringError::ServiceUnavailable)?;

        let collection = ss
            .get_default_collection()
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        collection
            .unlock()
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        let mut attrs = HashMap::new();
        attrs.insert("application", "libresync");
        attrs.insert("account", account_email);
        attrs.insert("type", "oauth");

        collection
            .create_item(
                &format!("libresync-{}", account_email),
                attrs,
                token_json.as_bytes(),
                true,
                "text/plain",
            )
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        Ok(())
    }

    pub async fn load_token(account_email: &str) -> KeyringResult<String> {
        let ss = secret_service::SecretService::connect(
            secret_service::EncryptionType::Dh,
        )
        .await
        .map_err(|_| KeyringError::ServiceUnavailable)?;

        let collection = ss
            .get_default_collection()
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        collection
            .unlock()
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        let mut attrs = HashMap::new();
        attrs.insert("application", "libresync");
        attrs.insert("account", account_email);
        attrs.insert("type", "oauth");

        let items = collection
            .search_items(attrs)
            .await
            .map_err(|_| KeyringError::TokenNotFound)?;

        let item = items
            .first()
            .ok_or(KeyringError::TokenNotFound)?;

        let secret = item
            .get_secret()
            .await
            .map_err(|_| KeyringError::TokenNotFound)?;

        String::from_utf8(secret).map_err(|_| KeyringError::InvalidFormat)
    }

    pub async fn delete_token(account_email: &str) -> KeyringResult<()> {
        let ss = secret_service::SecretService::connect(
            secret_service::EncryptionType::Dh,
        )
        .await
        .map_err(|_| KeyringError::ServiceUnavailable)?;

        let collection = ss
            .get_default_collection()
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        collection
            .unlock()
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        let mut attrs = HashMap::new();
        attrs.insert("application", "libresync");
        attrs.insert("account", account_email);
        attrs.insert("type", "oauth");

        let items = collection
            .search_items(attrs)
            .await
            .map_err(|_| KeyringError::TokenNotFound)?;

        let item = items
            .first()
            .ok_or(KeyringError::TokenNotFound)?;

        item.delete()
            .await
            .map_err(|_| KeyringError::ServiceUnavailable)?;

        Ok(())
    }
}
