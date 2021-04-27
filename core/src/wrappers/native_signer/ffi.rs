use super::open_storage;
use crate::{cstr_to_string, get_runtime, ok_or_ret, ExitCode};
use anyhow::Error;
use nekoton::core::keystore::KeyStore;
use std::os::raw::c_char;

pub unsafe extern "C" fn create_keystore(
    data: *const c_char,
    keystore: *const *mut KeyStore,
) -> ExitCode {
    if data.is_null() {
        ExitCode::BadKeystoreData
    }
    let data = cstr_to_string!(data, ExitCode::BadKeystoreData);
    let ks = ok_or_ret!(
        (get_runtime!().block_on(open_storage(&data))),
        ExitCode::FailedToCreateKeystore
    );
    let ks = Box::new(ks);
    **keystore = *Box::into_raw(ks);
    ExitCode::Ok
}
