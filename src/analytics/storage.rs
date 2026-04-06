use std::{collections::HashMap, sync::Mutex};

use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::{RngCore, rngs::OsRng};
use rocket::serde::{Deserialize, Serialize};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

const RETENTION_DAYS: u32 = 90;
pub(super) const SECONDS_PER_DAY: i64 = time::Duration::DAY.whole_seconds();

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub(super) struct PageView {
    pub timestamp: i64,
    pub path: String,
    pub method: String,
    pub status_code: u16,
    pub response_time_ms: i64,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub visitor_hash: Option<String>,
}

struct CurrentKey {
    key_id: i64,
    aes_key: [u8; 32],
    created_date: String,
}

pub struct Analytics {
    connection: Mutex<Connection>,
    public_key: RsaPublicKey,
    current_key: Mutex<Option<CurrentKey>>,
    visitor_salt: [u8; 32],
}

impl Analytics {
    pub fn open(db_path: &str, public_key: RsaPublicKey) -> rusqlite::Result<Self> {
        let connection = Connection::open(db_path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                visitor_salt TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS encryption_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                encrypted_key TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS page_views (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                key_id INTEGER NOT NULL,
                encrypted_data TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_page_views_created_at ON page_views(created_at);",
        )?;

        let visitor_salt = connection
            .query_row("SELECT visitor_salt FROM config WHERE id = 1", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;

        let visitor_salt = match visitor_salt {
            Some(encoded) => {
                let decoded = BASE64
                    .decode(&encoded)
                    .expect("Invalid visitor_salt in config table");
                let mut salt = [0u8; 32];
                salt.copy_from_slice(&decoded);
                salt
            }
            None => {
                let mut salt = [0u8; 32];
                OsRng.fill_bytes(&mut salt);
                connection.execute(
                    "INSERT INTO config (id, visitor_salt) VALUES (1, ?1)",
                    [BASE64.encode(&salt)],
                )?;
                salt
            }
        };

        Ok(Self {
            connection: Mutex::new(connection),
            public_key,
            current_key: Mutex::new(None),
            visitor_salt,
        })
    }

    pub fn cleanup_old_records(&self) {
        let retention = format!("-{RETENTION_DAYS} days");
        let connection = self.connection.lock().unwrap();
        let _ = connection.execute(
            "DELETE FROM page_views WHERE created_at < strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?1)",
            [&retention],
        );
        let _ = connection.execute(
            "DELETE FROM encryption_keys WHERE date < date('now', ?1)",
            [&retention],
        );
    }

    pub(super) fn record_page_view(&self, view: &PageView) {
        let Ok(json_data) = serde_json::to_vec(view) else {
            return;
        };
        let (aes_key, key_id) = self.get_or_create_daily_key();

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
        let Ok(ciphertext) = cipher.encrypt(Nonce::from_slice(&nonce_bytes), json_data.as_ref())
        else {
            return;
        };

        let encoded = BASE64.encode([nonce_bytes.as_slice(), &ciphertext].concat());

        let connection = self.connection.lock().unwrap();
        let _ = connection.execute(
            "INSERT INTO page_views (key_id, encrypted_data) VALUES (?1, ?2)",
            rusqlite::params![key_id, encoded],
        );
    }

    fn get_or_create_daily_key(&self) -> ([u8; 32], i64) {
        let today = epoch_to_date(OffsetDateTime::now_utc().unix_timestamp());
        let mut current = self.current_key.lock().unwrap();

        if let Some(cached) = current
            .as_ref()
            .filter(|cached| cached.created_date == today)
        {
            return (cached.aes_key, cached.key_id);
        }

        let mut aes_key = [0u8; 32];
        OsRng.fill_bytes(&mut aes_key);

        let encrypted = self
            .public_key
            .encrypt(&mut OsRng, Oaep::new::<Sha256>(), &aes_key)
            .unwrap();
        let encoded = BASE64.encode(&encrypted);

        let connection = self.connection.lock().unwrap();
        connection
            .execute(
                "INSERT INTO encryption_keys (date, encrypted_key) VALUES (?1, ?2)",
                rusqlite::params![today, encoded],
            )
            .unwrap();
        let key_id = connection.last_insert_rowid();

        *current = Some(CurrentKey {
            key_id,
            aes_key,
            created_date: today,
        });

        (aes_key, key_id)
    }

    pub(super) fn decrypt_all(&self, private_key: &RsaPrivateKey) -> Result<Vec<PageView>, String> {
        let connection = self.connection.lock().unwrap();

        let mut key_statement = connection
            .prepare("SELECT id, encrypted_key FROM encryption_keys")
            .unwrap();
        let encrypted_keys: Vec<(i64, String)> = key_statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|result| result.ok())
            .collect();

        let mut decrypted_keys: HashMap<i64, [u8; 32]> = HashMap::new();
        for (key_id, encoded_key) in &encrypted_keys {
            let encrypted = BASE64.decode(encoded_key).map_err(|err| err.to_string())?;
            if let Ok(aes_key_vec) = private_key.decrypt(Oaep::new::<Sha256>(), &encrypted) {
                if aes_key_vec.len() == 32 {
                    let mut key_array = [0u8; 32];
                    key_array.copy_from_slice(&aes_key_vec);
                    decrypted_keys.insert(*key_id, key_array);
                }
            }
        }

        if decrypted_keys.is_empty() && !encrypted_keys.is_empty() {
            return Err("Decryption failed. Wrong private key?".to_owned());
        }

        let mut row_statement = connection
            .prepare("SELECT key_id, encrypted_data FROM page_views ORDER BY id")
            .unwrap();
        let encrypted_rows: Vec<(i64, String)> = row_statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|result| result.ok())
            .collect();

        let mut views = Vec::with_capacity(encrypted_rows.len());
        for (key_id, encoded_data) in &encrypted_rows {
            let Some(aes_key) = decrypted_keys.get(key_id) else {
                continue;
            };
            let Ok(encrypted) = BASE64.decode(encoded_data) else {
                continue;
            };
            if encrypted.len() < 12 {
                continue;
            }
            let (nonce, ciphertext) = encrypted.split_at(12);
            let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(aes_key));
            let Ok(plaintext) = cipher.decrypt(Nonce::from_slice(nonce), ciphertext) else {
                continue;
            };
            if let Ok(view) = serde_json::from_slice::<PageView>(&plaintext) {
                views.push(view);
            }
        }

        Ok(views)
    }

    pub(super) fn hash_visitor(&self, client_ip: &str) -> String {
        let day_number = OffsetDateTime::now_utc().unix_timestamp() / SECONDS_PER_DAY;
        let mut hasher = Sha256::new();
        hasher.update(&self.visitor_salt);
        hasher.update(client_ip.as_bytes());
        hasher.update(day_number.to_le_bytes());
        format!("{:x}", hasher.finalize())
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
