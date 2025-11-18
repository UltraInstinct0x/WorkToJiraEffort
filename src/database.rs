use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

use crate::screenpipe::Activity;
use crate::state::{Session, TrackingState};

/// Activity tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityTier {
    Micro,    // < 10 minutes
    Billable, // >= 10 minutes
}

impl ActivityTier {
    pub fn from_duration(duration_secs: u64) -> Self {
        if duration_secs < 600 {
            ActivityTier::Micro
        } else {
            ActivityTier::Billable
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityTier::Micro => "micro",
            ActivityTier::Billable => "billable",
        }
    }
}

/// Stored activity with additional metadata
#[derive(Debug, Clone)]
pub struct StoredActivity {
    pub id: i64,
    pub session_id: i64,
    pub timestamp: DateTime<Utc>,
    pub duration_secs: u64,
    pub window_title: String,
    pub app_name: String,
    pub description: String,
    pub tier: ActivityTier,
    pub logged_to_jira: bool,
}

impl From<&Activity> for StoredActivity {
    fn from(activity: &Activity) -> Self {
        Self {
            id: 0, // Will be set by database
            session_id: 0,
            timestamp: activity.timestamp,
            duration_secs: activity.duration_secs,
            window_title: activity.window_title.clone(),
            app_name: activity.app_name.clone(),
            description: activity.description.clone(),
            tier: ActivityTier::from_duration(activity.duration_secs),
            logged_to_jira: false,
        }
    }
}

/// LLM analysis result storage
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub id: i64,
    pub session_id: i64,
    pub analyzed_at: DateTime<Utc>,
    pub llm_response: String, // JSON response from LLM
    pub confidence: f64,
}

