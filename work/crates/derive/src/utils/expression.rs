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

use proc_macro2::Span;
use syn::{
    parse::{Lookahead1, Parse, ParseStream, Result},
    spanned::Spanned,
};

#[derive(Clone)]
pub enum Expression<O: ExpressionOperator> {
    Operand(O::Operand),
    Binary(Box<Self>, O, Box<Self>),
    Unary(O, Box<Self>),
}

impl<O: ExpressionOperator> AsRef<Self> for Expression<O> {
    #[inline(always)]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<O> Default for Expression<O>
where
    O: ExpressionOperator,
    <O as ExpressionOperator>::Operand: Default,
{
    #[inline(always)]
    fn default() -> Self {
        Self::Operand(<O as ExpressionOperator>::Operand::default())
    }
}

impl<O> Spanned for Expression<O>
where
    O: ExpressionOperator,
    <O as ExpressionOperator>::Operand: Spanned,
{
    fn span(&self) -> Span {
        match self {
            Self::Operand(operand) => operand.span(),
            Self::Binary(left, _, _) => left.span(),
            Self::Unary(_, inner) => inner.span(),
        }
    }
}

impl<O: ExpressionOperator> Parse for Expression<O> {
    #[inline(always)]
    fn parse(input: ParseStream) -> Result<Self> {
        Self::binding_parse(input, 0)
    }
}

impl<O: ExpressionOperator> Expression<O> {
    fn binding_parse(input: ParseStream, right_binding_power: u8) -> Result<Self> {
        let mut left = O::Operand::parse(input)?;

        'outer: loop {
            if input.is_empty() || input.peek(Token![,]) {
                break;
            }

            let lookahead = input.lookahead1();

            for mut operator in O::enumerate() {
                let binding_power = operator.binding_power();

                match operator.peek(&lookahead) {
                    Applicability::Mismatch => (),

                    Applicability::Unary => {
                        if binding_power <= right_binding_power {
                            break 'outer;
                        }

                        operator.parse(input)?;

                        left = Expression::Unary(operator, Box::new(left));

                        continue 'outer;
                    }

                    Applicability::Binary => {
                        if binding_power <= right_binding_power {
                            break 'outer;
                        }

                        operator.parse(input)?;

                        let right = Self::binding_parse(input, binding_power - 1)?;

                        left = Expression::Binary(Box::new(left), operator, Box::new(right));

                        continue 'outer;
                    }
                }
            }

            return Err(lookahead.error());
        }

        Ok(left)
    }
}

pub trait ExpressionOperator: Sized {
    type Operand: ExpressionOperand<Self>;

    fn enumerate() -> Vec<Self>;

    fn binding_power(&self) -> u8;

    fn peek(&self, lookahead: &Lookahead1) -> Applicability;

    fn parse(&mut self, input: ParseStream) -> Result<()>;
}

pub trait ExpressionOperand<O: ExpressionOperator> {
    fn parse(input: ParseStream) -> Result<Expression<O>>;
}

pub enum Applicability {
    Mismatch,
    Unary,
    Binary,
}
