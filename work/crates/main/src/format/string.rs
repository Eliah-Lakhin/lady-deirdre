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
    lexis::{Chunk, Length, Token},
    std::*,
};

#[derive(Clone)]
pub struct PrintString<'a> {
    string: Cow<'a, str>,
    length: Length,
}

impl<'a> Debug for PrintString<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        Debug::fmt(&self.string, formatter)
    }
}

impl<'a> Display for PrintString<'a> {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        Display::fmt(&self.string, formatter)
    }
}

impl<'a, S: AsRef<str>> PartialEq<S> for PrintString<'a> {
    #[inline(always)]
    fn eq(&self, other: &S) -> bool {
        self.string.eq(other.as_ref())
    }
}

impl<'a, 'b> PartialEq<PrintString<'a>> for &'b str {
    #[inline(always)]
    fn eq(&self, other: &PrintString<'a>) -> bool {
        self.eq(&other.string)
    }
}

impl<'a> Eq for PrintString<'a> {}

impl<'a> Hash for PrintString<'a> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.string.hash(state)
    }
}

impl<'a> PartialOrd for PrintString<'a> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.string.partial_cmp(&other.string)
    }
}

impl<'a> Ord for PrintString<'a> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.string.cmp(&other.string)
    }
}

impl<'a> AsRef<str> for PrintString<'a> {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.string.as_ref()
    }
}

impl<'a> Borrow<str> for PrintString<'a> {
    #[inline(always)]
    fn borrow(&self) -> &str {
        self.string.borrow()
    }
}

impl<'a> Default for PrintString<'a> {
    #[inline(always)]
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a> From<&'a str> for PrintString<'a> {
    #[inline(always)]
    fn from(value: &'a str) -> Self {
        Self {
            length: value.chars().count(),
            string: Cow::from(value),
        }
    }
}

impl<'a> From<String> for PrintString<'a> {
    #[inline(always)]
    fn from(value: String) -> Self {
        Self {
            length: value.chars().count(),
            string: Cow::from(value),
        }
    }
}

impl<'a> From<Cow<'a, str>> for PrintString<'a> {
    #[inline(always)]
    fn from(value: Cow<'a, str>) -> Self {
        Self {
            length: value.chars().count(),
            string: value,
        }
    }
}

impl<'a, T: Token> From<Chunk<'a, T>> for PrintString<'a> {
    #[inline(always)]
    fn from(value: Chunk<'a, T>) -> Self {
        Self {
            length: value.length,
            string: Cow::from(value.string),
        }
    }
}

impl<'a> Extend<char> for PrintString<'a> {
    #[inline(always)]
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        self.string.to_mut().extend(iter)
    }
}

impl<'a, S: AsRef<str>> AddAssign<S> for PrintString<'a> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: S) {
        self.push_str(rhs.as_ref())
    }
}

impl<'a> PrintString<'a> {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self {
            string: Cow::Borrowed(""),
            length: 0,
        }
    }

    #[inline(always)]
    pub const fn whitespace() -> Self {
        Self {
            length: 1,
            string: Cow::Borrowed(" "),
        }
    }

    #[inline(always)]
    pub fn owned(string: impl Into<String>) -> Self {
        let string = string.into();

        Self {
            length: string.chars().count(),
            string: Cow::from(string),
        }
    }

    #[inline(always)]
    pub const fn borrowed(string: &'a str) -> Self {
        Self {
            length: length_of(string.as_bytes()),
            string: Cow::Borrowed(string),
        }
    }

    // Safety: `length` is equal to the number of Unicode characters in the `text`.
    pub const unsafe fn new_unchecked(string: Cow<'a, str>, length: Length) -> Self {
        Self { string, length }
    }

    #[inline(always)]
    pub fn reserve(&mut self, additional: usize) {
        if additional == 0 {
            return;
        }

        self.string.to_mut().reserve(additional)
    }

    #[inline(always)]
    pub fn push(&mut self, ch: char) {
        self.string.to_mut().push(ch);
        self.length += 1;
    }

    #[inline(always)]
    pub fn push_str(&mut self, string: &str) {
        if string.is_empty() {
            return;
        }

        if self.length == 0 {
            *self = Self::owned(string);
            return;
        }

        self.string.to_mut().push_str(string);
        self.length += string.chars().count();
    }

    #[inline(always)]
    pub fn append<'b: 'a>(&mut self, string: impl Into<PrintString<'b>>) {
        let string = string.into();

        if string.is_empty() {
            return;
        }

        if self.length == 0 {
            *self = string;
            return;
        }

        self.string.to_mut().push_str(string.string.as_ref());
        self.length += string.length;
    }

    #[inline(always)]
    pub fn length(&self) -> Length {
        self.length
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.string.is_empty()
    }

    #[inline(always)]
    pub fn into_string(self) -> String {
        self.string.into_owned()
    }
}

const fn length_of(bytes: &[u8]) -> Length {
    const PAT_1: u8 = 0b10000000;
    const PAT_3: u8 = 0b11100000;
    const PAT_4: u8 = 0b11110000;

    let mut index = 0;
    let mut length = 0;

    while index < bytes.len() {
        length += 1;

        let first = bytes[index];

        if first & PAT_1 == 0 {
            index += 1;
            continue;
        }

        let prefix = first & PAT_4;

        match prefix {
            PAT_4 => index += 4,
            PAT_3 => index += 3,
            _ => index += 2,
        }
    }

    length
}