/// Local database for activity storage and analytics
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new database connection
    pub fn new(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let conn = Connection::open(db_path).context("Failed to open database")?;

        let db = Self { conn };
        db.init_schema()?;

        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                start_time TEXT NOT NULL,
                end_time TEXT,
                state TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS breaks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS activities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                duration_secs INTEGER NOT NULL,
                window_title TEXT NOT NULL,
                app_name TEXT NOT NULL,
                description TEXT NOT NULL,
                tier TEXT NOT NULL,
                logged_to_jira INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS analysis_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                analyzed_at TEXT NOT NULL,
                llm_response TEXT NOT NULL,
                confidence REAL NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_activities_session ON activities(session_id);
            CREATE INDEX IF NOT EXISTS idx_activities_timestamp ON activities(timestamp);
            CREATE INDEX IF NOT EXISTS idx_activities_tier ON activities(tier);
            CREATE INDEX IF NOT EXISTS idx_breaks_session ON breaks(session_id);
            "#,
        )?;

        Ok(())
    }

    /// Create a new session
    pub fn create_session(&self) -> Result<i64> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO sessions (start_time, state) VALUES (?1, ?2)",
            params![now.to_rfc3339(), TrackingState::Tracking.as_str()],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// End a session
    pub fn end_session(&self, session_id: i64) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            "UPDATE sessions SET end_time = ?1, state = ?2 WHERE id = ?3",
            params![now.to_rfc3339(), TrackingState::Stopped.as_str(), session_id],
        )?;

        Ok(())
    }

    /// Get active session
    pub fn get_active_session(&self) -> Result<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, start_time, end_time, state FROM sessions WHERE end_time IS NULL ORDER BY id DESC LIMIT 1",
        )?;

        let session = stmt
            .query_row([], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    start_time: row.get::<_, String>(1)?.parse().unwrap(),
                    end_time: row.get::<_, Option<String>>(2)?.and_then(|s| s.parse().ok()),
                    state: match row.get::<_, String>(3)?.as_str() {
                        "tracking" => TrackingState::Tracking,
                        "paused" => TrackingState::Paused,
                        _ => TrackingState::Stopped,
                    },
                })
            })
            .optional()?;

        Ok(session)
    }

    /// Create a break period
    pub fn create_break(&self, session_id: i64) -> Result<i64> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO breaks (session_id, start_time) VALUES (?1, ?2)",
            params![session_id, now.to_rfc3339()],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// End a break period
    pub fn end_break(&self, break_id: i64) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            "UPDATE breaks SET end_time = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), break_id],
        )?;

        Ok(())
    }

    /// Store an activity
    pub fn store_activity(&self, session_id: i64, activity: &Activity) -> Result<i64> {
        let tier = ActivityTier::from_duration(activity.duration_secs);

        self.conn.execute(
            "INSERT INTO activities (session_id, timestamp, duration_secs, window_title, app_name, description, tier)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session_id,
                activity.timestamp.to_rfc3339(),
                activity.duration_secs as i64,
                activity.window_title,
                activity.app_name,
                activity.description,
                tier.as_str(),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get activities for a session
    pub fn get_session_activities(&self, session_id: i64, tier: Option<ActivityTier>) -> Result<Vec<StoredActivity>> {
        let query = if let Some(t) = tier {
            format!(
                "SELECT id, session_id, timestamp, duration_secs, window_title, app_name, description, tier, logged_to_jira
                 FROM activities WHERE session_id = ?1 AND tier = '{}' ORDER BY timestamp",
                t.as_str()
            )
        } else {
            "SELECT id, session_id, timestamp, duration_secs, window_title, app_name, description, tier, logged_to_jira
             FROM activities WHERE session_id = ?1 ORDER BY timestamp".to_string()
        };

        let mut stmt = self.conn.prepare(&query)?;
        let activities = stmt
            .query_map([session_id], |row| {
                Ok(StoredActivity {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    timestamp: row.get::<_, String>(2)?.parse().unwrap(),
                    duration_secs: row.get::<_, i64>(3)? as u64,
                    window_title: row.get(4)?,
                    app_name: row.get(5)?,
                    description: row.get(6)?,
                    tier: match row.get::<_, String>(7)?.as_str() {
                        "micro" => ActivityTier::Micro,
                        _ => ActivityTier::Billable,
                    },
                    logged_to_jira: row.get::<_, i64>(8)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(activities)
    }

    /// Mark activities as logged to Jira
    pub fn mark_activities_logged(&self, activity_ids: &[i64]) -> Result<()> {
        let placeholders = activity_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("UPDATE activities SET logged_to_jira = 1 WHERE id IN ({})", placeholders);

        let params: Vec<&dyn rusqlite::ToSql> = activity_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        self.conn.execute(&query, &params[..])?;

        Ok(())
    }

    /// Store LLM analysis result
    pub fn store_analysis(&self, session_id: i64, llm_response: String, confidence: f64) -> Result<i64> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO analysis_results (session_id, analyzed_at, llm_response, confidence) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, now.to_rfc3339(), llm_response, confidence],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get total break time for a session
    pub fn get_session_break_time(&self, session_id: i64) -> Result<u64> {
        let total: Option<i64> = self.conn.query_row(
            "SELECT SUM(
                CASE
                    WHEN end_time IS NOT NULL
                    THEN (julianday(end_time) - julianday(start_time)) * 86400
                    ELSE (julianday('now') - julianday(start_time)) * 86400
                END
            ) FROM breaks WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?;

        Ok(total.unwrap_or(0).max(0) as u64)
    }

    /// Get session statistics
    pub fn get_session_stats(&self, session_id: i64) -> Result<SessionStats> {
        let session = self.conn.query_row(
            "SELECT start_time, end_time FROM sessions WHERE id = ?1",
            [session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?.parse::<DateTime<Utc>>().unwrap(),
                    row.get::<_, Option<String>>(1)?.and_then(|s| s.parse().ok()),
                ))
            },
        )?;

        let total_duration = {
            let end = session.1.unwrap_or_else(Utc::now);
            (end - session.0).num_seconds().max(0) as u64
        };

        let break_time = self.get_session_break_time(session_id)?;

        let activities = self.get_session_activities(session_id, None)?;
        let billable_activities = self.get_session_activities(session_id, Some(ActivityTier::Billable))?;
        let micro_activities = self.get_session_activities(session_id, Some(ActivityTier::Micro))?;

        let billable_time: u64 = billable_activities.iter().map(|a| a.duration_secs).sum();
        let micro_time: u64 = micro_activities.iter().map(|a| a.duration_secs).sum();

        Ok(SessionStats {
            session_id,
            start_time: session.0,
            end_time: session.1,
            total_duration_secs: total_duration,
            break_duration_secs: break_time,
            billable_time_secs: billable_time,
            micro_time_secs: micro_time,
            total_activities: activities.len(),
            billable_activities: billable_activities.len(),
            micro_activities: micro_activities.len(),
        })
    }
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_duration_secs: u64,
    pub break_duration_secs: u64,
    pub billable_time_secs: u64,
    pub micro_time_secs: u64,
    pub total_activities: usize,
    pub billable_activities: usize,
    pub micro_activities: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_database_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::new(temp_file.path().to_path_buf()).unwrap();

        let session_id = db.create_session().unwrap();
        assert!(session_id > 0);
    }

    #[test]
    fn test_activity_storage() {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::new(temp_file.path().to_path_buf()).unwrap();

        let session_id = db.create_session().unwrap();

        let activity = Activity {
            timestamp: Utc::now(),
            duration_secs: 300,
            window_title: "Test".to_string(),
            app_name: "Test App".to_string(),
            description: "Test description".to_string(),
        };

        let activity_id = db.store_activity(session_id, &activity).unwrap();
        assert!(activity_id > 0);

        let activities = db.get_session_activities(session_id, None).unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].tier, ActivityTier::Micro);
    }
}
