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
