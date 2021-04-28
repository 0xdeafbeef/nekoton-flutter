use std::sync::Arc;

use nekoton::core::keystore::KeyStore;
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::TransferAction;
use nekoton::crypto::{
    DerivedKeySigner, DerivedKeySignParams, EncryptedKeyPassword, EncryptedKeySigner,
    UnsignedMessage,
};
use nekoton::helpers::abi::create_comment_payload;
use nekoton::transport::models::ContractState;
use nekoton::transport::Transport;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use tokio::time::Duration;
use ton_block::MsgAddressInt;

use crate::{GqlTransport, TonWalletSubscription};
use crate::match_option;
use crate::wrappers::ton_wallet::SendError::TransportError;
use tokio::sync::Mutex;

mod ffi;

#[derive(Serialize, Deserialize)]
pub enum SignData {
    Derived(DerivedKeySignParams),
    Encrypted(EncryptedKeyPassword),
}

pub async fn send_inner(
    keystore: Arc<Mutex<KeyStore>>,
    keystore_type: SignData,
    to: MsgAddressInt,
    amount: u64,
    ton_wallet: Arc<TonWalletSubscription>,
    transport: Arc<GqlTransport>,
    comment: Option<String>,
) -> Result<(), SendError> {
    let mut ton_wallet = ton_wallet.inner.clone();
    let transport = transport.inner.clone();
    tokio::runtime::Builder::new_current_thread();
    let state = match transport.get_contract_state(ton_wallet.address()).await? {
        ContractState::NotExists => {
            log::error!("Contract doesn't exist");
            return Err(SendError::ContractDoesntExist); //todo should I deploy in this case?
        }
        ContractState::Exists(a) => a.account,
    };

    let comment = comment
        .map(|x| match_option!(create_comment_payload(&x)))
        .flatten();
    let prepare_transfer_data =
        ton_wallet.prepare_transfer(&state, to, amount, false, comment, Expiration::Timeout(60))?;
    let keystore = keystore.lock().await;
    if let TransferAction::DeployFirst = prepare_transfer_data {
        deploy(&keystore, &keystore_type, &mut ton_wallet).await?;
    }
    let mut message = match prepare_transfer_data {
        TransferAction::DeployFirst => {
            unreachable!("Really?") //todo checkme
        }
        TransferAction::Sign(a) => a,
    };

    while let Err(e) = tokio::time::timeout(
        Duration::from_secs(60),
        sign_and_send(&keystore, &keystore_type, &mut ton_wallet, &mut message),
    )
        .await
    {

    }
    Ok(())
}

async fn get_balance(wallet: &mut nekoton::core::ton_wallet::TonWallet) -> Result<u64, SendError> {
    wallet
        .refresh()
        .await
        .map_err(|e| TransportError(e.to_string()))?;
    Ok(wallet.account_state().balance)
}

async fn sign_and_send(
    keystore: &KeyStore,
    keystore_type: &SignData,
    ton_wallet: &mut nekoton::core::ton_wallet::TonWallet,
    message: &mut Box<dyn UnsignedMessage>,
) -> Result<(), SendError> {
    message.refresh_timeout();
    let hash = message.hash();
    let signature = match keystore_type {
        SignData::Derived(a) => keystore.sign::<DerivedKeySigner>(hash, a.clone()).await,
        SignData::Encrypted(a) => keystore.sign::<EncryptedKeySigner>(hash, a.clone()).await,
    }
        .map_err(|e| {
            log::error!("Failed singing: {}", e);
            SendError::SignError
        })?;
    let singed = message.sign(&signature).map_err(|e| {
        log::error!("Failed signing: {}", e);
        SendError::SignError
    })?;
    let initial_balance = get_balance(ton_wallet).await?;

    ton_wallet
        .send(&singed.message, singed.expire_at)
        .await
        .map_err(|e| SendError::TransportError(e.to_string()))?;
    while initial_balance > get_balance(ton_wallet).await? {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    //todo check loop
    Ok(())
}

#[derive(Error, Debug)]
pub enum SendError {
    #[error("transport error")]
    TransportError(String),
    #[error("Constract doesn't exist")]
    ContractDoesntExist,
    #[error("Sign error")]
    SignError,
    #[error("Deploy error")]
    DeployError,
}

impl From<anyhow::Error> for SendError {
    fn from(e: anyhow::Error) -> Self {
        SendError::TransportError(e.to_string())
    }
}

pub async fn deploy(
    keystore: &KeyStore,
    keystore_type: &SignData,
    wallet: &mut nekoton::core::ton_wallet::TonWallet,
) -> Result<(), SendError> {
    let mut deploy = wallet.prepare_deploy(Expiration::Timeout(60))?;

    sign_and_send(keystore, keystore_type, wallet, &mut deploy).await
}
