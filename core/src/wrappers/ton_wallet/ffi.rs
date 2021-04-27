use std::ffi::CString;
use std::os::raw::c_char;
use std::str::FromStr;

use crate::ffi::StringResult;
use crate::global::{get_keystore, get_transport, get_wallet};
use crate::ExitCode;

use ton_block::MsgAddressInt;

use crate::wrappers::ton_wallet::{send_inner, SignData};
use crate::{cstr_to_string, get_runtime, ok_or_ret};

pub unsafe extern "C" fn send(
    sign_data: *mut c_char,
    answer_port: crate::ffi::SendPort,
    comment: *mut c_char,
    to: *mut c_char,
    amount: libc::c_ulonglong,
) -> ExitCode {
    if sign_data.is_null() {
        return ExitCode::BadSignData;
    }
    let comment = if comment.is_null() {
        None
    } else {
        Some(cstr_to_string!(comment, ExitCode::BadComment))
    };
    if to.is_null() {
        return ExitCode::BadAddress;
    }
    let sign_data = cstr_to_string!(sign_data, ExitCode::BadSignData);
    let sign_data: SignData = ok_or_ret!(serde_json::from_str(&sign_data), ExitCode::BadSignData);
    let to = cstr_to_string!(to, ExitCode::BadAddress);
    let to = ok_or_ret!(MsgAddressInt::from_str(&to), ExitCode::BadAddress);

    send_ffi(answer_port, sign_data, to, amount, comment)
}

fn send_ffi(
    port: crate::ffi::SendPort,
    keystore_type: SignData,
    to: MsgAddressInt,
    amount: u64,
    comment: Option<String>,
) -> ExitCode {
    let _rt = get_runtime!().enter();
    tokio::spawn(async move {
        let keystore = get_keystore().await;
        let wallet = get_wallet().await;
        let transport = get_transport().await;
        let res = send_inner(
            keystore,
            keystore_type,
            to,
            amount,
            wallet,
            transport,
            comment,
        )
        .await;
        let data = match res {
            Ok(_) => StringResult::Ok("".into()),
            Err(e) => StringResult::Error(e.to_string()),
        };
        port.post(serde_json::to_string(&data).unwrap());
    });
    ExitCode::Ok
}
