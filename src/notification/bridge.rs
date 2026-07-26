use crate::notification::sender::NotificationSender;
use crate::ui::notifications::{NotificationManager, NotificationType};

pub struct DesktopNotifier {
    manager: NotificationManager,
    sender: NotificationSender,
}

impl DesktopNotifier {
    pub fn new(refill_interval_secs: u64, suppress_when_focused: bool) -> Self {
        Self {
            manager: NotificationManager::new(refill_interval_secs, suppress_when_focused),
            sender: NotificationSender::new(),
        }
    }

    pub fn notify(&mut self, event_type: &NotificationType) -> Option<String> {
        if self.manager.send(event_type) {
            Some(self.sender.send(event_type))
        } else {
            None
        }
    }

    pub fn set_focused(&mut self, value: bool) {
        self.manager.set_focused(value);
    }

    pub fn set_suppress_when_focused(&mut self, value: bool) {
        self.manager.set_suppress_when_focused(value);
    }
}
