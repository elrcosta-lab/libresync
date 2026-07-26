use std::fmt;

use crate::sync::error::SyncError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncState {
    Idle,
    Scanning,
    Queuing,
    Uploading,
    Downloading,
    Verifying,
    Retrying,
    Conflict,
    Resolving,
    Paused,
    Offline,
    Error,
}

impl fmt::Display for SyncState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncState::Idle => write!(f, "Idle"),
            SyncState::Scanning => write!(f, "Scanning"),
            SyncState::Queuing => write!(f, "Queuing"),
            SyncState::Uploading => write!(f, "Uploading"),
            SyncState::Downloading => write!(f, "Downloading"),
            SyncState::Verifying => write!(f, "Verifying"),
            SyncState::Retrying => write!(f, "Retrying"),
            SyncState::Conflict => write!(f, "Conflict"),
            SyncState::Resolving => write!(f, "Resolving"),
            SyncState::Paused => write!(f, "Paused"),
            SyncState::Offline => write!(f, "Offline"),
            SyncState::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncStateMachine {
    current: SyncState,
}

impl SyncStateMachine {
    pub fn new() -> Self {
        Self {
            current: SyncState::Idle,
        }
    }

    pub fn from(state: SyncState) -> Self {
        Self { current: state }
    }

    pub fn current(&self) -> SyncState {
        self.current
    }

    pub fn can_transition_to(&self, target: &SyncState) -> bool {
        use SyncState::*;
        matches!(
            (self.current, target),
            (Idle, Scanning | Paused | Offline)
                | (Scanning, Queuing | Error)
                |                 (Queuing, Uploading | Downloading | Conflict | Error)
                | (Uploading, Verifying | Retrying | Conflict | Error)
                | (Downloading, Verifying | Retrying | Conflict | Error)
                | (Verifying, Idle | Retrying | Error)
                | (Retrying, Queuing | Error)
                | (Conflict, Resolving | Error)
                | (Resolving, Idle | Queuing | Error)
                | (Paused, Idle)
                | (Offline, Idle)
                | (Error, Idle)
        )
    }

    pub fn transition(&mut self, target: SyncState) -> Result<(), SyncError> {
        if self.can_transition_to(&target) {
            self.current = target;
            Ok(())
        } else {
            Err(SyncError::InvalidTransition {
                from: self.current,
                to: target,
            })
        }
    }
}

impl Default for SyncStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
