

use once_cell::sync::Lazy;



pub static RUNTIME_: Lazy<std::io::Result<tokio::runtime::Runtime>> =
    Lazy::new(tokio::runtime::Runtime::new);


#[macro_export]
macro_rules! get_runtime {
    () => {
        match crate::global::RUNTIME_.as_ref() {
            Ok(a) => {
                ::android_logger::init_once(
                    ::android_logger::Config::default()
                        .with_min_level(::log::Level::Info)
                        .with_tag("nekoton")
                        .with_filter(
                            android_logger::FilterBuilder::new()
                                .parse("ntbindings=debug,reqwest=debug")
                                .build(),
                        ),
                );
                a
            }
            Err(e) => {
                ::log::error!("Failed getting tokio runtime: {}", e);
                return crate::ExitCode::FailedToCreateRuntime;
            }
        }
    };
}
