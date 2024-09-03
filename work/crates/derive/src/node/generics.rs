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

use syn::{
    punctuated::Punctuated,
    spanned::Spanned,
    GenericParam,
    Generics,
    Lifetime,
    LifetimeParam,
};

pub(super) struct ParserGenerics {
    pub(super) ty: Generics,
    pub(super) func: Generics,
    pub(super) code: Lifetime,
}

impl ParserGenerics {
    pub(super) fn new(generics: Generics) -> Self {
        let code = {
            let mut candidate = String::from("'code");

            'outer: loop {
                for lifetime_def in generics.lifetimes() {
                    if candidate == lifetime_def.lifetime.ident.to_string() {
                        candidate.push('_');
                        continue 'outer;
                    }
                }

                break;
            }

            Lifetime::new(candidate.as_str(), generics.span())
        };

        let mut func = generics.clone();

        func.params.insert(
            0,
            GenericParam::Lifetime(LifetimeParam {
                attrs: Vec::new(),
                lifetime: code.clone(),
                colon_token: None,
                bounds: Punctuated::new(),
            }),
        );

        ParserGenerics {
            ty: generics,
            func,
            code,
        }
    }
}
