use crate::ui::notifications::NotificationType;

#[derive(Default)]
pub struct NotificationSender;

impl NotificationSender {
    pub fn new() -> Self {
        Self
    }

    pub fn send(&self, event_type: &NotificationType) -> String {
        let message = match event_type {
            NotificationType::SyncCompleted { file_count } => {
                format!("Sincronização concluída: {} arquivos", file_count)
            }
            NotificationType::Conflict { file_name } => {
                format!("Conflito detectado em {}", file_name)
            }
            NotificationType::AuthError { message } => {
                format!("Erro de autenticação: {}", message)
            }
            NotificationType::ConnectionLost => {
                "Conexão perdida. Tentando reconectar...".to_string()
            }
            NotificationType::ConnectionRestored => "Conexão restaurada".to_string(),
            NotificationType::Error { message } => {
                format!("Erro: {}", message)
            }
            NotificationType::Warning { message } => {
                format!("Aviso: {}", message)
            }
            NotificationType::Info { message } => message.clone(),
        };
        let _ = notify_rust::Notification::new()
            .appname("LibreSync")
            .summary("LibreSync")
            .body(&message)
            .icon("dialog-information")
            .show();
        message
    }
}
