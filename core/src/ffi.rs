use std::ffi::CString;
use std::mem::ManuallyDrop;
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
            DartArray::from(unsafe { self.value.as_array });
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

#[derive(Copy, Clone, Debug)]
pub struct SendPort {
    port: i64,
}

impl SendPort {
    pub const fn new(port: i64) -> Self {
        Self { port }
    }

    pub fn post(&self, msg: impl IntoDart) -> bool {
        unsafe {
            if let Some(func) = POST_COBJECT {
                let boxed_msg = Box::new(msg.into_dart());

                let ptr = Box::into_raw(boxed_msg);
                let result = func(self.port, ptr as DartCObjectPtr);
                Box::from_raw(ptr);

                result != 0
            } else {
                false
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DartArray {
    inner: Box<[*mut DartCObject]>,
}

impl<T: IntoDart> From<Vec<T>> for DartArray {
    fn from(vec: Vec<T>) -> Self {
        let vec: Vec<_> = vec
            .into_iter()
            .map(IntoDart::into_dart)
            .map(Box::new)
            .map(Box::into_raw)
            .collect();
        let inner = vec.into_boxed_slice();
        Self { inner }
    }
}

impl IntoDart for DartArray {
    fn into_dart(self) -> DartCObject {
        let mut s = ManuallyDrop::new(self);
        let (data, len) = (s.inner.as_mut_ptr(), s.inner.len());

        let array = DartNativeArray {
            length: len as isize,
            values: data,
        };
        DartCObject {
            ty: DartCObjectType::DartArray,
            value: DartCObjectValue { as_array: array },
        }
    }
}

impl From<DartNativeArray> for DartArray {
    fn from(arr: DartNativeArray) -> Self {
        let inner = unsafe {
            let slice = std::slice::from_raw_parts_mut(arr.values, arr.length as usize);
            Box::from_raw(slice)
        };
        Self { inner }
    }
}

impl Drop for DartArray {
    fn drop(&mut self) {
        for v in self.inner.iter() {
            unsafe {
                Box::from_raw(*v);
            }
        }
    }
}

pub trait IntoDart {
    fn into_dart(self) -> DartCObject;
}

impl<T> IntoDart for T
where
    T: Into<DartCObject>,
{
    fn into_dart(self) -> DartCObject {
        self.into()
    }
}

impl IntoDart for () {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartNull,
            value: DartCObjectValue { as_bool: false },
        }
    }
}

impl<T1, T2> IntoDart for (T1, T2)
where
    T1: IntoDart,
    T2: IntoDart,
{
    fn into_dart(self) -> DartCObject {
        vec![self.0.into_dart(), self.1.into_dart()].into_dart()
    }
}

impl IntoDart for u32 {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt32,
            value: DartCObjectValue {
                as_int32: self as i32,
            },
        }
    }
}

impl IntoDart for i32 {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt32,
            value: DartCObjectValue { as_int32: self },
        }
    }
}

impl IntoDart for u64 {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt64,
            value: DartCObjectValue {
                as_int64: self as i64,
            },
        }
    }
}

impl IntoDart for i64 {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt64,
            value: DartCObjectValue { as_int64: self },
        }
    }
}

impl IntoDart for f32 {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartDouble,
            value: DartCObjectValue {
                as_double: self as f64,
            },
        }
    }
}

impl IntoDart for f64 {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartDouble,
            value: DartCObjectValue { as_double: self },
        }
    }
}

impl IntoDart for bool {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartBool,
            value: DartCObjectValue { as_bool: self },
        }
    }
}

impl IntoDart for String {
    fn into_dart(self) -> DartCObject {
        let s = CString::new(self).unwrap_or_default();
        s.into_dart()
    }
}

impl IntoDart for &'_ str {
    fn into_dart(self) -> DartCObject {
        self.to_string().into_dart()
    }
}

impl IntoDart for CString {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartString,
            value: DartCObjectValue {
                as_string: self.into_raw(),
            },
        }
    }
}

impl IntoDart for Vec<u8> {
    fn into_dart(self) -> DartCObject {
        let mut vec = ManuallyDrop::new(self);
        let data = DartNativeTypedData {
            ty: DartTypedDataType::Uint8,
            length: vec.len() as isize,
            values: vec.as_mut_ptr(),
        };
        let value = DartCObjectValue {
            as_typed_data: data,
        };
        DartCObject {
            ty: DartCObjectType::DartTypedData,
            value,
        }
    }
}

impl IntoDart for Vec<i8> {
    fn into_dart(self) -> DartCObject {
        let mut vec = ManuallyDrop::new(self);
        let data = DartNativeTypedData {
            ty: DartTypedDataType::Int8,
            length: vec.len() as isize,
            values: vec.as_mut_ptr() as *mut _,
        };
        let value = DartCObjectValue {
            as_typed_data: data,
        };
        DartCObject {
            ty: DartCObjectType::DartTypedData,
            value,
        }
    }
}

impl<T> IntoDart for Vec<T>
where
    T: IntoDart,
{
    fn into_dart(self) -> DartCObject {
        DartArray::from(self).into_dart()
    }
}

impl<T> IntoDart for Option<T>
where
    T: IntoDart,
{
    fn into_dart(self) -> DartCObject {
        match self {
            Some(v) => v.into_dart(),
            None => ().into_dart(),
        }
    }
}

impl<T, E> IntoDart for Result<T, E>
where
    T: IntoDart,
    E: ToString,
{
    fn into_dart(self) -> DartCObject {
        match self {
            Ok(v) => v.into_dart(),
            Err(e) => e.to_string().into_dart(),
        }
    }
}

#[cfg(target_pointer_width = "64")]
impl<T> IntoDart for *const T {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt64,
            value: DartCObjectValue {
                as_int64: self as _,
            },
        }
    }
}

#[cfg(target_pointer_width = "64")]
impl<T> IntoDart for *mut T {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt64,
            value: DartCObjectValue {
                as_int64: self as _,
            },
        }
    }
}

#[cfg(target_pointer_width = "32")]
impl<T> IntoDart for *const T {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt32,
            value: DartCObjectValue {
                as_int32: self as _,
            },
        }
    }
}

#[cfg(target_pointer_width = "32")]
impl<T> IntoDart for *mut T {
    fn into_dart(self) -> DartCObject {
        DartCObject {
            ty: DartCObjectType::DartInt32,
            value: DartCObjectValue {
                as_int32: self as _,
            },
        }
    }
}

/// cbindgen:ignore
pub static mut POST_COBJECT: Option<DartPostCObjectFnType> = None;
