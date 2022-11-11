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

pub struct Facade {
    core_crate: TokenStream,
    option: TokenStream,
    vec: TokenStream,
    into: TokenStream,
    unreachable: TokenStream,
    unimplemented: TokenStream,
}

impl Facade {
    pub fn new() -> Self {
        let core_crate = quote! {
            ::lady_deirdre
        };

        #[cfg(feature = "std")]
        let option = quote! {
            ::std::option::Option
        };
        #[cfg(not(feature = "std"))]
        let option = quote! {
            ::core::option::Option
        };

        #[cfg(feature = "std")]
        let vec = quote! {
            ::std::vec::Vec
        };
        #[cfg(not(feature = "std"))]
        let vec = quote! {
            ::alloc::vec::Vec
        };

        #[cfg(feature = "std")]
        let into = quote! {
            ::std::convert::From
        };
        #[cfg(not(feature = "std"))]
        let into = quote! {
            ::core::convert::From
        };

        #[cfg(feature = "std")]
        let unreachable = quote! {
            ::std::unreachable!
        };
        #[cfg(not(feature = "std"))]
        let unreachable = quote! {
            ::core::unreachable!
        };

        #[cfg(feature = "std")]
        let unimplemented = quote! {
            ::std::unimplemented!
        };
        #[cfg(not(feature = "std"))]
        let unimplemented = quote! {
            ::core::unimplemented!
        };

        Self {
            core_crate,
            option,
            vec,
            into,
            unreachable,
            unimplemented,
        }
    }

    #[inline(always)]
    pub fn core_crate(&self) -> &TokenStream {
        &self.core_crate
    }

    #[inline(always)]
    pub fn option(&self) -> &TokenStream {
        &self.option
    }

    #[inline(always)]
    pub fn vec(&self) -> &TokenStream {
        &self.vec
    }

    #[inline(always)]
    pub fn convert(&self) -> &TokenStream {
        &self.into
    }

    #[inline(always)]
    pub fn unreachable(&self) -> &TokenStream {
        &self.unreachable
    }

    #[inline(always)]
    pub fn unimplemented(&self) -> &TokenStream {
        &self.unimplemented
    }
}
