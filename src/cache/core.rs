use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, RwLock};

type RespositoryTuple = (Vec<u8>, Option<Instant>);
type Repository = RwLock<HashMap<String, RespositoryTuple>>;

#[derive(Debug)]
pub struct CacheRepository {
    pub repo: Repository,
    pub expiry_map: RwLock<HashMap<Instant, HashSet<String>>>,
    pub curr_transaction_id: Mutex<Option<String>>,
}

impl Default for CacheRepository {
    fn default() -> Self {
        CacheRepository {
            repo: RwLock::new(HashMap::new()),
            expiry_map: RwLock::new(HashMap::new()),
            curr_transaction_id: Mutex::new(None),
        }
    }
}

impl CacheRepository {
    pub fn now(&self) -> Instant {
        Instant::now()
    }

    pub async fn get(&self, key: String) -> Option<Vec<u8>> {
        let repo = self.repo.read().await;
        if let Some((vec, ttl_option)) = repo.get(&key) {
            if let Some(interval) = ttl_option.as_ref() {
                if *interval < self.now() {
                    return None;
                }
            }

            return Some(vec.to_vec());
        }
        None
    }

    pub async fn set(&self, key: String, buff: Vec<u8>) -> std::io::Result<()> {
        if let Some((_, Some(ttl))) = {
            self.repo
                .write()
                .await
                .insert(key.to_string(), (buff.to_vec(), None))
        } {
            self.remove_key_from_ttl_set_if_exists(key.to_string(), ttl)
                .await;
        }
        Ok(())
    }

    pub async fn set_with_expiry(
        &mut self,
        key: String,
        buff: Vec<u8>,
        ttl: u64,
    ) -> std::io::Result<()> {
        let expiry = self.now() + Duration::from_millis(ttl);
        if let Some((_, Some(ttl))) = {
            self.repo
                .write()
                .await
                .insert(key.to_string(), (buff.to_vec(), Some(expiry)))
        } {
            self.remove_key_from_ttl_set_if_exists(key.to_string(), ttl)
                .await;
        }

        let mut expiry_map = self.expiry_map.write().await;

        expiry_map.entry(expiry).or_insert(HashSet::new());
        expiry_map.get_mut(&expiry).unwrap().insert(key.to_string());

        Ok(())
    }

    async fn remove_key_from_ttl_set_if_exists(&self, key: String, expiry: Instant) {
        if let Some(key_set) = self.expiry_map.write().await.get_mut(&expiry) {
            key_set.remove(&key.to_string());
        }
    }

    pub async fn actively_remove_expired_keys(&self) {
        let map = self.expiry_map.write().await;
        let expired_key_instants: Vec<&Instant> =
            map.keys().filter(|expiry| **expiry < self.now()).collect();

        for expired_key_instant in expired_key_instants {
            if let Some(expired_keys) = map.clone().get_mut(expired_key_instant) {
                for key in expired_keys.iter() {
                    self.repo.write().await.remove(&key.to_string());
                }
            }
        }
    }

    pub async fn set_transaction(&self, id: String) {
        let mut curr_transaction_id = self.curr_transaction_id.lock().await;
        curr_transaction_id.replace(id.to_string());
    }

    pub async fn unset_transaction(&self) {
        let mut curr_transaction_id = self.curr_transaction_id.lock().await;
        curr_transaction_id.take();
    }

    pub async fn get_transaction_id(&self) -> Option<String> {
        if let Some(id) = self.curr_transaction_id.lock().await.as_ref() {
            return Some(id.to_string());
        }
        None
    }
}

unsafe impl Send for CacheRepository {}
unsafe impl Sync for CacheRepository {}
