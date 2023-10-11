// TODO rename this file to test_utils to be aligned with the other test utils
// file
/// Check 2 errors are the same.
/// https://stackoverflow.com/a/65618681
macro_rules! assert_err {
    ($expression:expr, $($pattern:tt)+) => {
        match $expression {
            $($pattern)+ => (),
            ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
        }
    }
}
pub(crate) use assert_err;

/// Same as [assert_err] but without needing debug
/// https://stackoverflow.com/a/65618681
macro_rules! assert_err_simple {
        ($expression:expr, $($pattern:tt)+) => {
            match $expression {
                $($pattern)+ => (),
                _ => panic!("expected a specific error but did not get it"),
            }
        }
    }
pub(crate) use assert_err_simple;

pub fn init_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace"))
        .is_test(true)
        .try_init();
}
