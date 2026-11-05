use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

use crate::error::{AppError, Result};

#[derive(Clone, Default)]
pub struct InMemoryLockRegistry {
    locks: Arc<std::sync::Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl InMemoryLockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn acquire(&self, key: &str) -> Result<InMemoryLockGuard> {
        let mut map = self.locks.lock().unwrap();
        let mutex = map.entry(key.to_string()).or_insert_with(|| Arc::new(Mutex::new(()))).clone();
        drop(map);

        let guard = mutex.try_lock_owned().map_err(|_| {
            AppError::LockConflict(format!(
                "Ja existe sincronizacao em andamento para {key}."
            ))
        })?;

        Ok(InMemoryLockGuard { _guard: guard })
    }
}

pub struct InMemoryLockGuard {
    _guard: OwnedMutexGuard<()>,
}

pub fn lock_key_to_i64(key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash[..8]);
    i64::from_be_bytes(bytes)
}

pub async fn try_acquire_pg_advisory_lock(pool: &PgPool, key: &str) -> Result<bool> {
    let lock_id = lock_key_to_i64(key);
    let row = sqlx::query("SELECT pg_try_advisory_xact_lock($1)")
        .bind(lock_id)
        .fetch_one(pool)
        .await?;
    let acquired: bool = row.get(0);
    Ok(acquired)
}
