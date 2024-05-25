use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sled::Db;

pub struct CacheDb(Db);

impl CacheDb {
    #[must_use]
    pub fn new() -> Result<Self> {
        let cargo_languagetool_project_dir =
            directories::ProjectDirs::from("in", "ranadeep", "cargo-languagetool")
                .context("failed to get cache directory")?;

        Ok(Self(
            sled::Config::default()
                .path(cargo_languagetool_project_dir.cache_dir())
                .cache_capacity(10_000_000_000)
                .flush_every_ms(Some(1000))
                .open()?,
        ))
    }

    pub fn get_or<K, V>(&self, key: K, func: impl FnOnce(&K) -> Result<V>) -> Result<V>
    where
        K: Serialize,
        V: Serialize + DeserializeOwned,
    {
        Ok(
            if let Some(value) = self.get_raw(serde_json::to_vec(&key)?) {
                serde_json::from_slice(&value)?
            } else {
                let value = func(&key)?;
                self.set_raw(serde_json::to_vec(&key)?, serde_json::to_vec(&value)?)
                    .context("failed to cache value")?;
                value
            },
        )
    }

    #[must_use]
    pub fn get_raw(&self, key: Vec<u8>) -> Option<Vec<u8>> {
        let hashed_key = Self::hashed_key(key);
        self.0.get(hashed_key).ok()?.map(|v| v.to_vec())
    }

    #[must_use]
    pub fn set_raw(&self, key: Vec<u8>, value: Vec<u8>) -> Option<()> {
        let hashed_key = Self::hashed_key(key);
        self.0.insert(hashed_key, value).ok()?;
        Some(())
    }

    #[must_use]
    pub fn hashed_key(key: Vec<u8>) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().into()
    }
}
