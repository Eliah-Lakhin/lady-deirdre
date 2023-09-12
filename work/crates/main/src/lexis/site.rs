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
    arena::{Id, Identifiable},
    format::{Priority, SnippetFormatter},
    lexis::{Position, SourceCode, TokenCursor, TokenRef},
    report::debug_unreachable,
    std::*,
    syntax::PolyRef,
};

/// A number of Unicode characters in the text.
pub type Length = usize;

/// A number of Unicode characters behind specified character in the source code text.
pub type Site = usize;

/// A number of bytes behind specified Unicode character in the source code text.
pub type ByteIndex = usize;

unsafe impl ToSite for Site {
    #[inline(always)]
    fn to_site(&self, code: &impl SourceCode) -> Option<Site> {
        Some(code.length().min(*self))
    }

    #[inline(always)]
    fn is_valid_site(&self, _code: &impl SourceCode) -> bool {
        true
    }
}

/// A weak reference of the [Site] inside the source code text.
///
/// This object "pins" particular Site inside the source code text, and this "pin" can survive write
/// operations in the text happening aside of this "pin"(before or after referred Site) resolving
/// to relevant pinned Site after the source code mutations.
///
/// An API user is encouraged to use SiteRef to fix particular bounds of the source code snippets
/// for later use. For example, one can use a [Range](::std::ops::Range) of the
/// SiteRefs(a [SiteRefSpan](crate::lexis::SiteRefSpan) object) to refer particular bounds of the
/// Syntax or Semantic error inside the code.
///
/// In practice SiteRef could refer Tokens start and end bounds only.
///
/// An API user constructs this object either from the
/// [`TokenRef::site_ref`](crate::lexis::TokenRef::site_ref) to refer Token's start Site or from the
/// [`SourceCode::end_site_ref`](crate::lexis::SourceCode::end_site_ref) to refer source code's end
/// Site.
///
/// SiteRef implements [ToSite] trait, and as such can be used as an index into the source code.
///
/// SiteRef is a cheap to [Copy] and cheap to dereference object.
///
/// ```rust
/// use lady_deirdre::{
///     Document,
///     lexis::{SimpleToken, SourceCode, TokenCursor, ToSite},
///     syntax::NoSyntax,
/// };
///
/// let mut doc = Document::<NoSyntax<SimpleToken>>::from("foo bar baz");
///
/// // Obtaining the beginning Site weak reference to the third token("bar").
/// let site_ref = doc.cursor(..).site_ref(2);
///
/// // "bar" starts on the fifth character.
/// assert_eq!(site_ref.to_site(&doc).unwrap(), 4);
///
/// // Write something in the beginning of the text.
/// doc.write(0..0, "123");
/// assert_eq!(doc.substring(..), "123foo bar baz");
///
/// // From now on "bar" starts on the 8th character.
/// assert_eq!(site_ref.to_site(&doc).unwrap(), 7);
///
/// // But if we erase the entire source code, "site_ref" turns to invalid reference.
/// doc.write(.., "123456");
/// assert_eq!(doc.substring(..), "123456");
///
/// assert!(!site_ref.is_valid_site(&doc));
/// assert_eq!(site_ref.to_site(&doc), None);
/// ```
///
/// For details on the Weak references framework design see [Arena](crate::arena) module
/// documentation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SiteRef(SiteRefInner);

impl Debug for SiteRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        match &self.0 {
            SiteRefInner::CodeEnd(id) => formatter.write_fmt(format_args!("SiteRef({:?})", id)),
            SiteRefInner::ChunkStart(reference) => match reference.is_nil() {
                false => formatter.write_fmt(format_args!("SiteRef({:?})", reference.id())),
                true => formatter.write_str("SiteRef(Nil)"),
            },
        }
    }
}

impl Identifiable for SiteRef {
    #[inline(always)]
    fn id(&self) -> Id {
        match &self.0 {
            SiteRefInner::ChunkStart(reference) => reference.id(),
            SiteRefInner::CodeEnd(code_id) => *code_id,
        }
    }
}

