macro_rules! debug_panic {
    ($message:expr) => {
        panic!(
            "Derive macro internal error. This is a bug.\nIf you see this \
            message, please report an Issue: \
            https://github.com/Eliah-Lakhin/lady-deirdre/issues\n\n\
            Message: {}\n\
            File: {}\n\
            Line: {}\n\
            Column: {}",
            $message,
            ::std::file!(),
            ::std::line!(),
            ::std::column!(),
        )
    };
}

pub(crate) use debug_panic;
