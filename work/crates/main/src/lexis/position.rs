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

use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    ops::AddAssign,
};

use crate::lexis::{Site, SourceCode, ToSite};

/// An index of the line in the source code text.
///
/// Line numeration starts from 1, such that 1 denotes the first line,
/// 2 denotes the second line, and so on.
///
/// Line 0 also denotes the first line.
///
/// If this number exceed the total number of lines, the value interpreted
/// as the source code text end.
pub type Line = usize;

/// An index of the character of the line in the source code text.
///
/// Column numeration starts from 1, such that 1 denotes the first char,
/// 2 denotes the second char, and so on.
///
/// Column 0 also denotes the first char.
///
/// If this number exceed the total number of characters of the line, the value
/// interpreted as the end of the line.
///
/// Note that the line delimiters (`\n` and `\r` chars) are parts of the line
/// tail.
pub type Column = usize;

/// A line-column index of the Unicode character within the source code text.
///
/// The line and the column indices are 1-based, and the Position object
/// is always [valid](ToSite::is_valid_site) index for any source code.
///
/// For details, see [Line] and [Column] specifications.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    /// A line number. This value is 1-based.
    pub line: Line,

    /// A number of the character in the line. This value is 1-based.
    pub column: Column,
}

impl Ord for Position {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.line < other.line {
            return Ordering::Less;
        }

        if self.line > other.line {
            return Ordering::Greater;
        }

        self.column.cmp(&other.column)
    }
}

impl PartialOrd for Position {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for Position {
    #[inline(always)]
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}

impl Display for Position {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_fmt(format_args!("{}:{}", self.line, self.column))
    }
}

impl<I: Iterator<Item = char>> AddAssign<I> for Position {
    #[inline]
    fn add_assign(&mut self, rhs: I) {
        for ch in rhs {
            match ch {
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                }

                _ => {
                    self.column += 1;
                }
            }
        }
    }
}

unsafe impl ToSite for Position {
    fn to_site(&self, code: &impl SourceCode) -> Option<Site> {
        let span = code.lines().line_span(self.line);

        Some(
            self.column
                .checked_sub(1)
                .unwrap_or_default()
                .checked_add(span.start)
                .unwrap_or(span.end)
                .min(span.end),
        )
    }

    #[inline(always)]
    fn is_valid_site(&self, _code: &impl SourceCode) -> bool {
        true
    }
}

impl Position {
    /// A constructor of the Position object.
    #[inline(always)]
    pub fn new(line: Line, column: Column) -> Self {
        Self { line, column }
    }
}