unsafe impl ToSite for SiteRef {
    #[inline(always)]
    fn to_site(&self, code: &impl SourceCode) -> Option<Site> {
        match &self.0 {
            SiteRefInner::ChunkStart(token_ref) => code.get_site(&token_ref.chunk_entry),

            SiteRefInner::CodeEnd(id) => match id == &code.id() {
                false => None,
                true => Some(code.length()),
            },
        }
    }

    #[inline(always)]
    fn is_valid_site(&self, code: &impl SourceCode) -> bool {
        match &self.0 {
            SiteRefInner::ChunkStart(reference) => reference.is_valid_ref(code),
            SiteRefInner::CodeEnd(id) => id == &code.id(),
        }
    }
}

impl SiteRef {
    /// Returns an invalid instance of the SiteRef.
    ///
    /// This instance never resolves to a valid [Site](crate::lexis::Site).
    #[inline(always)]
    pub const fn nil() -> Self {
        Self(SiteRefInner::ChunkStart(TokenRef::nil()))
    }

    #[inline(always)]
    pub(crate) const fn new_code_end(code_id: Id) -> Self {
        Self(SiteRefInner::CodeEnd(code_id))
    }

    #[inline(always)]
    pub(super) const fn new_chunk_start(reference: TokenRef) -> Self {
        Self(SiteRefInner::ChunkStart(reference))
    }

