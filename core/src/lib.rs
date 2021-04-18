mod external;
mod ffi;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_longlong, c_uchar, c_uint};
use std::sync::Arc;

use anyhow::Result;

use nekoton::external::GqlConnection;

pub struct CoreState {}

pub struct Runtime {
    runtime: Arc<tokio::runtime::Runtime>,
}

impl Runtime {
    pub fn new(worker_threads: usize) -> Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(worker_threads)
                .enable_all()
                .build()?,
        );

        std::thread::spawn({
            let runtime = runtime.clone();
            move || {
                runtime.block_on(async move {
                    futures::future::pending::<()>().await;
                });
            }
        });

        Ok(Self { runtime })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SendPort {
    port: i64,
}

impl SendPort {
    pub const fn new(port: i64) -> Self {
        Self { port }
    }

    pub fn post(&self, msg: ffi::DartCObject) -> bool {
        unsafe {
            if let Some(func) = POST_COBJECT {
                let boxed_msg = Box::new(msg);

                let ptr = Box::into_raw(boxed_msg);
                let result = func(self.port, ptr as ffi::DartCObjectPtr);
                Box::from_raw(ptr);

                result != 0
            } else {
                false
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn init(post_cobject: ffi::DartPostCObjectFnType) {
    POST_COBJECT = Some(post_cobject);
}

#[repr(C)]
pub struct RuntimeParams {
    worker_threads: c_uint,
}

#[no_mangle]
pub unsafe extern "C" fn create_runtime(
    params: RuntimeParams,
    runtime: *mut *const Runtime,
) -> ExitCode {
    if runtime.is_null() {
        return ExitCode::FailedToCreateRuntime;
    }

    match Runtime::new(params.worker_threads as usize) {
        Ok(new_runtime) => {
            *runtime = Box::into_raw(Box::new(new_runtime));
            ExitCode::Ok
        }
        Err(_) => ExitCode::FailedToCreateRuntime,
    }
}

#[no_mangle]
pub unsafe extern "C" fn delete_runtime(runtime: *mut Runtime) -> ExitCode {
    if runtime.is_null() {
        return ExitCode::RuntimeIsNotInitialized;
    }
    Box::from_raw(runtime);
    ExitCode::Ok
}

#[no_mangle]
pub unsafe extern "C" fn wait(
    runtime: *mut Runtime,
    seconds: c_uint,
    send_port: c_longlong,
) -> ExitCode {
    if runtime.is_null() {
        return ExitCode::RuntimeIsNotInitialized;
    }

    (*runtime).runtime.spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(seconds as u64)).await;

        SendPort::new(send_port).post(ffi::DartCObject {
            ty: ffi::DartCObjectType::DartNull,
            value: ffi::DartCObjectValue { as_bool: false },
        });
    });

    ExitCode::Ok
}

#[repr(C)]
pub enum ExitCode {
    Ok = 0,
    FailedToCreateRuntime = 1,
    RuntimeIsNotInitialized = 2,
}

static mut POST_COBJECT: Option<ffi::DartPostCObjectFnType> = None;
