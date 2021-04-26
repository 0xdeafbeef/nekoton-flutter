use super::KeyStore;
use crate::{ExitCode, TonWallet};
use ed25519_dalek::PublicKey;
use std::ffi::CString;
use std::os::raw::c_char;
use std::str::Utf8Error;

use crate::wrappers::native_signer::sign_legacy_inner;
use crate::{cstr_to_string, ok_or_ret};

pub unsafe extern "C" fn send(
    keystore: *const KeyStore,
    wallet: *const TonWallet,
    password: *mut c_char,
    key: *mut c_char,
    data: *const c_char,
    data_len: libc::size_t,
    answer_port: crate::ffi::DartPort,
) -> ExitCode {
    if keystore.is_null() {
        return ExitCode::BadKeystore;
    }
    if data.is_null() {
        return ExitCode::BadSignData;
    }

    let sign_data: Vec<u8> = std::slice::from_raw_parts(data, data_len)
        .iter()
        .map(|x| *x as u8)
        .collect();
    let password = cstr_to_string!(password, ExitCode::BadPassword);
    let key = cstr_to_string!(key, ExitCode::InvalidPublicKey);
    let key = ok_or_ret!(
        PublicKey::from_bytes(&ok_or_ret!(hex::decode(key), ExitCode::InvalidPublicKey)),
        ExitCode::InvalidPublicKey
    );
    let sign = sign_legacy_inner(&*keystore, password, &key, &sign_data);
}
