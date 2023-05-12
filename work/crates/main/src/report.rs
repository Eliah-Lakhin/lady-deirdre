////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" Work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This Work is a proprietary software with source available code.            //
//                                                                            //
// To copy, use, distribute, and contribute into this Work you must agree to  //
// the terms of the End User License Agreement:                               //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The Agreement let you use this Work in commercial and non-commercial       //
// purposes. Commercial use of the Work is free of charge to start,           //
// but the Agreement obligates you to pay me royalties                        //
// under certain conditions.                                                  //
//                                                                            //
// If you want to contribute into the source code of this Work,               //
// the Agreement obligates you to assign me all exclusive rights to           //
// the Derivative Work or contribution made by you                            //
// (this includes GitHub forks and pull requests to my repository).           //
//                                                                            //
// The Agreement does not limit rights of the third party software developers //
// as long as the third party software uses public API of this Work only,     //
// and the third party software does not incorporate or distribute            //
// this Work directly.                                                        //
//                                                                            //
// AS FAR AS THE LAW ALLOWS, THIS SOFTWARE COMES AS IS, WITHOUT ANY WARRANTY  //
// OR CONDITION, AND I WILL NOT BE LIABLE TO ANYONE FOR ANY DAMAGES           //
// RELATED TO THIS SOFTWARE, UNDER ANY KIND OF LEGAL CLAIM.                   //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this Work.                                                      //
//                                                                            //
// Copyright (c) 2022 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use crate::std::String;

macro_rules! debug_unreachable (
    ($message:expr) => {
        {
            #[cfg(debug_assertions)]
            {
                $crate::report::system_panic!($message);
            }

            $crate::std::unreachable_unchecked()
        }
    };

    ($message:expr, $($args:tt)*) => {
        $crate::report::debug_unreachable!($crate::std::format!($message, $($args)*))
    };
);

macro_rules! system_panic (
    ($message:expr) => {{
        #[cfg(feature = "std")]
        {
            if !$crate::std::panicking() {
                $crate::std::panic!(
                    "{}",
                    $crate::report::error_message!($message),
                );
            }
        }

        #[cfg(not(feature = "std"))]
        {
            $crate::report::panic_once(
                $crate::report::error_message!($message),
            );
        }
    }};

    ($message:expr, $($args:tt)*) => {
        $crate::report::system_panic!($crate::std::format!($message, $($args)*))
    };
);

macro_rules! error_message (
    ($message:expr) => {
        $crate::std::format!(
r#" !! LADY DEIRDRE INTERNAL ERROR
 !!
 !! This is bug.
 !! If you see this message, please open an Issue: https://github.com/Eliah-Lakhin/lady-deirdre/issues
 !!
 !! Message: {}
 !! File: {}
 !! Line: {}
 !! Column: {}
"#,
            $message,
            $crate::std::file!(),
            $crate::std::line!(),
            $crate::std::column!(),
        )
    };

    ($message:expr, $($args:tt)*) => {
        $crate::report::error_message!($crate::std::format!($message, $($args)*))
    };
);

#[cfg(not(feature = "std"))]
pub(crate) fn panic_once(message: String) {
    use crate::std::*;

    static FLAG: AtomicUsize = AtomicUsize::new(0);

    if FLAG.fetch_add(1, AtomicOrdering::SeqCst) == 1 {
        return;
    }

    panic!("{}", message);
}

pub(crate) use debug_unreachable;
pub(crate) use error_message;
pub(crate) use system_panic;
