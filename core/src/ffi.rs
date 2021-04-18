use std::ffi::CString;
use std::os::raw::{c_char, c_longlong, c_uchar, c_void};

/// cbindgen:ignore
#[repr(i32)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DartTypedDataType {
    ByteData = 0,
    Int8 = 1,
    Uint8 = 2,
    Uint8Clamped = 3,
    Int16 = 4,
    Uint16 = 5,
    Int32 = 6,
    Uint32 = 7,
    Int64 = 8,
    Uint64 = 9,
    Float32 = 10,
    Float64 = 11,
    Float32x4 = 12,
    Invalid = 13,
}

/// cbindgen:ignore
#[repr(i32)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum DartCObjectType {
    DartNull = 0,
    DartBool = 1,
    DartInt32 = 2,
    DartInt64 = 3,
    DartDouble = 4,
    DartString = 5,
    DartArray = 6,
    DartTypedData = 7,
    DartExternalTypedData = 8,
    DartSendPort = 9,
    DartCapability = 10,
    DartUnsupported = 11,
    DartNumberOfTypes = 12,
}

/// cbindgen:ignore
#[repr(C)]
#[derive(Clone, Copy)]
pub union DartCObjectValue {
    pub as_bool: bool,
    pub as_int32: i32,
    pub as_int64: i64,
    pub as_double: f64,
    pub as_string: *mut c_char,
    pub as_send_port: DartNativeSendPort,
    pub as_capability: DartNativeCapability,
    pub as_array: DartNativeArray,
    pub as_typed_data: DartNativeTypedData,
    _bindgen_union_align: [u64; 5usize],
}

/// cbindgen:ignore
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DartNativeSendPort {
    pub id: DartPort,
    pub origin_id: DartPort,
}

/// cbindgen:ignore
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DartNativeCapability {
    pub id: i64,
}

/// cbindgen:ignore
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DartNativeArray {
    pub length: isize,
    pub values: *mut *mut DartCObject,
}

/// cbindgen:ignore
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DartNativeTypedData {
    pub ty: DartTypedDataType,
    pub length: isize,
    pub values: *mut u8,
}

/// cbindgen:ignore
#[repr(C)]
pub struct DartCObject {
    pub ty: DartCObjectType,
    pub value: DartCObjectValue,
}

pub type DartPort = c_longlong;
pub type DartCObjectPtr = *mut c_void;

pub type DartPostCObjectFnType =
    unsafe extern "C" fn(port_id: DartPort, message: DartCObjectPtr) -> c_uchar;

impl Drop for DartCObject {
    fn drop(&mut self) {
        if self.ty == DartCObjectType::DartString {
            let _ = unsafe { CString::from_raw(self.value.as_string) };
        } else if self.ty == DartCObjectType::DartArray {
            //DartArray::from(unsafe { self.value.as_array });
            todo!()
        } else if self.ty == DartCObjectType::DartTypedData {
            let v = unsafe { self.value.as_typed_data };
            let ty = v.ty;
            match ty {
                DartTypedDataType::Int8 => {
                    let _ = unsafe {
                        Vec::from_raw_parts(
                            v.values as *mut i8,
                            v.length as usize,
                            v.length as usize,
                        )
                    };
                }
                DartTypedDataType::Uint8 => {
                    let _ = unsafe {
                        Vec::from_raw_parts(
                            v.values as *mut u8,
                            v.length as usize,
                            v.length as usize,
                        )
                    };
                }
                _ => {}
            };
        }
    }
}
