use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tracking states for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackingState {
    /// Not tracking - vendor not working on company tasks
    Stopped,
    /// Actively tracking - vendor is working
    Tracking,
    /// Paused - vendor on break
    Paused,
}

impl TrackingState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackingState::Stopped => "stopped",
            TrackingState::Tracking => "tracking",
            TrackingState::Paused => "paused",
        }
    }

    pub fn is_tracking(&self) -> bool {
        matches!(self, TrackingState::Tracking)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, TrackingState::Paused)
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self, TrackingState::Stopped)
    }
}

/// Represents a tracking session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: TrackingState,
}

impl Session {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            start_time: Utc::now(),
            end_time: None,
            state: TrackingState::Tracking,
        }
    }

    pub fn duration_secs(&self) -> u64 {
        let end = self.end_time.unwrap_or_else(Utc::now);
        (end - self.start_time).num_seconds().max(0) as u64
    }

    pub fn is_active(&self) -> bool {
        self.end_time.is_none()
    }
}

/// Represents a break period during a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakPeriod {
    pub id: i64,
    pub session_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
}

impl BreakPeriod {
    pub fn new(id: i64, session_id: i64) -> Self {
        Self {
            id,
            session_id,
            start_time: Utc::now(),
            end_time: None,
        }
    }

    pub fn duration_secs(&self) -> u64 {
        let end = self.end_time.unwrap_or_else(Utc::now);
        (end - self.start_time).num_seconds().max(0) as u64
    }

    pub fn is_active(&self) -> bool {
        self.end_time.is_none()
    }
}

/// State manager for tracking state transitions
pub struct StateManager {
    current_state: TrackingState,
    current_session: Option<Session>,
    current_break: Option<BreakPeriod>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            current_state: TrackingState::Stopped,
            current_session: None,
            current_break: None,
        }
    }

    pub fn current_state(&self) -> TrackingState {
        self.current_state
    }

    pub fn current_session(&self) -> Option<&Session> {
        self.current_session.as_ref()
    }

    pub fn current_break(&self) -> Option<&BreakPeriod> {
        self.current_break.as_ref()
    }

    /// Start tracking
    pub fn start_tracking(&mut self, session_id: i64) -> Result<(), String> {
        match self.current_state {
            TrackingState::Stopped => {
                self.current_state = TrackingState::Tracking;
                self.current_session = Some(Session::new(session_id));
                Ok(())
            }
            TrackingState::Tracking => Err("Already tracking".to_string()),
            TrackingState::Paused => {
                // Resume from pause
                self.current_state = TrackingState::Tracking;
                if let Some(ref mut brk) = self.current_break {
                    brk.end_time = Some(Utc::now());
                }
                self.current_break = None;
                Ok(())
            }
        }
    }

    /// Pause tracking (break)
    pub fn pause_tracking(&mut self, break_id: i64) -> Result<(), String> {
        match self.current_state {
            TrackingState::Tracking => {
                if let Some(session) = &self.current_session {
                    self.current_state = TrackingState::Paused;
                    self.current_break = Some(BreakPeriod::new(break_id, session.id));
                    Ok(())
                } else {
                    Err("No active session".to_string())
                }
            }
            TrackingState::Paused => Err("Already paused".to_string()),
            TrackingState::Stopped => Err("Not tracking".to_string()),
        }
    }

    /// Resume tracking from pause
    pub fn resume_tracking(&mut self) -> Result<(), String> {
        match self.current_state {
            TrackingState::Paused => {
                self.current_state = TrackingState::Tracking;
                if let Some(ref mut brk) = self.current_break {
                    brk.end_time = Some(Utc::now());
                }
                self.current_break = None;
                Ok(())
            }
            TrackingState::Tracking => Err("Already tracking".to_string()),
            TrackingState::Stopped => Err("Not in a session".to_string()),
        }
    }

    /// Stop tracking
    pub fn stop_tracking(&mut self) -> Result<(), String> {
        match self.current_state {
            TrackingState::Tracking | TrackingState::Paused => {
                self.current_state = TrackingState::Stopped;
                if let Some(ref mut session) = self.current_session {
                    session.end_time = Some(Utc::now());
                }
                if let Some(ref mut brk) = self.current_break {
                    brk.end_time = Some(Utc::now());
                }
                Ok(())
            }
            TrackingState::Stopped => Err("Not tracking".to_string()),
        }
    }

    /// Clear session after it's been processed
    pub fn clear_session(&mut self) {
        self.current_session = None;
        self.current_break = None;
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        let mut manager = StateManager::new();

        assert_eq!(manager.current_state(), TrackingState::Stopped);

        // Start tracking
        assert!(manager.start_tracking(1).is_ok());
        assert_eq!(manager.current_state(), TrackingState::Tracking);
        assert!(manager.current_session().is_some());

        // Pause
        assert!(manager.pause_tracking(1).is_ok());
        assert_eq!(manager.current_state(), TrackingState::Paused);
        assert!(manager.current_break().is_some());

        // Resume
        assert!(manager.resume_tracking().is_ok());
        assert_eq!(manager.current_state(), TrackingState::Tracking);
        assert!(manager.current_break().is_none() || !manager.current_break().unwrap().is_active());

        // Stop
        assert!(manager.stop_tracking().is_ok());
        assert_eq!(manager.current_state(), TrackingState::Stopped);
    }
}
