////////////////////////////////////////////////////////////////////////////////
// This file is part of "Lady Deirdre", a compiler front-end foundation       //
// technology.                                                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, or contribute to this work, you must agree to    //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md           //
//                                                                            //
// The agreement grants a Basic Commercial License, allowing you to use       //
// this work in non-commercial and limited commercial products with a total   //
// gross revenue cap. To remove this commercial limit for one of your         //
// products, you must acquire a Full Commercial License.                      //
//                                                                            //
// If you contribute to the source code, documentation, or related materials, //
// you must grant me an exclusive license to these contributions.             //
// Contributions are governed by the "Contributions" section of the General   //
// License Agreement.                                                         //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted       //
// under the General License Agreement.                                       //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is", without any warranties, express or implied, //
// except where such disclaimers are legally invalid.                         //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

use proc_macro2::TokenStream;
use syn::spanned::Spanned;

pub trait Facade: Spanned {
    #[inline(always)]
    fn face_core(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::lady_deirdre)
    }

    #[inline(always)]
    fn face_string(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::string::String)
    }

    #[inline(always)]
    fn face_vec(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::vec::Vec)
    }

    #[inline(always)]
    fn face_option(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::option::Option)
    }

    #[inline(always)]
    fn face_result(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::result::Result)
    }

    #[inline(always)]
    fn face_from(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::convert::From)
    }

    #[inline(always)]
    fn face_default(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::default::Default)
    }

    #[inline(always)]
    fn face_unimplemented(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::unimplemented!)
    }

    #[inline(always)]
    fn face_unreachable(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::unreachable!)
    }

    #[inline(always)]
    fn face_panic(&self) -> TokenStream {
        let span = self.span();

        quote_spanned!(span=> ::std::panic!)
    }
}

impl<S: Spanned> Facade for S {}
