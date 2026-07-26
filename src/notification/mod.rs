pub mod bridge;
pub mod sender;

pub use bridge::DesktopNotifier;
pub use sender::NotificationSender;
pub use crate::ui::notifications::NotificationType;
