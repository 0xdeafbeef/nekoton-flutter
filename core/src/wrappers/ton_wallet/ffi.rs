use std::os::raw::{c_char, c_longlong};
use std::str::FromStr;
use std::sync::Arc;

use ton_block::MsgAddressInt;

use crate::context::Context;
use crate::ffi::StringResult;
use crate::wrappers::ton_wallet::{send_inner, SignData};
use crate::ExitCode;
use crate::{cstr_to_string, get_runtime, ok_or_ret};

#[no_mangle]
pub unsafe extern "C" fn send(
    ctx: *mut Context,
    sign_data: *mut c_char,
    answer_port: c_longlong,
    comment: *mut c_char,
    to: *mut c_char,
    amount: libc::c_ulonglong,
) -> ExitCode {
    if ctx.is_null() {
        return ExitCode::NoContextProvided;
    }

    if sign_data.is_null() {
        return ExitCode::BadSignData;
    }
    let context = Box::from_raw(ctx).into();
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

    send_ffi(answer_port, sign_data, to, amount, comment, context)
}

fn send_ffi(
    port: c_longlong,
    keystore_type: SignData,
    to: MsgAddressInt,
    amount: u64,
    comment: Option<String>,
    context: Arc<Context>,
) -> ExitCode {
    let _rt = get_runtime!().enter();
    let (keystore, wallet, transport) = (
        context.keystore.clone(),
        context.wallet_state.clone(),
        context.transport.clone(),
    );

    context.spawn(async move {
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
        let port = crate::ffi::SendPort::new(port);
        port.post(serde_json::to_string(&data).unwrap());
    });
    ExitCode::Ok
}
