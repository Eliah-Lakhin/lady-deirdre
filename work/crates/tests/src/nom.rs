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

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of Geoffroy Couprie's and the authors' "nom" work.       //
//                                                                                                             //
// The original work by Geoffroy Couprie and the authors is available here:                                    //
// https://github.com/rust-bakery/nom/blob/f87d397231830c99cc633d47d1bc855736fa83a0/benchmarks/benches/json.rs //
//                                                                                                             //
// Geoffroy Couprie and the authors provided their work under the following terms:                             //
//                                                                                                             //
//   Copyright (c) 2014-2019 Geoffroy Couprie                                                                  //
//                                                                                                             //
//   Permission is hereby granted, free of charge, to any person obtaining                                     //
//   a copy of this software and associated documentation files (the                                           //
//   "Software"), to deal in the Software without restriction, including                                       //
//   without limitation the rights to use, copy, modify, merge, publish,                                       //
//   distribute, sublicense, and/or sell copies of the Software, and to                                        //
//   permit persons to whom the Software is furnished to do so, subject to                                     //
//   the following conditions:                                                                                 //
//                                                                                                             //
//   The above copyright notice and this permission notice shall be                                            //
//   included in all copies or substantial portions of the Software.                                           //
//                                                                                                             //
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,                                           //
//   EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF                                        //
//   MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND                                                     //
//   NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE                                    //
//   LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION                                    //
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION                                     //
//   WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.                                           //
//                                                                                                             //
// Kindly be advised that the terms governing the distribution of my work are                                  //
// distinct from those pertaining to the original "nom" work.                                                  //
/////////////////////////////////////////////////////////////////////////////////////////////////////////////////

use std::{collections::HashMap, marker::PhantomData, num::ParseIntError};

use criterion::black_box;
use nom::{
    branch::alt,
    bytes::{tag, take},
    character::{anychar, char, multispace0, none_of},
    combinator::{map, map_opt, map_res, value, verify},
    error::{Error, FromExternalError, ParseError},
    multi::{fold, separated_list0},
    number::double,
    sequence::{delimited, preceded, separated_pair},
    Complete,
    Emit,
    Mode,
    OutputM,
    Parser,
};

pub fn nom_parse(text: &str) {
    let result = json::<Error<&str>>()
        .process::<OutputM<Emit, Emit, Complete>>(text)
        .unwrap();

    black_box(result);
}

#[derive(Debug, PartialEq, Clone)]
enum JsonValue {
    Null,
    Bool(bool),
    Str(String),
    Num(f64),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

fn boolean<'a, E: ParseError<&'a str>>() -> impl Parser<&'a str, Output = bool, Error = E> {
    alt((value(false, tag("false")), value(true, tag("true"))))
}

fn u16_hex<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
) -> impl Parser<&'a str, Output = u16, Error = E> {
    map_res(take(4usize), |s| u16::from_str_radix(s, 16))
}

fn unicode_escape<'a, E: ParseError<&'a str> + FromExternalError<&'a str, ParseIntError>>(
) -> impl Parser<&'a str, Output = char, Error = E> {
    map_opt(
        alt((
            map(
                verify(u16_hex(), |cp| !(0xD800..0xE000).contains(cp)),
                |cp| cp as u32,
            ),
            map(
                verify(
                    separated_pair(u16_hex(), tag("\\u"), u16_hex()),
                    |(high, low)| (0xD800..0xDC00).contains(high) && (0xDC00..0xE000).contains(low),
                ),
                |(high, low)| {
                    let high_ten = (high as u32) - 0xD800;
                    let low_ten = (low as u32) - 0xDC00;
                    (high_ten << 10) + low_ten + 0x10000
                },
            ),
        )),
        std::char::from_u32,
    )
}

fn character<
    'a,
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = char, Error = E> {
    Character { e: PhantomData }
}

struct Character<E> {
    e: PhantomData<E>,
}

impl<'a, E> Parser<&'a str> for Character<E>
where
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
{
    type Output = char;

    type Error = E;

    fn process<OM: nom::OutputMode>(
        &mut self,
        input: &'a str,
    ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
        let (input, c): (&str, char) =
            none_of("\"").process::<OutputM<Emit, OM::Error, OM::Incomplete>>(input)?;
        if c == '\\' {
            alt((
                map_res(anychar, |c| {
                    Ok(match c {
                        '"' | '\\' | '/' => c,
                        'b' => '\x08',
                        'f' => '\x0C',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        _ => return Err(()),
                    })
                }),
                preceded(char('u'), unicode_escape()),
            ))
            .process::<OM>(input)
        } else {
            Ok((input, OM::Output::bind(|| c)))
        }
    }
}

fn string<
    'a,
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = String, Error = E> {
    delimited(
        char('"'),
        fold(0.., character(), String::new, |mut string, c| {
            string.push(c);
            string
        }),
        char('"'),
    )
}

fn ws<
    'a,
    O,
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
    F: Parser<&'a str, Output = O, Error = E>,
>(
    f: F,
) -> impl Parser<&'a str, Output = O, Error = E> {
    delimited(multispace0(), f, multispace0())
}

fn array<
    'a,
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = Vec<JsonValue>, Error = E> {
    delimited(
        char('['),
        ws(separated_list0(ws(char(',')), json_value())),
        char(']'),
    )
}

fn object<
    'a,
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = HashMap<String, JsonValue>, Error = E> {
    map(
        delimited(
            char('{'),
            ws(separated_list0(
                ws(char(',')),
                separated_pair(string(), ws(char(':')), json_value()),
            )),
            char('}'),
        ),
        |key_values| key_values.into_iter().collect(),
    )
}

fn json_value<
    'a,
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
>() -> JsonParser<E> {
    JsonParser { e: PhantomData }
}

struct JsonParser<E> {
    e: PhantomData<E>,
}

impl<'a, E> Parser<&'a str> for JsonParser<E>
where
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
{
    type Output = JsonValue;
    type Error = E;

    fn process<OM: nom::OutputMode>(
        &mut self,
        input: &'a str,
    ) -> nom::PResult<OM, &'a str, Self::Output, Self::Error> {
        use JsonValue::*;

        alt((
            value(Null, tag("null")),
            map(boolean(), Bool),
            map(string(), Str),
            map(double(), Num),
            map(array(), Array),
            map(object(), Object),
        ))
        .process::<OM>(input)
    }
}

fn json<
    'a,
    E: ParseError<&'a str>
        + FromExternalError<&'a str, ParseIntError>
        + FromExternalError<&'a str, ()>,
>() -> impl Parser<&'a str, Output = JsonValue, Error = E> {
    ws(json_value())
}
