mod external;
mod ffi;
mod wrappers;

#[macro_use]
mod macros;
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_longlong, c_uchar, c_uint};
use std::sync::Arc;

use anyhow::{Error, Result};
use ed25519_dalek::PublicKey;
use tokio::sync::RwLock;

use nekoton::core::models::{AccountState, PendingTransaction, Transaction, TransactionsBatchInfo};
use nekoton::core::ton_wallet;
use nekoton::transport::gql;
use nekoton::transport::Transport;
use crate::wrappers::native_signer::NativeStorage;

use crate::external::GqlConnection;
use crate::ffi::IntoDart;
use android_logger::{Config, FilterBuilder};
use log::Level;
use nekoton::core::ton_wallet::compute_address;
use once_cell::sync::Lazy;
use tokio::task::JoinHandle;

static RUNTIME_: Lazy<std::io::Result<tokio::runtime::Runtime>> =
    Lazy::new(|| tokio::runtime::Runtime::new());

macro_rules! tokio {
    () => {
        match RUNTIME_.as_ref() {
            Ok(a) => {
                android_logger::init_once(Config::default().with_min_level(Level::Trace));
                a
            }
            Err(e) => {
                log::error!("Failed getting tokio runtime: {}", e);
                return ExitCode::FailedToCreateRuntime;
            }
        }
    };
}


pub struct CoreState {}

pub struct Runtime {}

impl Runtime {
    pub fn new() -> Result<Self> {
        android_logger::init_once(
            Config::default()
                .with_min_level(Level::Info)
                .with_tag("mytag")
                .with_filter(
                    FilterBuilder::new()
                        .parse("ntbindings=debug,reqwest=debug")
                        .build(),
                ),
        );
        log::info!("Created runtime");
        Ok(Self {})
    }
}

// struct TaskManager<T> {
//     tasks: Vec<JoinHandle<T>>,
// }
//
// impl Drop for TaskManager<T> {
//     fn drop(&mut self) {
//         let _handle = tokio!().enter();
//         for task in &self.tasks {
//             task.abort();
//         }
//     }
// }

#[no_mangle]
pub unsafe extern "C" fn create_storage(
    data: *const c_char,
    storage_ptr: *mut *const NativeStorage,
) -> ExitCode {
    if data.is_null() {
        return ExitCode::InvalidUrl;
    }
    let data = match CStr::from_ptr(data).to_str() {
        Ok(a) => a,
        Err(e) => {
            return ExitCode::InvalidUrl;
        }
    };
    let storage = match native_signer::NativeStorage::new(data) {
        Ok(a) => a,
        Err(e) => {
            return ExitCode::InvalidUrl;
        }
    };

    *storage_ptr = Box::into_raw(Box::new(storage));
    ExitCode::Ok
}

pub struct TonWallet {
    transport: Arc<dyn Transport>,
    wallet: Arc<RwLock<ton_wallet::TonWallet>>,
}

#[no_mangle]
pub unsafe extern "C" fn init(post_cobject: ffi::DartPostCObjectFnType) {
    ffi::POST_COBJECT = Some(post_cobject);
}

#[no_mangle]
pub unsafe extern "C" fn wait(seconds: c_uint, send_port: c_longlong) -> ExitCode {
    tokio!().spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(seconds as u64)).await;

        ffi::SendPort::new(send_port).post(());
    });

    ExitCode::Ok
}

pub struct GqlTransport {
    inner: Arc<gql::GqlTransport>,
}

impl GqlTransport {
    pub fn new(connection: GqlConnection) -> Self {
        Self {
            inner: Arc::new(gql::GqlTransport::new(Arc::new(connection))),
        }
    }
}

#[repr(C)]
pub struct TransportParams {
    pub url: *mut c_char,
}

#[no_mangle]
pub unsafe extern "C" fn create_gql_transport(
    params: TransportParams,
    gql_transport: *mut *const GqlTransport,
) -> ExitCode {
    let url = match CStr::from_ptr(params.url).to_str() {
        Ok(url) => url,
        Err(_) => return ExitCode::InvalidUrl,
    };

    match GqlConnection::new(url) {
        Ok(connection) => {
            *gql_transport = Box::into_raw(Box::new(GqlTransport::new(connection)));
            ExitCode::Ok
        }
        Err(_) => ExitCode::InvalidUrl,
    }
}

