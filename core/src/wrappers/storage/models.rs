use nekoton::core::keystore::KeyStore;
use nekoton::crypto::{
    DerivedKeyCreateInput, DerivedKeyExportParams, DerivedKeyUpdateParams, EncryptedKeyCreateInput,
    EncryptedKeyPassword, EncryptedKeyUpdateParams,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum CreateKeyData {
    Derived(DerivedKeyCreateInput),
    Encrypted(EncryptedKeyCreateInput),
}

#[derive(Serialize, Deserialize)]
pub enum UpdateKeyData {
    Derived(DerivedKeyUpdateParams),
    Encrypted(EncryptedKeyUpdateParams),
}

#[derive(Serialize, Deserialize)]
pub enum ExportKeyData {
    Derived(DerivedKeyExportParams),
    Encrypted(EncryptedKeyPassword),
}

pub struct KeyStoreWrapper(KeyStore);

impl KeyStoreWrapper {
    pub fn inner(&self) -> &KeyStore {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut KeyStore {
        &mut self.0
    }
}
