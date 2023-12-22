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

use crate::{
    lexis::{Site, SourceCode, ToSite},
    std::*,
};

/// A one-based line number inside the source code.
///
/// In contrast to [Sites](crate::lexis::Site), Lines numeration starts with `1`. Number `1` means
/// the first line in the source code. Number `2` means the second line in the source code,
/// and so on. Number `0` is a valid value that means the first line too.
pub type Line = usize;

/// A one-based Unicode character number inside the source code line string.
///
/// In contrast to [Sites](crate::lexis::Site), Columns numeration starts with `1`. Number `1` means
/// the first UTF-8 character in the source code line. Number `2` means the second UTF-8 character
/// in the source code line, and so on. Number `0` is a valid value that means the first character
/// inside the source code line too.
pub type Column = usize;

/// A line-column index object into the source code text.
///
/// This object interprets the source code text as a table of UTF-8 characters, where the rows are
/// text lines, and the columns are UTF-8 characters inside lines.
///
/// Lines separated either by `\n`, or `\r\n`, or `\n\r` character sequences.
/// Line-break/Caret-return symbols interpretation is encoding-independent to some extent.
///
/// This object implements [ToSite](crate::lexis::ToSite) trait. Any Position value is always
/// [valid to resolve](crate::lexis::ToSite::is_valid_site), but resolution complexity is linear
/// to the entire source code text size. An API user should take into account this performance
/// characteristic in the end compilation system design. [AddAssign](::std::ops::AddAssign)(`+=`)
/// operation that incrementally moves Position into specified string symbols forward could help
/// in resolving possible performance bottlenecks when the Position object is supposed to be used
/// frequently.
///
/// Also, the companion auto-implemented trait [ToPosition](crate::lexis::ToPosition) allows turning
/// of any `ToSite` implementation back to Position. In Particular [Site](crate::lexis::Site) or
/// [SiteRef](crate::lexis::SiteRef) can be turned into Position instance.
///
/// ```rust
/// use lady_deirdre::lexis::{
///     Position, ToSite, SimpleToken, TokenBuffer, SourceCode
/// };
///
/// let mut code = TokenBuffer::<SimpleToken>::default();
///
/// code.append("First line\n");
/// code.append("Second line\n");
/// code.append("Third line\n");
///
/// assert_eq!(code.substring(Position::new(1, 1)..Position::new(1, 100)), "First line\n");
/// assert_eq!(code.substring(Position::new(2, 1)..Position::new(2, 100)), "Second line\n");
/// assert_eq!(code.substring(Position::new(3, 1)..Position::new(3, 100)), "Third line\n");
///
/// assert!(Position::new(2, 8) < Position::new(3, 6));
/// assert_eq!(code.substring(Position::new(2, 8)..Position::new(3, 6)), "line\nThird");
///
/// let site = Position::new(2, 8).to_site(&code).unwrap();
/// let mut position = site.to_position(&code).unwrap();
///
/// assert_eq!(position, Position::new(2, 8));
///
/// position += "line\nThird".chars();
///
/// assert_eq!(position, Position::new(3, 6));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// A number of the line inside the Source code.
    ///
    /// Numeration is One-based. Line `1` is the first line. Line `2` is the second line, and so on.
    /// Number `0` is a valid value that means the first line too.
    ///
    /// If the `line` number is greater than the total number of lines inside the source code, this
    /// number will be interpreted as a source code text end.
    pub line: Line,

    /// A number of the UTF-8 character inside the `line` of the Source code.
    ///
    /// Numeration is One-based. Colum `1` is the first character. Line `2` is the character, and
    /// so on. Number `0` is a valid value that means the first character too.
    ///
    /// If the `column` number is greater than the total number of characters inside this `line`,
    /// this number will be interpreted as the line string end.
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
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
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

        Some((self.column.checked_sub(1).unwrap_or_default() + span.start).min(span.end))
    }

    #[inline(always)]
    fn is_valid_site(&self, _code: &impl SourceCode) -> bool {
        true
    }
}

impl Position {
    /// A helper shortcut constructor of the Position object.
    #[inline(always)]
    pub fn new(line: Line, column: Column) -> Self {
        Self { line, column }
    }
}
