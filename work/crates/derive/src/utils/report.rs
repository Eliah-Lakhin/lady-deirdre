macro_rules! error {
    ($span:expr, $message:expr, $( $args:tt)*) => {
        syn::Error::new($span, format!($message, $($args)*))
    };
}

macro_rules! expect_some {
    ($unwrapped:expr, $message:expr, $( $args:tt)*) => {(match $unwrapped {
        Some(inner) => inner,
        None =>panic!("{}", $crate::utils::error_message!($message, $($args)*))
    })};
}

macro_rules! null {
    () => {
        $crate::utils::system_panic!("Automata with Null transition.")
    };
}

macro_rules! system_panic {
    ($message:expr) => {
        panic!("{}", $crate::utils::error_message!($message))
    };
    ($message:expr, $($args:tt)*) => {
        panic!("{}", $crate::utils::error_message!($message, $($args)*))
    };
}

macro_rules! error_message (
    ($message:expr) => {
        format!(
r#"
 !! LADY DEIRDRE INTERNAL ERROR
 !!
 !! This is a bug.
 !! If you see this message, please open an Issue: https://github.com/Eliah-Lakhin/lady-deirdre/issues
 !!
 !! Message: {}
 !! File: {}
 !! Line: {}
 !! Column: {}
"#,
            $message,
            file!(),
            line!(),
            column!(),
        )
    };

    ($message:expr, $($args:tt)*) => {
        $crate::utils::error_message!(format!($message, $($args)*))
    };
);

pub(crate) use error;
pub(crate) use error_message;
pub(crate) use expect_some;
pub(crate) use null;
pub(crate) use system_panic;
