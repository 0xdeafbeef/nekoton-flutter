use super::KeyStore;
use crate::native_signer::sign_legacy_inner;
use crate::ExitCode;
use ed25519_dalek::PublicKey;
use std::ffi::CString;
use std::os::raw::c_char;
use std::str::Utf8Error;

macro_rules! cstr_to_string {
    ($stri:expr, $ret_val:expr) => {
        match CString::from_raw($stri).to_str() {
            Ok(a) => a.to_string(),
            Err(e) => {
                ::log::error!("Failed decoding {}: {}", stringify!($stri), e);
                return $ret_val;
            }
        }
    };
}

macro_rules! ok_or_ret {
    ($x:expr, $ret_val:expr) => {
        match $x {
            Ok(a) => a,
            Err(e) => {
                ::log::error!("Failed with {}: {}", stringify!($x), e);
                return $ret_val;
            }
        }
    };
}

pub unsafe extern "C" fn sign_legacy(
    keystore: *const KeyStore,
    password: *const c_char,
    key: *const c_char,
    data: *mut c_char,
    data_len: libc::size_t,
) -> ExitCode {
    if keystore.is_null() {
        return ExitCode::BadKeystore;
    }
    if data.is_null() {
        return ExitCode::BadSignData;
    }

    let sign_data: Box<i8> = Box::from_raw(data);
    let password = cstr_to_string!(password, ExitCode::BadPassword);
    let key = cstr_to_string!(key, ExitCode::InvalidPublicKey);
    let key = ok_or_ret!(
        PublicKey::from_bytes(&ok_or_ret!(hex::decode(key), ExitCode::InvalidPublicKey)),
        ExitCode::InvalidPublicKey
    );

    todo!()
    // sign_legacy_inner(&keystore, password);
}
