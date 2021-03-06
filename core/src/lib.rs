use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_longlong, c_uint};
use std::sync::Arc;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use nekoton::core::models::{
    ContractState, PendingTransaction, Transaction, TransactionAdditionalInfo, TransactionWithData,
    TransactionsBatchInfo,
};
use nekoton::core::ton_wallet;
use nekoton::core::ton_wallet::compute_address;
use nekoton::transport::gql;
use nekoton::transport::Transport;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::Duration;

use crate::context::{Context, TaskManager};
use crate::external::GqlConnection;
use crate::ffi::IntoDart;
pub use crate::wrappers::send;
use crate::wrappers::storage;
use crate::wrappers::storage::NativeStorage;

mod external;
mod ffi;
mod wrappers;

mod context;
mod global;
pub(crate) mod macros;
mod utils;

pub struct Runtime {}

impl Runtime {
    pub fn new() -> Result<Self> {
        ::android_logger::init_once(
            ::android_logger::Config::default()
                .with_min_level(::log::Level::Info)
                .with_tag("nekoton")
                .with_filter(
                    android_logger::FilterBuilder::new()
                        .parse("ntbindings=debug,reqwest=debug")
                        .build(),
                ),
        );
        log::info!("Created runtime");
        Ok(Self {})
    }
}

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
        Err(_) => {
            return ExitCode::InvalidUrl;
        }
    };
    let storage = match storage::NativeStorage::new(data) {
        Ok(a) => a,
        Err(_) => {
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
    get_runtime!().spawn(async move {
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

#[no_mangle]
pub unsafe extern "C" fn create_context(
    params: TransportParams,
    public_key: *const c_char,
    contract_type: ContractType,
    subscription_port: c_longlong,
    keystore_data: *mut c_char,
    context_ffi: *mut *mut Context,
) -> ExitCode {
    let manager = TaskManager::default();

    let transport = match create_gql_transport(params) {
        None => return ExitCode::InvalidUrl,
        Some(a) => Arc::new(a),
    };
    let wallet = match get_runtime!().block_on(subscribe_to_ton_wallet(
        manager.clone(),
        public_key,
        contract_type,
        transport.inner.clone(),
        subscription_port,
    )) {
        Ok(a) => a,
        Err(e) => {
            return e;
        }
    };
    let keystore = match get_runtime!().block_on(crate::wrappers::storage::ffi::create_keystore(
        keystore_data,
    )) {
        Ok(a) => a,
        Err(e) => {
            return e;
        }
    };
    let context = Box::new(Context::new(wallet, transport, keystore, manager));

    *context_ffi = Box::into_raw(context);
    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn delete_context(context: *mut Context) -> ExitCode {
    if context.is_null() {
        return ExitCode::NoContextProvided;
    }
    Box::from_raw(context);
    ExitCode::Ok
}

#[repr(C)]
pub struct TransportParams {
    pub url: *mut c_char,
}

unsafe fn create_gql_transport(params: TransportParams) -> Option<GqlTransport> {
    let url = CStr::from_ptr(params.url).to_str().ok()?;
    GqlConnection::new(url).map(GqlTransport::new).ok()
}

#[no_mangle]
pub unsafe extern "C" fn delete_gql_transport(gql_transport: *mut GqlTransport) -> ExitCode {
    if gql_transport.is_null() {
        return ExitCode::TransportIsNotInitialized;
    }
    Box::from_raw(gql_transport);
    ExitCode::Ok
}

pub async fn subscribe_to_ton_wallet(
    manager: TaskManager,
    public_key: *const c_char,
    contract_type: ContractType,
    transport: Arc<dyn Transport>,
    subscription_port: c_longlong,
) -> Result<TonWalletSubscription, ExitCode> {
    let public_key = match read_public_key(public_key) {
        Ok(key) => key,
        Err(_) => return Err(ExitCode::InvalidPublicKey),
    };
    let contract_type = contract_type.into();

    log::info!(
        "address: {}",
        compute_address(&public_key, contract_type, 0).to_string()
    );
    let handler = Arc::new(TonWalletSubscriptionHandler::new(subscription_port));
    match ton_wallet::TonWallet::subscribe(transport, public_key, contract_type, handler).await {
        Ok(new_subscription) => {
            let mut wallet = new_subscription.clone();
            let wallet_subscription = TonWalletSubscription {
                inner: new_subscription,
            };
            let handle = tokio::spawn(async move {
                loop {
                    if let Err(e) = wallet.refresh().await {
                        log::error!("Failed refreshing: {}", e);
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            });
            manager.track(handle).await;
            Ok(wallet_subscription)
        }
        Err(_) => Err(ExitCode::FailedToSubscribeToTonWallet),
    }
}

#[no_mangle]
pub unsafe extern "C" fn delete_subscription(subscription: *mut TonWalletSubscription) -> ExitCode {
    if subscription.is_null() {
        return ExitCode::SubscriptionIsNotInitialized;
    }
    Box::from_raw(subscription);
    ExitCode::Ok
}

#[derive(Clone)]
pub struct TonWalletSubscription {
    inner: ton_wallet::TonWallet,
}

struct TonWalletSubscriptionHandler {
    port: ffi::SendPort,
}

#[derive(Serialize, Deserialize)]
enum OnUpdate {
    OnMessageSent(OnMessageSent),
    OnTransactionsFound(OnTransactionsFound),
    OnMessageExpired(PendingTransaction),
    OnStateChanged(ContractState),
}

impl OnUpdate {
    fn prepare(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Deserialize, Serialize)]
pub struct OnMessageSent {
    pending_transaction: PendingTransaction,
    transaction: Option<Transaction>,
}

#[derive(Deserialize, Serialize)]
struct OnTransactionsFound {
    transactions: Vec<TransactionWithData<TransactionAdditionalInfo>>,
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
            OnUpdate::OnMessageSent(OnMessageSent {
                pending_transaction,
                transaction,
            })
            .prepare(),
        );
    }

    fn on_message_expired(&self, pending_transaction: PendingTransaction) {
        // log::debug!("{:?}", &pending_transaction);
        log::debug!("on_message_expired");
        self.port
            .post(OnUpdate::OnMessageExpired(pending_transaction).prepare());
    }

    fn on_state_changed(&self, new_state: ContractState) {
        log::debug!("State changed");
        self.port
            .post(OnUpdate::OnStateChanged(new_state).prepare());
    }

    fn on_transactions_found(
        &self,
        transactions: Vec<TransactionWithData<TransactionAdditionalInfo>>,
        batch_info: TransactionsBatchInfo,
    ) {
        // log::debug!("{:?} {:?}", &transactions, &batch_info);
        log::debug!("on_transactions_found");
        self.port.post(
            OnUpdate::OnTransactionsFound(OnTransactionsFound {
                transactions,
                batch_info,
            })
            .prepare(),
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
    FailedToCreateKeystore,
    FailedToAddKey,
    FailedToRemoveKey,
    FailedToUpdateKey,
    FailedToExportKey,
    InvalidUrl,
    InvalidPublicKey,

    NullOutputPointer,

    NoContextProvided,

    BadPassword,
    BadKeystoreData,
    BadSignData,
    BadWallet,
    BadComment,
    BadAddress,
    BadCreateKeyData,
    BadUpdateData,
    BadExportData,
}

impl IntoDart for ExitCode {
    fn into_dart(self) -> ffi::DartCObject {
        (self as c_int).into_dart()
    }
}