#[no_mangle]
pub unsafe extern "C" fn delete_gql_transport(gql_transport: *mut GqlTransport) -> ExitCode {
    if gql_transport.is_null() {
        return ExitCode::TransportIsNotInitialized;
    }
    Box::from_raw(gql_transport);
    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn subscribe_to_ton_wallet(
    gql_transport: *mut GqlTransport,
    public_key: *const c_char,
    contract_type: ContractType,
    subscription_port: c_longlong,
    result_port: c_longlong,
) -> ExitCode {
    if gql_transport.is_null() {
        return ExitCode::TransportIsNotInitialized;
    }

    let public_key = match read_public_key(public_key) {
        Ok(key) => key,
        Err(_) => return ExitCode::InvalidPublicKey,
    };
    let contract_type = contract_type.into();

    let handler = Arc::new(TonWalletSubscriptionHandler::new(subscription_port));
    let result_port = ffi::SendPort::new(result_port);

    let transport = (*gql_transport).inner.clone();

    tokio!().spawn(async move {
        log::info!(
            "address: {}",
            compute_address(&public_key, contract_type, 0).to_string()
        );
        match ton_wallet::TonWallet::subscribe(transport, public_key, contract_type, handler).await
        {
            Ok(new_subscription) => {
                let mut wallet = new_subscription.clone();
                let subscription = Box::into_raw(Box::new(TonWalletSubscription {
                    inner: new_subscription,
                }));
                let token = tokio::spawn(async move {
                    log::info!("Started refresh loop");
                    loop {
                        loge!(wallet.refresh().await);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                });

                result_port.post((ExitCode::Ok, subscription));
            }
            Err(_) => {
                result_port.post((
                    ExitCode::FailedToSubscribeToTonWallet,
                    std::ptr::null::<TonWalletSubscription>(),
                ));
            }
        }
    });

    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn delete_subscription(subscription: *mut TonWalletSubscription) -> ExitCode {
    if subscription.is_null() {
        return ExitCode::SubscriptionIsNotInitialized;
    }
    Box::from_raw(subscription);
    ExitCode::Ok
}

pub struct TonWalletSubscription {
    inner: ton_wallet::TonWallet,
}

struct TonWalletSubscriptionHandler {
    port: ffi::SendPort,
}

#[derive(Deserialize, Serialize)]
pub struct OnMessageSent {
    pending_transaction: PendingTransaction,
    transaction: Option<Transaction>,
}

#[derive(Deserialize, Serialize)]
struct OnTransactionsFound {
    transactions: Vec<Transaction>,
    batch_info: TransactionsBatchInfo,
}

impl TonWalletSubscriptionHandler {
    pub fn new(port: i64) -> Self {
        Self {
            port: ffi::SendPort::new(port),
        }
    }
}

impl ton_wallet::TonWalletSubscriptionHandler for TonWalletSubscriptionHandler {
    fn on_message_sent(
        &self,
        pending_transaction: PendingTransaction,
        transaction: Option<Transaction>,
    ) {
        // log::debug!("{:?} {:?}", &pending_transaction, &transaction);
        log::debug!("on_message_sent");
        self.port.post(
            serde_json::to_string(&OnMessageSent {
                pending_transaction,
                transaction,
            })
            .expect("oops"),
        );
    }

    fn on_message_expired(&self, pending_transaction: PendingTransaction) {
        // log::debug!("{:?}", &pending_transaction);
        log::debug!("on_message_expired");
        self.port
            .post(serde_json::to_string(&pending_transaction).expect("oops"));
    }

    fn on_state_changed(&self, new_state: AccountState) {
        log::debug!("State changed");
        self.port.post(new_state.balance);
    }

    fn on_transactions_found(
        &self,
        transactions: Vec<Transaction>,
        batch_info: TransactionsBatchInfo,
    ) {
        // log::debug!("{:?} {:?}", &transactions, &batch_info);
        log::debug!("on_transactions_found");
        self.port.post(
            serde_json::to_string(&{
                OnTransactionsFound {
                    transactions,
                    batch_info,
                }
            })
            .expect("oops"),
        );
    }
}

fn read_public_key(public_key: *const c_char) -> Result<PublicKey> {
    if public_key.is_null() {
        return Err(NekotonError::NullPointerPassed.into());
    }

    let public_key = unsafe { CStr::from_ptr(public_key) }.to_str()?;
    let data = hex::decode(public_key)?;
    let public_key = PublicKey::from_bytes(&data)?;
    Ok(public_key)
}

#[repr(C)]
pub enum ContractType {
    SafeMultisig,
    SafeMultisig24h,
    SetcodeMultisig,
    Surf,
    WalletV3,
}

impl From<ContractType> for ton_wallet::ContractType {
    fn from(t: ContractType) -> Self {
        match t {
            ContractType::SafeMultisig => {
                ton_wallet::ContractType::Multisig(ton_wallet::MultisigType::SafeMultisigWallet)
            }
            ContractType::SafeMultisig24h => {
                ton_wallet::ContractType::Multisig(ton_wallet::MultisigType::SafeMultisigWallet24h)
            }
            ContractType::SetcodeMultisig => {
                ton_wallet::ContractType::Multisig(ton_wallet::MultisigType::SetcodeMultisigWallet)
            }
            ContractType::Surf => {
                ton_wallet::ContractType::Multisig(ton_wallet::MultisigType::SurfWallet)
            }
            ContractType::WalletV3 => ton_wallet::ContractType::WalletV3,
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum NekotonError {
    #[error("Null pointer passed")]
    NullPointerPassed,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum ExitCode {
    Ok = 0,

    FailedToCreateRuntime,
    RuntimeIsNotInitialized,
    TransportIsNotInitialized,
    SubscriptionIsNotInitialized,
    FailedToSubscribeToTonWallet,

    InvalidUrl,
    InvalidPublicKey,

    BadPassword,
    BadKeystore,
    BadSignData,
}

impl IntoDart for ExitCode {
    fn into_dart(self) -> ffi::DartCObject {
        (self as c_int).into_dart()
    }
}
