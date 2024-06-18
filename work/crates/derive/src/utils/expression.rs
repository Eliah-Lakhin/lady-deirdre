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

use syn::parse::{Lookahead1, Parse, ParseStream, Result};

use crate::utils::system_panic;

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

impl<O: ExpressionOperator> Parse for Expression<O> {
    #[inline(always)]
    fn parse(input: ParseStream) -> Result<Self> {
        if let Some(mut head) = O::head() {
            match head.test(input, &input.lookahead1()) {
                Applicability::Mismatch => (),

                Applicability::Unary => {
                    system_panic!("Unary operator as a head.",)
                }

                Applicability::Binary => {
                    head.parse(input)?;
                    return Self::binding_parse(input, head.binding_power() - 1);
                }
            }
        }

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

                match operator.test(input, &lookahead) {
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

    fn head() -> Option<Self>;

    fn enumerate() -> Vec<Self>;

    fn binding_power(&self) -> u8;

    fn test(&self, input: ParseStream, lookahead: &Lookahead1) -> Applicability;

    fn parse(&mut self, input: ParseStream) -> Result<()>;
}

pub trait ExpressionOperand<O: ExpressionOperator> {
    fn parse(input: ParseStream) -> Result<Expression<O>>;

    fn test(input: ParseStream) -> bool;
}

pub enum Applicability {
    Mismatch,
    Unary,
    Binary,
}
