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
