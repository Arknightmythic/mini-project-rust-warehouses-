use std::time::Duration;

use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::Serialize;
use serde::de::DeserializeOwned;

// THE RULE FOR THIS WHOLE MODULE: a cache is an optimisation, never a dependency.
// Every method below swallows Redis errors and reports a miss, so a dead Redis
// makes the service slower and never makes it fail. Any code path that can return
// an error to the caller because the cache misbehaved is a bug.
// Hard ceiling on every cache operation. Redis normally answers in well under a
// millisecond, so anything approaching this is already useless: Postgres will
// answer faster than we are willing to keep waiting.
//
// This bound is enforced here rather than left to the client library. Measured
// the hard way: with Redis stopped and only the library defaults in play, a
// single request took 16-30 SECONDS to fall through to the database. The error
// handling was correct the whole time; the latency was not. A cache that is
// merely "not required for correctness" is still an outage if it can stall you.
const OP_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct Cache {
    conn: ConnectionManager,
    ttl_secs: u64,
}

impl Cache {
    pub async fn connect(url: &str, ttl_secs: u64) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;

        let conn = tokio::time::timeout(Duration::from_secs(3), ConnectionManager::new(client))
            .await
            .map_err(|_| anyhow::anyhow!("timed out connecting to redis at {url}"))??;

        Ok(Self { conn, ttl_secs })
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = self.conn.clone();

        let raw: Option<String> = match tokio::time::timeout(OP_TIMEOUT, conn.get(key)).await {
            Ok(Ok(value)) => value,
            Ok(Err(err)) => {
                tracing::warn!(key, error = %err, "cache read failed, falling through");
                return None;
            }
            Err(_) => {
                tracing::warn!(key, "cache read timed out, falling through");
                return None;
            }
        };

        match raw {
            Some(json) => match serde_json::from_str(&json) {
                Ok(value) => {
                    tracing::debug!(key, "cache hit");
                    Some(value)
                }
                // A shape change in the cached type is not an error worth failing
                // on: treat it as a miss and let the write path overwrite it.
                Err(err) => {
                    tracing::warn!(key, error = %err, "cached value no longer deserialises");
                    None
                }
            },
            None => {
                tracing::debug!(key, "cache miss");
                None
            }
        }
    }

    // One MGET instead of N GETs. Having just removed an N+1 across gRPC it would
    // be careless to introduce one against Redis.
    pub async fn get_many<T: DeserializeOwned>(&self, keys: &[String]) -> Vec<Option<T>> {
        if keys.is_empty() {
            return Vec::new();
        }
        // MGET with a single key can come back as a scalar rather than an array,
        // so route that case through the plain GET path.
        if keys.len() == 1 {
            return vec![self.get(&keys[0]).await];
        }

        let mut conn = self.conn.clone();

        let raw: Vec<Option<String>> = match tokio::time::timeout(OP_TIMEOUT, conn.mget(keys)).await
        {
            Ok(Ok(values)) => values,
            Ok(Err(err)) => {
                tracing::warn!(error = %err, "cache bulk read failed, falling through");
                // Not vec![None; n]: that needs T: Clone, which callers should not owe us.
                return keys.iter().map(|_| None).collect();
            }
            Err(_) => {
                tracing::warn!("cache bulk read timed out, falling through");
                return keys.iter().map(|_| None).collect();
            }
        };

        raw.into_iter()
            .map(|value| value.and_then(|json| serde_json::from_str(&json).ok()))
            .collect()
    }

    pub async fn put<T: Serialize>(&self, key: &str, value: &T) {
        let Ok(json) = serde_json::to_string(value) else {
            return;
        };

        let mut conn = self.conn.clone();

        // Always with a TTL. An entry without one lives until something explicitly
        // deletes it, which is exactly how a cache silently serves stale data
        // forever after a missed invalidation.
        match tokio::time::timeout(OP_TIMEOUT, conn.set_ex::<_, _, ()>(key, json, self.ttl_secs))
            .await
        {
            Ok(Ok(())) => {}
            Ok(Err(err)) => tracing::warn!(key, error = %err, "cache write failed, ignoring"),
            Err(_) => tracing::warn!(key, "cache write timed out, ignoring"),
        }
    }

    pub async fn invalidate(&self, key: &str) {
        let mut conn = self.conn.clone();

        match tokio::time::timeout(OP_TIMEOUT, conn.del::<_, ()>(key)).await {
            Ok(Ok(())) => tracing::debug!(key, "cache invalidated"),
            Ok(Err(err)) => tracing::warn!(key, error = %err, "cache invalidation failed"),
            // Worth a louder log than a read: a lost invalidation means stale data
            // keeps being served until the TTL rescues us.
            Err(_) => tracing::warn!(key, "cache invalidation timed out"),
        }
    }
}
