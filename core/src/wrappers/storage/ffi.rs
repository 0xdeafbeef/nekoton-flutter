use std::os::raw::c_char;

use nekoton::core::keystore::KeyStore;

use super::open_storage;
use crate::utils::ffi_mut_cast;
use crate::wrappers::storage::models::{
    CreateKeyData, ExportKeyData, KeyStoreWrapper, UpdateKeyData,
};
use crate::{cstr_to_string, get_runtime, ok_or_ret, ExitCode};
use crate::{ffi_ensure, read_public_key};
use nekoton::crypto::{DerivedKeySigner, EncryptedKeySigner};
use std::ffi::CString;

pub async unsafe fn create_keystore(data: *mut c_char) -> Result<KeyStore, ExitCode> {
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

#[no_mangle]
pub unsafe extern "C" fn add_key(
    keystore: *mut KeyStoreWrapper,
    key_input: *mut c_char,
    key_name: *mut c_char,
) -> ExitCode {
    ffi_ensure!(
        keystore.is_null(),
        ExitCode::BadKeystoreData,
        "Keystore is null"
    );
    let input = cstr_to_string!(key_input, ExitCode::BadCreateKeyData);
    let key_name = cstr_to_string!(key_name, ExitCode::BadCreateKeyData);
    let keystore = ffi_mut_cast(keystore).inner_mut();
    let key_input: super::models::CreateKeyData =
        ok_or_ret!(serde_json::from_str(&input), ExitCode::BadCreateKeyData);
    let res = get_runtime!().block_on(async {
        match key_input {
            CreateKeyData::Derived(a) => keystore.add_key::<DerivedKeySigner>(&key_name, a).await,
            CreateKeyData::Encrypted(a) => {
                keystore.add_key::<EncryptedKeySigner>(&key_name, a).await
            }
        }
    });
    ok_or_ret!(res, ExitCode::FailedToAddKey);
    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn remove_key(
    keystore: *mut KeyStoreWrapper,
    pubkey: *mut c_char,
) -> ExitCode {
    ffi_ensure!(
        keystore.is_null(),
        ExitCode::BadKeystoreData,
        "Keystore is null"
    );
    let keystore = ffi_mut_cast(keystore).inner_mut();

    let pubkey = ok_or_ret!(read_public_key(pubkey), ExitCode::InvalidPublicKey);
    let res = get_runtime!().block_on(async { keystore.remove_key(&pubkey).await });
    ok_or_ret!(res, ExitCode::FailedToRemoveKey);
    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn update_key(
    keystore: *mut KeyStoreWrapper,
    update_input: *mut c_char,
) -> ExitCode {
    ffi_ensure!(
        keystore.is_null(),
        ExitCode::BadKeystoreData,
        "Keystore is null"
    );
    let keystore = ffi_mut_cast(keystore).inner_mut();
    let up_data = cstr_to_string!(update_input, ExitCode::BadUpdateData);
    let up_data: super::models::UpdateKeyData =
        ok_or_ret!(serde_json::from_str(&up_data), ExitCode::BadUpdateData);
    let res = get_runtime!().block_on(async {
        match up_data {
            UpdateKeyData::Derived(a) => keystore.update_key::<DerivedKeySigner>(a).await,
            UpdateKeyData::Encrypted(a) => keystore.update_key::<EncryptedKeySigner>(a).await,
        }
    });
    ok_or_ret!(res, ExitCode::FailedToUpdateKey);
    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn export_key(
    keystore: *mut KeyStoreWrapper,
    export_data: *mut c_char,
    output: *mut *const c_char,
) -> ExitCode {
    ffi_ensure!(
        keystore.is_null(),
        ExitCode::BadKeystoreData,
        "Keystore is null"
    );
    ffi_ensure!(
        output.is_null(),
        ExitCode::NullOutputPointer,
        "Export is null"
    );
    let keystore = ffi_mut_cast(keystore).inner_mut();
    let export_data = cstr_to_string!(export_data, ExitCode::BadExportData);
    let export_data: ExportKeyData =
        ok_or_ret!(serde_json::from_str(&export_data), ExitCode::BadExportData);
    let export_data = get_runtime!().block_on(async {
        match export_data {
            ExportKeyData::Derived(a) => keystore
                .export_key::<DerivedKeySigner>(a)
                .await
                .map(|x| serde_json::to_string(&x).unwrap()),
            ExportKeyData::Encrypted(a) => keystore
                .export_key::<EncryptedKeySigner>(a)
                .await
                .map(|x| serde_json::to_string(&x).unwrap()),
        }
    });
    let export_data = ok_or_ret!(export_data, ExitCode::FailedToExportKey);
    let export_data = CString::new(export_data).unwrap();
    *output = export_data.into_raw();
    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn clear_keystore(keystore: *mut KeyStoreWrapper) -> ExitCode {
    ffi_ensure!(
        keystore.is_null(),
        ExitCode::BadKeystoreData,
        "Keystore is null"
    );
    let keystore = ffi_mut_cast(keystore);
    let res = get_runtime!().block_on(async { keystore.inner_mut().clear().await });
    ok_or_ret!(res, ExitCode::FailedToRemoveKey);
    ExitCode::Ok
}

// pub async fn clear(&self) -> Result<()> {
//     let mut state = self.state.write().await;
//
//     state.entries.clear();
//     future::join_all(state.signers.values_mut().map(|(_, signer)| signer.clear())).await;
//
//     self.save(&state.signers).await
// }
//
//
