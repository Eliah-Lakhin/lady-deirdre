/*
A part of this file is copied from
https://github.com/Geal/nom/blob/761ab0a24fccb4c560367b583b608fbae5f31647/benchmarks/benches/json.r
as is under the terms and conditions of the MIT License:

Copyright (c) 2014-2019 Geoffroy Couprie

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use criterion::black_box;
use lady_deirdre::lexis::SiteSpan;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{anychar, char, multispace0, none_of},
    combinator::{map, map_opt, map_res, value, verify},
    error::ParseError,
    multi::{fold_many0, separated_list0},
    number::complete::double,
    sequence::{delimited, preceded, separated_pair},
    IResult,
    Parser,
};

use crate::{frameworks::FrameworkConfiguration, BenchDataLayer, FrameworkCase};

#[derive(Debug, PartialEq, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Str(String),
    Num(f64),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

fn boolean(input: &str) -> IResult<&str, bool> {
    alt((value(false, tag("false")), value(true, tag("true"))))(input)
}

fn u16_hex(input: &str) -> IResult<&str, u16> {
    map_res(take(4usize), |s| u16::from_str_radix(s, 16))(input)
}

fn unicode_escape(input: &str) -> IResult<&str, char> {
    map_opt(
        alt((
            // Not a surrogate
            map(verify(u16_hex, |cp| !(0xD800..0xE000).contains(cp)), |cp| {
                cp as u32
            }),
            // See https://en.wikipedia.org/wiki/UTF-16#Code_points_from_U+010000_to_U+10FFFF for details
            map(
                verify(
                    separated_pair(u16_hex, tag("\\u"), u16_hex),
                    |(high, low)| (0xD800..0xDC00).contains(high) && (0xDC00..0xE000).contains(low),
                ),
                |(high, low)| {
                    let high_ten = (high as u32) - 0xD800;
                    let low_ten = (low as u32) - 0xDC00;
                    (high_ten << 10) + low_ten + 0x10000
                },
            ),
        )),
        // Could probably be replaced with .unwrap() or _unchecked due to the verify checks
        std::char::from_u32,
    )(input)
}

fn character(input: &str) -> IResult<&str, char> {
    let (input, c) = none_of("\"")(input)?;
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
            preceded(char('u'), unicode_escape),
        ))(input)
    } else {
        Ok((input, c))
    }
}

fn string(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        fold_many0(character, String::new, |mut string, c| {
            string.push(c);
            string
        }),
        char('"'),
    )(input)
}

fn ws<'a, O, E: ParseError<&'a str>, F: Parser<&'a str, O, E>>(f: F) -> impl Parser<&'a str, O, E> {
    delimited(multispace0, f, multispace0)
}

fn array(input: &str) -> IResult<&str, Vec<JsonValue>> {
    delimited(
        char('['),
        ws(separated_list0(ws(char(',')), json_value)),
        char(']'),
    )(input)
}

fn object(input: &str) -> IResult<&str, HashMap<String, JsonValue>> {
    map(
        delimited(
            char('{'),
            ws(separated_list0(
                ws(char(',')),
                separated_pair(string, ws(char(':')), json_value),
            )),
            char('}'),
        ),
        |key_values| key_values.into_iter().collect(),
    )(input)
}

fn json_value(input: &str) -> IResult<&str, JsonValue> {
    use JsonValue::*;

    alt((
        value(Null, tag("null")),
        map(boolean, Bool),
        map(string, Str),
        map(double, Num),
        map(array, Array),
        map(object, Object),
    ))(input)
}

fn json(input: &str) -> IResult<&str, JsonValue> {
    ws(json_value).parse(input)
}

pub struct NomCase(pub &'static str);

impl FrameworkCase for NomCase {
    fn name(&self) -> &'static str {
        self.0
    }

    fn configuration(&self, layer: &BenchDataLayer) -> FrameworkConfiguration {
        FrameworkConfiguration {
            sample_size: match layer.index == 0 {
                false => 10,
                true => 100,
            },

            many_edits: layer.index == 0,

            ..FrameworkConfiguration::default()
        }
    }

    #[inline(never)]
    fn bench_load(&self, text: &str) -> Duration {
        let start = Instant::now();
        let result = json(text).unwrap();
        let time = start.elapsed();

        black_box(result);

        time
    }

    #[inline(never)]
    fn bench_single_edit<'a>(&self, text: &'a str, span: SiteSpan, edit: &'a str) -> Duration {
        let start = Instant::now();
        let text = format!("{}{}{}", &text[0..span.start], edit, &text[span.end..]);
        let result = json(text.as_str()).unwrap();
        let time = start.elapsed();

        black_box(result);

        time
    }

    #[inline(never)]
    fn bench_sequential_edits<'a>(
        &self,
        text: &'a str,
        edits: Vec<(SiteSpan, &'a str)>,
    ) -> Duration {
        let mut text = text.to_string();

        let mut total = Duration::ZERO;

        for (span, edit) in edits {
            let start = Instant::now();
            text = format!("{}{}{}", &text[0..span.start], edit, &text[span.end..]);
            let result = json(text.as_str()).unwrap();
            let time = start.elapsed();

            total += time;

            black_box(result);
        }

        total
    }
}
