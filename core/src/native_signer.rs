use nekoton::crypto::{EncryptedKey, Signer, SignerStorage};

#[derive(Default, Clone, Debug)]
pub struct NativeSigner {
    key: Option<EncryptedKey>,
}
