use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use subtle::ConstantTimeEq;
use time::OffsetDateTime;

const RETENTION_DAYS: u32 = 90;
pub(super) const SECONDS_PER_DAY: i64 = time::Duration::DAY.whole_seconds();

pub(super) struct PageView {
    pub timestamp: i64,
    pub path: String,
    pub method: String,
    pub status_code: u16,
    pub response_time_ms: i64,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub visitor_ip: Option<String>,
}

#[derive(Clone)]
pub struct Analytics {
    connection: Arc<Mutex<Connection>>,
    password: Arc<str>,
}

impl Analytics {
    pub fn open(db_path: &str, password: &str) -> rusqlite::Result<Self> {
        let connection = Connection::open(db_path)?;
        connection.pragma_update(None, "key", password)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS page_views (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                timestamp INTEGER NOT NULL,
                path TEXT NOT NULL,
                method TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                response_time_ms INTEGER NOT NULL,
                user_agent TEXT,
                referrer TEXT,
                visitor_ip TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_page_views_created_at ON page_views(created_at);",
        )?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            password: password.into(),
        })
    }

    pub fn cleanup_old_records(&self) {
        let retention = format!("-{RETENTION_DAYS} days");
        let connection = self.connection.lock().unwrap();
        if let Err(error) = connection.execute(
            "DELETE FROM page_views WHERE created_at < strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?1)",
            [&retention],
        ) {
            eprintln!("Failed to clean up old page views: {error}");
        }
    }

    pub(super) fn check_password(&self, password: &str) -> bool {
        self.password.as_bytes().ct_eq(password.as_bytes()).into()
    }

    pub(super) fn record_page_view(&self, view: &PageView) {
        let connection = self.connection.lock().unwrap();
        if let Err(error) = connection.execute(
            "INSERT INTO page_views (timestamp, path, method, status_code, response_time_ms, user_agent, referrer, visitor_ip)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                view.timestamp,
                view.path,
                view.method,
                view.status_code,
                view.response_time_ms,
                view.user_agent,
                view.referrer,
                view.visitor_ip,
            ],
        ) {
            eprintln!("Failed to insert page view: {error}");
        }
    }

    pub(super) fn query_all(&self) -> Result<Vec<PageView>, String> {
        let connection = self.connection.lock().unwrap();
        let mut statement = connection
            .prepare(
                "SELECT timestamp, path, method, status_code, response_time_ms, user_agent, referrer, visitor_ip
                 FROM page_views ORDER BY id",
            )
            .map_err(|error| error.to_string())?;

        let views = statement
            .query_map([], |row| {
                Ok(PageView {
                    timestamp: row.get(0)?,
                    path: row.get(1)?,
                    method: row.get(2)?,
                    status_code: row.get(3)?,
                    response_time_ms: row.get(4)?,
                    user_agent: row.get(5)?,
                    referrer: row.get(6)?,
                    visitor_ip: row.get(7)?,
                })
            })
            .map_err(|error| error.to_string())?
            .filter_map(|result| result.ok())
            .collect();

        Ok(views)
    }
}

pub(super) fn epoch_to_date(epoch_secs: i64) -> String {
    let date = OffsetDateTime::from_unix_timestamp(epoch_secs)
        .unwrap()
        .date();
    format!("{date}")
}

pub(super) fn epoch_to_datetime(epoch_secs: i64) -> String {
    let datetime = OffsetDateTime::from_unix_timestamp(epoch_secs).unwrap();
    format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        datetime.year(),
        datetime.month() as u8,
        datetime.day(),
        datetime.hour(),
        datetime.minute(),
        datetime.second(),
    )
}
