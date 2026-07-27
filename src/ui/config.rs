use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UIConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub sync_folder: String,
    pub bandwidth_limit: u64,
    pub auto_start: bool,
    pub polling_interval: u64,
    pub auto_sync_on_login: bool,
    pub notify_only_errors: bool,
    pub minimize_to_tray: bool,
    pub locale: String,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: None,
            sync_folder: String::new(),
            bandwidth_limit: 0,
            auto_start: false,
            polling_interval: 30,
            auto_sync_on_login: true,
            notify_only_errors: false,
            minimize_to_tray: true,
            locale: "pt-BR".into(),
        }
    }
}
