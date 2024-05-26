use std::path::Path;

use color_eyre::Result;
use sha2::{Digest, Sha256};
use sled::Db as SledDb;

use crate::cache::Cacheable;

pub struct SledCacheStore(SledDb);

impl Cacheable for SledCacheStore {
    fn new(dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(
            sled::Config::default()
                .path(dir)
                .cache_capacity(10_000_000_000)
                .flush_every_ms(Some(1000))
                .open()?,
        ))
    }

    fn get_hashed_key_raw(&self, hashed_key: [u8; 32]) -> Result<Option<Vec<u8>>> {
        Ok(self.0.get(hashed_key)?.map(|hash| hash.to_vec()))
    }

    fn set_hashed_key_raw(&self, hashed_key: [u8; 32], value: Vec<u8>) -> Result<()> {
        self.0.insert(hashed_key, value)?;
        Ok(())
    }

    fn hashed_key(key: Vec<u8>) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().into()
    }
}
