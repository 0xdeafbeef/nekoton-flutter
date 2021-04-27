use crate::{GqlTransport, TonWalletSubscription};
use nekoton::core::keystore::KeyStore;

#[derive(Clone)]
pub struct Context {
    wallet_state: TonWalletSubscription,
    transport: GqlTransport,
    keystore: KeyStore,
}

impl Context {
    pub fn new(
        wallet_state: TonWalletSubscription,
        transport: GqlTransport,
        keystore: KeyStore,
    ) -> Self {
        Self {
            wallet_state,
            transport,
            keystore,
        }
    }
}
