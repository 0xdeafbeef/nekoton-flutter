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

pub async fn send_legacy(keystore: &KeyStore,
                         password: String,
                         key: &PublicKey,
                         data: &[u8],
                         to: MsgAddressInt,
                         ammount: u64,
                         wallet: &TonWallet,
                         comment: Option<String>,
) -> Result<(), SendError> {
    let wallet_g = wallet.wallet.read().await;
    let transport = &wallet.transport;
    let mut wallet: nekoton::core::ton_wallet::TonWallet = *wallet_g;

    let state = match transport.get_contract_state(wallet.address()).await? {
        ContractState::NotExists => {
            log::error!("Contract doesn't exist");
            return Err(SendError::ContractDoesntExist);
        }
        ContractState::Exists(a) => { a.account }
    };
    let comment = comment.map(|x| { match_option!(create_comment_payload(&x)) }).flatten();
    let data = wallet.prepare_transfer(&state.account_state(), to, ammount, false, comment, Expiration::Timeout(60))?;
    match data{
        TransferAction::DeployFirst => {}
        TransferAction::Sign(_) => {}
    }
    wallet.send()
}


pub async fn sign(keystore: &KeyStore){

}


#[derive(Error, Debug)]
enum SendError {
    #[error("transport error")]
    TransportError(String),
    #[error("Constract doesn't exist")]
    ContractDoesntExist,
}

impl From<anyhow::Error> for SendError {
    fn from(e: anyhow::Error) -> Self {
        SendError::TransportError(e.to_string())
    }
}


pub async fn deploy(){
    todo!()
}