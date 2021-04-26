use std::ffi::CString;
use std::os::raw::c_char;
use std::str::Utf8Error;

use ed25519_dalek::PublicKey;
use ton_block::MsgAddressInt;

use crate::{ExitCode, TonWallet};
#[macro_use]
use crate::{cstr_to_string, get_runtime, ok_or_ret};
use crate::wrappers::ton_wallet::SignData;

use super::KeyStore;

pub unsafe extern "C" fn send(
    keystore: *const KeyStore,
    wallet: *const TonWallet,
    sign_data: *mut c_char,
    answer_port: crate::ffi::SendPort,
) -> ExitCode {
    if keystore.is_null() {
        return ExitCode::BadKeystore;
    }

    if wallet.is_null() {
        return ExitCode::BadWallet;
    }

    if sign_data.is_null() {
        return ExitCode::BadSignData;
    }

    let sign_data = cstr_to_string!(sign_data, ExitCode::BadSignData);
    let sign_data: SignData = ok_or_ret!(serde_json::from_str(&sign_data), ExitCode::BadSignData);
    get_runtime!().spawn();
    ExitCode::Ok
}

async fn send_ffi(
    port: crate::ffi::SendPort,
    keystore: &KeyStore,
    keystore_type: SignData,
    to: MsgAddressInt,
    ammount: u64,
    wallet: &TonWallet,
    comment: Option<String>,
) {
    todo!()
}
