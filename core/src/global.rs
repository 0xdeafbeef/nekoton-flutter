use crate::{ExitCode, GqlTransport, TonWalletSubscription};
use nekoton::core::keystore::KeyStore;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

pub static RUNTIME_: Lazy<std::io::Result<tokio::runtime::Runtime>> =
    Lazy::new(|| tokio::runtime::Runtime::new());

static WALLET_STATE_: Lazy<Mutex<Option<Arc<TonWalletSubscription>>>> =
    Lazy::new(|| Mutex::default());

static TRANSPORT_: Lazy<Mutex<Option<Arc<GqlTransport>>>> = Lazy::new(|| Mutex::default());

static KEYSTORE_: Lazy<Mutex<Option<Arc<Mutex<Arc<KeyStore>>>>>> = Lazy::new(|| Mutex::default());

#[macro_export]
macro_rules! get_runtime {
    () => {
        match crate::global::RUNTIME_.as_ref() {
            Ok(a) => {
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
                a
            }
            Err(e) => {
                ::log::error!("Failed getting tokio runtime: {}", e);
                return crate::ExitCode::FailedToCreateRuntime;
            }
        }
    };
}

pub async fn get_keystore() -> Arc<Mutex<KeyStore>> {
    match KEYSTORE_.lock().await.as_ref() {
        None => {
            panic!("Attempt to get uncreated keystore");
        }
        Some(a) => a.clone(),
    }
}

pub fn set_keystore(keystore: KeyStore) -> ExitCode {
    let data = Mutex::new(Arc::new(keystore));
    get_runtime!().spawn(async move {
        let mut guard = KEYSTORE_.lock().await;
        guard.replace(Arc::new(data));
    });
    ExitCode::Ok
}

pub async fn get_wallet() -> Arc<TonWalletSubscription> {
    match WALLET_STATE_.lock().await.as_ref() {
        None => {
            panic!("Attempt to get uncreated wallet");
        }
        Some(a) => a.clone(),
    }
}

pub fn set_wallet(wallet: TonWalletSubscription) -> ExitCode {
    let _h = get_runtime!().enter();
    tokio::spawn(async move {
        let mut guard = WALLET_STATE_.lock().await;
        guard.replace(Arc::new(wallet))
    });
    ExitCode::Ok
}

pub async fn get_transport() -> Arc<GqlTransport> {
    match TRANSPORT_.lock().await.as_ref() {
        None => {
            panic!("Attempt to get uncreated transport");
        }
        Some(a) => a.clone(),
    }
}

pub fn set_transport(transport: GqlTransport) -> ExitCode {
    let _h = get_runtime!().enter();
    tokio::spawn(async move {
        let mut guard = TRANSPORT_.lock().await;
        guard.replace(Arc::new(transport))
    });
    ExitCode::Ok
}
