use nekoton::core::keystore::KeyStore;
use ed25519_dalek::PublicKey;
use crate::TonWallet;
use thiserror::Error;
use nekoton::transport::models::ContractState;
use ton_block::{MsgAddress, MsgAddressInt};
use nekoton::helpers::abi::create_comment_payload;
use crate::match_option;
use tokio::time::Duration;
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::TransferAction;
use nekoton::crypto::{DerivedKeySignParams, EncryptedKey, EncryptedKeySigner, EncryptedKeyPassword, DerivedKeySigner, UnsignedMessage};
use crate::loge;

pub enum KeyStoreType {
    Derived(DerivedKeySignParams),
    Encrypted(EncryptedKeyPassword),
}

pub async fn send_legacy(keystore: &KeyStore,
                         keystoreType: KeyStoreType,
                         // key: &PublicKey,
                         data: &[u8],
                         to: MsgAddressInt,
                         ammount: u64,
                         wallet: &TonWallet,
                         comment: Option<String>,
) -> Result<(), SendError> {
    let wallet_g = wallet.wallet.read().await;
    let transport = &wallet.transport;
    let mut ton_wallet =  *wallet_g;

    let state = match transport.get_contract_state(ton_wallet.address()).await? {
        ContractState::NotExists => {
            log::error!("Contract doesn't exist");
            return Err(SendError::ContractDoesntExist); //todo should I deploy in this case?
        }
        ContractState::Exists(a) => { a.account }
    };

    let comment = comment.map(|x| { match_option!(create_comment_payload(&x)) }).flatten();
    let prepare_transfer_data = ton_wallet.prepare_transfer(&state, to, ammount, false, comment, Expiration::Timeout(60))?;
    if let TransferAction::DeployFirst = prepare_transfer_data {
        deploy(&keystore, &keystoreType, &mut ton_wallet).await?;
    }
    let mut message =
        match prepare_transfer_data {
            TransferAction::DeployFirst => {
                unreachable!("Realy?") //todo checkme
            }
            TransferAction::Sign(a) => { a }
        };

    sign_and_send(keystore, &keystoreType, &mut ton_wallet, &mut message).await //todo retry loop
}

async fn sign_and_send(keystore: &KeyStore, keystoreType: &KeyStoreType, ton_wallet: &mut nekoton::core::ton_wallet::TonWallet, message: &mut Box<dyn UnsignedMessage>) -> Result<(), SendError> {
    let signature = match
    keystoreType {
        KeyStoreType::Derived(a) => {
            keystore.sign::<DerivedKeySigner>(message.hash(), a)
        }
        KeyStoreType::Encrypted(a) => {
            keystore.sign::<EncryptedKeySigner>(message.hash(), a)
        }
    }.await.map_err(|e| {
        loge!(e);
        SendError::SignError
    })?;
    message.refresh_timeout();
    let singed =
        message.sign(&signature).map_err(|e|
            {
                loge!(e);
                SendError::SignError
            })?;
    ton_wallet.send(&singed.message, singed.expire_at).await.map_err(|e| SendError::TransportError(e.to_string()))?;
    //todo check loop
    Ok(())
}


#[derive(Error, Debug)]
enum SendError {
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


pub async fn deploy(keystore: &KeyStore, keystoreType: &KeyStoreType, wallet: &mut nekoton::core::ton_wallet::TonWallet) -> Result<(), SendError> {
    let mut deploy = wallet.prepare_deploy(Expiration::Timeout(60))?;

    sign_and_send(keystore, keystoreType, wallet, &mut deploy).await
}