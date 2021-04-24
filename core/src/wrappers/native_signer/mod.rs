mod ffi;

use nekoton::crypto::{
    DerivedKeySigner, EncryptedKey, EncryptedKeySigner, Signature, Signer, SignerStorage,
};

use anyhow;
use anyhow::Error;
use async_trait::async_trait;
use ed25519_dalek::PublicKey;
use nekoton::core::keystore::KeyStore;
use nekoton::external::Storage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::TonWallet;

#[derive(Clone)]
pub struct NativeStorage {
    inner: Arc<RwLock<HashMap<String, String>>>,
}

impl NativeStorage {
    pub fn new(data: &str) -> Result<Self, Error> {
        let map: HashMap<String, String> = serde_json::from_str(data)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(map)),
        })
    }

    pub async fn dump(&self) -> Result<String, Error> {
        let data = self.inner.read().await;
        Ok(serde_json::to_string(&*data)?)
    }
}

#[async_trait]
impl Storage for NativeStorage {
    async fn get(&self, key: &str) -> Result<Option<String>, Error> {
        Ok(self.inner.read().await.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> Result<(), Error> {
        self.inner
            .write()
            .await
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn set_unchecked(&self, key: &str, value: &str) {
        let (key, value) = (key.to_string(), value.to_string());
        let store = self.clone();
        tokio::spawn(async move {
            let _ = store.set(&key, &value).await;
        });
    }

    async fn remove(&self, key: &str) -> Result<(), Error> {
        self.inner.write().await.remove(key);
        Ok(())
    }

    fn remove_unchecked(&self, key: &str) {
        let key = key.to_string();
        let store = self.clone();
        tokio::spawn(async move {
            let _ = store.remove(&key).await;
        });
    }
}

pub async fn open_storage(data: &str) -> Result<KeyStore, Error> {
    let storage = NativeStorage::new(data)?;
    let storage = Arc::new(storage) as Arc<dyn Storage>;
    let der_signer = DerivedKeySigner::new();

    let signer = EncryptedKeySigner::new();
    let keystore = KeyStore::builder(storage)
        .with_signer("encrypted", signer)?
        .with_signer("derived", der_signer)?
        .load()
        .await?;

    Ok(keystore)
}

async fn sign_legacy_inner(
    keystore: &KeyStore,
    password: String,
    key: &PublicKey,
    data: &[u8],
) -> Result<Signature, Error> {
    use nekoton::crypto::EncryptedKeyPassword;
    keystore
        .sign::<EncryptedKeySigner>(
            data,
            EncryptedKeyPassword {
                public_key: *key,
                password: password.into(),
            },
        )
        .await
}

pub async fn sign_labs(
    keystore: &KeyStore,
    password: String,
    key: &PublicKey,
    data: &[u8],
) -> Result<Signature, Error> {
    todo!()
}


