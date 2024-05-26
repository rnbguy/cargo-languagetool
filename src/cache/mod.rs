mod sled;

use std::path::Path;

use color_eyre::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
pub use sled::Db as SledCacheDb;

pub trait CacheDb<const HASHED_KEY_SIZE: usize = 32>: Sized {
    /// Create a new cache database.
    ///
    /// # Errors
    /// If the cache db cannot be created.
    fn new(dir: impl AsRef<Path>) -> Result<Self>;

    /// Get a value using a hashed key from the cache database.
    ///
    /// # Errors
    /// If the key or the value cannot be serialized or deserialized.
    fn get_hashed_key_raw(&self, hashed_key: [u8; HASHED_KEY_SIZE]) -> Result<Option<Vec<u8>>>;

    /// Set a value using a hashed key in the cache database.
    ///
    /// # Errors
    /// If the key or the value cannot be serialized or deserialized, or if the value cannot be set.
    fn set_hashed_key_raw(&self, hashed_key: [u8; HASHED_KEY_SIZE], value: Vec<u8>) -> Result<()>;

    /// Hash a key.
    #[must_use]
    fn hashed_key(key: Vec<u8>) -> [u8; HASHED_KEY_SIZE];

    /// Set a value in the cache database and return the value.
    ///
    /// # Errors
    /// If the key or the value cannot be serialized or deserialized.
    fn set_and_get<K, V>(&self, key: &K, func: impl FnOnce(&K) -> Result<V>) -> Result<V>
    where
        K: Serialize,
        V: Serialize,
    {
        let value = func(&key)?;
        self.set_raw(serde_json::to_vec(key)?, serde_json::to_vec(&value)?)?;
        Ok(value)
    }

    /// Try to get a value from the cache database.
    ///
    /// # Errors
    /// If the key or the value cannot be serialized or deserialized.
    fn try_get<K, V>(&self, key: &K) -> Result<Option<V>>
    where
        K: Serialize,
        V: Serialize + DeserializeOwned,
    {
        let value = self.get_raw(serde_json::to_vec(&key)?)?;
        Ok(value.map(|v| serde_json::from_slice(&v)).transpose()?)
    }

    /// Check if a key exists in the cache database.
    ///
    /// # Errors
    /// If the key cannot be serialized.
    fn hits<K>(&self, key: &K) -> Result<bool>
    where
        K: Serialize,
    {
        let hashed_key = Self::hashed_key(serde_json::to_vec(key)?);
        Ok(self.get_hashed_key_raw(hashed_key)?.is_some())
    }

    /// Get a value from the cache database.
    ///
    /// # Errors
    /// If the key or the value cannot be serialized or deserialized.
    fn get_or<K, V>(&self, key: &K, func: impl FnOnce(&K) -> Result<V>) -> Result<V>
    where
        K: Serialize,
        V: Serialize + DeserializeOwned,
    {
        (self.try_get(key)?).map_or_else(|| self.set_and_get(key, func), |value| Ok(value))
    }

    /// Get a value from the cache database.
    ///
    /// # Errors
    /// If the key or the value cannot be serialized or deserialized.
    fn get_raw(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let hashed_key = Self::hashed_key(key);
        self.get_hashed_key_raw(hashed_key)
    }

    /// Set a value in the cache database.
    ///
    /// # Errors
    /// If the key or the value cannot be serialized or deserialized, or if the value cannot be set.
    fn set_raw(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let hashed_key = Self::hashed_key(key);
        self.set_hashed_key_raw(hashed_key, value)
    }
}
