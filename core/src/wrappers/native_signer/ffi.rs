use std::os::raw::c_char;

use nekoton::core::keystore::KeyStore;

use crate::{cstr_to_string, ExitCode, ok_or_ret};

use super::open_storage;

pub async unsafe fn create_keystore(
    data: *mut c_char,
) -> Result<KeyStore, ExitCode> {
    if data.is_null() {
        return Err(ExitCode::BadKeystoreData);
    }
    let data = cstr_to_string!(data, Err(ExitCode::BadKeystoreData));
    let ks = ok_or_ret!(
       open_storage(&data).await,
        Err(ExitCode::FailedToCreateKeystore)
    );
    let ks = ks;
    Ok(ks)
}
