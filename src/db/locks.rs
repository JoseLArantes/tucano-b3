use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

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
        let mutex = map
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        drop(map);

        let guard = mutex.try_lock_owned().map_err(|_| {
            AppError::LockConflict(format!("Ja existe sincronizacao em andamento para {key}."))
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

/// Adquire trava consultiva no nível de transação executada diretamente no pool.
///
/// ATENÇÃO (Nota de Arquitetura): Executar `pg_try_advisory_xact_lock` diretamente em um `&PgPool`
/// inicia e encerra uma transação implícita de um único comando, liberando a trava imediatamente.
/// Para retenção ao longo de um bloco crítico, use `try_acquire_pg_advisory_xact_lock_tx` com uma `Transaction` retida.
pub async fn try_acquire_pg_advisory_lock(pool: &PgPool, key: &str) -> Result<bool> {
    let lock_id = lock_key_to_i64(key);
    let row = sqlx::query("SELECT pg_try_advisory_xact_lock($1)")
        .bind(lock_id)
        .fetch_one(pool)
        .await?;
    let acquired: bool = row.get(0);
    Ok(acquired)
}

/// Adquire trava consultiva transacional retida durante o ciclo de vida da transação fornecida.
pub async fn try_acquire_pg_advisory_xact_lock_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
) -> Result<bool> {
    let lock_id = lock_key_to_i64(key);
    let row = sqlx::query("SELECT pg_try_advisory_xact_lock($1)")
        .bind(lock_id)
        .fetch_one(&mut **tx)
        .await?;
    let acquired: bool = row.get(0);
    Ok(acquired)
}

/// Adquire trava consultiva no nível de sessão na conexão de banco fornecida.
pub async fn try_acquire_session_advisory_lock(
    conn: &mut sqlx::PgConnection,
    key: &str,
) -> Result<bool> {
    let lock_id = lock_key_to_i64(key);
    let row = sqlx::query("SELECT pg_try_advisory_lock($1)")
        .bind(lock_id)
        .fetch_one(conn)
        .await?;
    let acquired: bool = row.get(0);
    Ok(acquired)
}

/// Libera trava consultiva de sessão previamente adquirida na conexão de banco fornecida.
pub async fn release_session_advisory_lock(
    conn: &mut sqlx::PgConnection,
    key: &str,
) -> Result<bool> {
    let lock_id = lock_key_to_i64(key);
    let row = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_id)
        .fetch_one(conn)
        .await?;
    let released: bool = row.get(0);
    Ok(released)
}
