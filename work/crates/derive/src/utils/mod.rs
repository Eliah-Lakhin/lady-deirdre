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

mod automata;
mod context;
mod description;
mod deterministic;
mod dump;
mod expression;
mod facade;
mod map;
mod predictable;
mod report;
mod set;
mod transitions;

pub(crate) use report::{error, error_message, expect_some, null, system_panic};

pub use crate::utils::{
    automata::Automata,
    context::{AutomataContext, AutomataTerminal, State, Strategy},
    description::Description,
    dump::Dump,
    expression::{Applicability, Expression, ExpressionOperand, ExpressionOperator},
    facade::Facade,
    map::Map,
    predictable::PredictableCollection,
    set::{Set, SetImpl},
};

pub mod dump_kw {
    syn::custom_keyword!(output);
    syn::custom_keyword!(trivia);
    syn::custom_keyword!(meta);
    syn::custom_keyword!(dry);
    syn::custom_keyword!(decl);
    syn::custom_keyword!(dump);
}
