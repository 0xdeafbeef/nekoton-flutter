mod external;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uchar, c_uint};

use anyhow::Result;

use nekoton::external::GqlConnection;

pub struct CoreState {}

pub struct Runtime {
    runtime: tokio::runtime::Runtime,
}

impl Runtime {
    pub fn new(worker_threads: usize) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .build()?;

        Ok(Self { runtime })
    }
}

#[repr(C)]
pub struct RuntimeParams {
    worker_threads: c_uint,
}

#[no_mangle]
pub unsafe extern "C" fn create_runtime(
    params: RuntimeParams,
    runtime: *mut *const Runtime,
) -> c_uchar {
    if runtime.is_null() {
        return ExitCode::FailedToCreateRuntime as c_uchar;
    }

    match Runtime::new(params.worker_threads as usize) {
        Ok(new_runtime) => {
            *runtime = Box::into_raw(Box::new(new_runtime));

            ExitCode::Ok as c_uchar
        }
        Err(e) => {
            eprintln!("{:?}", e);
            ExitCode::FailedToCreateRuntime as c_uchar
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn delete_runtime(runtime: *mut Runtime) {
    Box::from_raw(runtime);
}

#[repr(u8)]
pub enum ExitCode {
    Ok = 0,
    FailedToCreateRuntime = 1,
}

#[no_mangle]
pub extern "C" fn rust_greeting(to: *const c_char) -> *mut c_char {
    let c_str = unsafe { CStr::from_ptr(to) };
    let recipient = match c_str.to_str() {
        Err(_) => "there",
        Ok(string) => string,
    };

    CString::new("Hello ".to_owned() + recipient)
        .unwrap()
        .into_raw()
}

#[no_mangle]
pub extern "C" fn rust_cstr_free(s: *mut c_char) {
    unsafe {
        if s.is_null() {
            return;
        }
        CString::from_raw(s)
    };
}
