/// Converts c string to String
#[macro_export]
macro_rules! cstr_to_string {
    ($stri:expr, $ret_val:expr) => {
        match ::std::ffi::CString::from_raw($stri).to_str() {
            Ok(a) => a.to_string(),
            Err(e) => {
                ::log::error!("Failed decoding {}: {}", stringify!($stri), e);
                return $ret_val;
            }
        }
    };
}

///Matches expression, returning provided expression in case of error;
#[macro_export]
macro_rules! ok_or_ret {
    ($x:expr, $ret_val:expr) => {
        match $x {
            Ok(a) => a,
            Err(e) => {
                ::log::error!("Failed with {}: {}", stringify!($x), e);
                return $ret_val;
            }
        }
    };
}

/// if `check_expr` is `true` returns `ret_value`
#[macro_export]
macro_rules! ffi_ensure {
    ($check_expr:expr,$ret_value:expr, $message:literal) => {
        if $check_expr {
            ::log::error!("Failed with {}", $message);
            return $ret_value;
        }
    };
}

///Logs error
#[macro_export]
macro_rules! loge {
    ($expr:expr) => {
        if let Err(e) = $expr {
            ::log::error!("Error occured in {}:{}: {}", file!(), line!(), e);
        }
    };
}

///Converts `Result` to `Option` logging error
#[macro_export]
macro_rules! match_option {
    ($matched:expr) => {
        match $matched {
            Ok(a) => Some(a),
            Err(e) => {
                ::log::error!("Failed with: {}: {}", stringify!($matched), e);
                None
            }
        }
    };
}
