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

use syn::parse::{Parse, ParseStream};

use crate::utils::Strategy;

#[derive(Clone, Copy, Debug)]
pub(super) enum Opt {
    Flat,
    Deep,
}

impl Default for Opt {
    #[inline(always)]
    fn default() -> Self {
        match cfg!(debug_assertions) {
            true => Self::Flat,
            false => Self::Deep,
        }
    }
}

impl Parse for Opt {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(opt_kw::flat) {
            let _ = input.parse::<opt_kw::flat>()?;
            return Ok(Self::Flat);
        }

        if lookahead.peek(opt_kw::deep) {
            let _ = input.parse::<opt_kw::deep>()?;
            return Ok(Self::Deep);
        }

        return Err(lookahead.error());
    }
}

impl Opt {
    #[inline(always)]
    pub(super) fn into_strategy(self) -> Strategy {
        match self {
            Self::Flat => Strategy::DETERMINIZE,
            Self::Deep => Strategy::CANONICALIZE,
        }
    }
}

mod opt_kw {
    syn::custom_keyword!(flat);
    syn::custom_keyword!(deep);
}
