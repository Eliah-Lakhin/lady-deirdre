////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

macro_rules! ld_unreachable (
    ($message:expr) => {
        {
            #[cfg(debug_assertions)]
            {
                $crate::report::system_panic!($message);
            }

            ::std::hint::unreachable_unchecked()
        }
    };

    ($message:expr, $($args:tt)*) => {
        $crate::report::ld_unreachable!(::std::format!($message, $($args)*))
    };
);

macro_rules! ld_assert {
    ($assertion:expr, $message:expr) => {
        #[cfg(debug_assertions)]
        {
            if !$assertion {
                $crate::report::system_panic!($message);
            }
        }
    };

    ($assertion:expr, $message:expr, $($args:tt)*) => {
        #[cfg(debug_assertions)]
        {
            if !$assertion {
                $crate::report::system_panic!($message, $($args)*);
            }
        }
    };
}

macro_rules! ld_assert_eq {
    ($left:expr, $right:expr, $message:expr) => {
        $crate::report::ld_assert!($left == $right, $message);
    };

    ($left:expr, $right:expr, $message:expr, $($args:tt)*) => {
        $crate::report::ld_assert!($left == $right, $message, $($args)*);
    };
}

macro_rules! ld_assert_ne {
    ($left:expr, $right:expr, $message:expr) => {
        $crate::report::ld_assert!($left != $right, $message);
    };

    ($left:expr, $right:expr, $message:expr, $($args:tt)*) => {
        $crate::report::ld_assert!($left != $right, $message, $($args)*);
    };
}

macro_rules! system_panic (
    ($message:expr) => {{
        if !::std::thread::panicking() {
            ::std::panic!(
                "{}",
                $crate::report::error_message!($message),
            );
        }
    }};

    ($message:expr, $($args:tt)*) => {
        $crate::report::system_panic!(::std::format!($message, $($args)*))
    };
);

macro_rules! error_message (
    ($message:expr) => {
        ::std::format!(
r#" !! LADY DEIRDRE INTERNAL ERROR
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
            ::std::file!(),
            ::std::line!(),
            ::std::column!(),
        )
    };

    ($message:expr, $($args:tt)*) => {
        $crate::report::error_message!(::std::format!($message, $($args)*))
    };
);

pub(crate) use error_message;
pub(crate) use ld_assert;
pub(crate) use ld_assert_eq;
pub(crate) use ld_assert_ne;
pub(crate) use ld_unreachable;
pub(crate) use system_panic;
