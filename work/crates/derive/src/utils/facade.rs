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

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::string::String)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::alloc::string::String)
        }
    }

    #[inline(always)]
    fn face_vec(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::vec::Vec)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::alloc::vec::Vec)
        }
    }

    #[inline(always)]
    fn face_option(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::option::Option)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::core::option::Option)
        }
    }

    #[inline(always)]
    fn face_result(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::result::Result)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::core::result::Result)
        }
    }

    #[inline(always)]
    fn face_from(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::convert::From)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::core::convert::From)
        }
    }

    #[inline(always)]
    fn face_default(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::default::Default)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::core::default::Default)
        }
    }

    #[inline(always)]
    fn face_unimplemented(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::unimplemented!)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::core::unimplemented!)
        }
    }

    #[inline(always)]
    fn face_unreachable(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::unreachable!)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::core::unreachable!)
        }
    }

    #[inline(always)]
    fn face_panic(&self) -> TokenStream {
        let span = self.span();

        #[cfg(feature = "std")]
        {
            quote_spanned!(span=> ::std::panic!)
        }

        #[cfg(not(feature = "std"))]
        {
            quote_spanned!(span=> ::core::panic!)
        }
    }
}

impl<S: Spanned> Facade for S {}