    /// Returns `true` if this instance will never resolve to a valid [Site](crate::lexis::Site).
    ///
    /// It is guaranteed that `SiteRef::nil().is_nil()` is always `true`, but in general if
    /// this function returns `false` it is not guaranteed that provided instance is a valid
    /// reference.
    ///
    /// To determine reference validity per specified [SourceCode](crate::lexis::SourceCode)
    /// instance use [is_valid_site](crate::lexis::ToSite::is_valid_site) function instead.
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        match &self.0 {
            SiteRefInner::ChunkStart(reference) => reference.is_nil(),
            SiteRefInner::CodeEnd(_) => false,
        }
    }

    #[inline(always)]
    pub(crate) const fn inner(&self) -> &SiteRefInner {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SiteRefInner {
    ChunkStart(TokenRef),
    CodeEnd(Id),
}

/// An interface of the source code character index.
///
/// The underlying object may be a valid or invalid index for particular
/// [SourceCode](crate::lexis::SourceCode) instance. If the object considered to be not valid,
/// [is_valid_site](ToSite::is_valid_site) function returns `false`, and
/// [to_site](ToSite::to_site), [to_byte_index](ToSite::to_byte_index) functions return [None].
/// Otherwise "is_valid_site" function returns `true` and other two functions return meaningful
/// [Some] values.
///
/// It is up to implementation to decide whether particular instance considered to be valid or not.
///
/// The most trivial implementation of this trait is a [Site] type(a UTF-8 character absolute
/// offset). Sites are always valid indices. For the sake of simplicity ToSite implementation of
/// the Site type always clamps Site's value to the source code character
/// [length](crate::lexis::SourceCode::length).
///
/// Another two implementations provided for an API user out of the box are
/// [Position](crate::lexis::Position) and [SiteRef](crate::lexis::SiteRef). Positions are always
/// valid values, and SiteRefs could be invalid if referred site does not belong to specified `code`
/// reference, or if the SiteRef obsolete.
///
/// **Safety:**
///   - If [to_site](ToSite::to_site) function returns [Some] value, this value is always within
///     the `0..=SourceCode::length(code)` range.
///   - If [to_byte_index](ToSite::to_byte_index) function returns Some value, this value is
///     Unicode-valid character byte offset within the `code` underlying text.
pub unsafe trait ToSite {
    /// Resolves index object into a valid [Site](crate::lexis::Site) index for specified `code`
    /// instance.
    ///
    /// This function returns [Some] value if and only if
    /// [is_valid_site](crate::lexis::ToSite::is_valid_site) function returns `true`.
    ///
    /// Returned Some value is always within the `0..=SourceCode::length(code)` range.
    fn to_site(&self, code: &impl SourceCode) -> Option<Site>;

    fn to_position(&self, code: &impl SourceCode) -> Option<Position> {
        let site = self.to_site(code)?;

        if site == 0 {
            return Some(Position::default());
        }

        let mut line = 1;
        let mut column = 1;
        let mut cursor = 0;
        let mut chars = code.chars(..);

        loop {
            let ch = match chars.next() {
                None => break,
                Some(ch) => ch,
            };

            cursor += 1;

            match ch {
                '\n' => {
                    line += 1;
                    column = 1;
                }

                _ => {
                    column += 1;
                }
            }

            if cursor >= site {
                break;
            }
        }

        Some(Position { line, column })
    }

    /// Resolves index object into a valid Unicode [byte index](crate::lexis::ByteIndex) for
    /// specified `code` instance text.
    ///
    /// This function returns [Some] value if and only if
    /// [is_valid_site](crate::lexis::ToSite::is_valid_site) function returns `true`.
    ///
    /// Returned Some value is always Unicode-valid byte offset into the `code` underlying text.
    ///
    /// **Safety:**
    ///   - The default implementation of this function is safe as long as
    ///     [to_site](ToSite::to_site) function follows trait's general safety requirements.
    fn to_byte_index(&self, code: &impl SourceCode) -> Option<ByteIndex> {
        let mut site = match self.to_site(code) {
            None => return None,
            Some(site) => site,
        };

        if site == 0 {
            return Some(0);
        }

        let mut cursor = code.cursor(..);
        let mut byte_index = 0;
        let mut token_index = 0;

        loop {
            let length = match cursor.length(token_index) {
                None => break,
                Some(length) => length,
            };

            let string = match cursor.string(token_index) {
                None => break,
                Some(string) => string,
            };

            if site > length {
                site -= length;
                byte_index += string.len();
                token_index += 1;
                continue;
            }

            if site == 0 {
                break;
            }

            for character in string.chars() {
                site -= 1;
                byte_index += character.len_utf8();

                if site == 0 {
                    break;
                }
            }

            break;
        }

        Some(byte_index)
    }

    /// Returns `true` if this index object could be resolved to valid
    /// [Site](crate::lexis::Site) and [ByteIndex](crate::lexis::ByteIndex) values for specified
    /// `code` instance.
    fn is_valid_site(&self, code: &impl SourceCode) -> bool;

    #[inline(always)]
    fn display<'a, Code: SourceCode>(&self, code: &'a Code) -> DisplaySite<'a, Code> {
        DisplaySite {
            code,
            site: self.to_site(code),
        }
    }
}

pub struct DisplaySite<'a, Code: SourceCode> {
    code: &'a Code,
    site: Option<Site>,
}

impl<'a, Code> Debug for DisplaySite<'a, Code>
where
    Code: SourceCode,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        Display::fmt(self, formatter)
    }
}

impl<'a, Code> Display for DisplaySite<'a, Code>
where
    Code: SourceCode,
{
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        let site = match &self.site {
            None => return formatter.write_str("?"),
            Some(site) => *site,
        };

        let position = match site.to_position(self.code) {
            Some(span) => span,

            // Safety: Sites are always valid to resolve.
            None => unsafe { debug_unreachable!("Invalid site.") },
        };

        if !formatter.alternate() {
            return formatter.write_fmt(format_args!("{}", position));
        }

        formatter
            .snippet(self.code)
            .set_caption(format!("Unit({})", self.code.id()))
            .set_summary(format!("Site: {site}\nPosition: {position}"))
            .annotate(site..site, Priority::Default, "")
            .finish()
    }
}
