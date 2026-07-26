use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum NotificationCategory {
    SyncCompleted,
    Conflict,
    AuthError,
    ConnectionLost,
    ConnectionRestored,
    Error,
    Warning,
    Info,
}

struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u64, refill_interval_secs: u64) -> Self {
        Self {
            capacity: capacity as f64,
            tokens: capacity as f64,
            refill_rate: 1.0 / refill_interval_secs as f64,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationType {
    SyncCompleted { file_count: u32 },
    Conflict { file_name: String },
    AuthError { message: String },
    ConnectionLost,
    ConnectionRestored,
    Error { message: String },
    Warning { message: String },
    Info { message: String },
}

impl NotificationType {
    fn category(&self) -> NotificationCategory {
        match self {
            NotificationType::SyncCompleted { .. } => NotificationCategory::SyncCompleted,
            NotificationType::Conflict { .. } => NotificationCategory::Conflict,
            NotificationType::AuthError { .. } => NotificationCategory::AuthError,
            NotificationType::ConnectionLost => NotificationCategory::ConnectionLost,
            NotificationType::ConnectionRestored => NotificationCategory::ConnectionRestored,
            NotificationType::Error { .. } => NotificationCategory::Error,
            NotificationType::Warning { .. } => NotificationCategory::Warning,
            NotificationType::Info { .. } => NotificationCategory::Info,
        }
    }
}

pub struct NotificationManager {
    rate_limiters: HashMap<NotificationCategory, TokenBucket>,
    refill_interval_secs: u64,
    suppress_when_focused: bool,
    is_focused: bool,
}

impl NotificationManager {
    pub fn new(refill_interval_secs: u64, suppress_when_focused: bool) -> Self {
        Self {
            rate_limiters: HashMap::new(),
            refill_interval_secs,
            suppress_when_focused,
            is_focused: false,
        }
    }

    pub fn send(&mut self, notif_type: &NotificationType) -> bool {
        if self.suppress_when_focused && self.is_focused {
            return false;
        }
        let category = notif_type.category();
        let bucket = self
            .rate_limiters
            .entry(category)
            .or_insert_with(|| TokenBucket::new(1, self.refill_interval_secs));
        bucket.try_consume(1.0)
    }

    pub fn set_suppress_when_focused(&mut self, value: bool) {
        self.suppress_when_focused = value;
    }

    pub fn set_focused(&mut self, value: bool) {
        self.is_focused = value;
    }
}
