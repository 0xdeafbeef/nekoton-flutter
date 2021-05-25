pub unsafe fn ffi_mut_cast<'a, T>(ty: *mut T) -> &'a mut T {
    &mut *ty
}

pub unsafe fn ffi_cast<'a, T>(ty: *const T) -> &'a T {
    &*ty
}
