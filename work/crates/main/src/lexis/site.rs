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

use std::fmt::{Debug, Display, Formatter};

use crate::{
    arena::{Id, Identifiable},
    format::{AnnotationPriority, SnippetFormatter},
    lexis::{Position, SourceCode, TokenCursor, TokenRef, NIL_TOKEN_REF},
    report::ld_unreachable,
    syntax::PolyRef,
};

/// The number of Unicode characters in the source code text fragment.
pub type Length = usize;

/// An absolute index of the Unicode character within the source code text.
///
/// The index is in Unicode units, not in UTF-8 bytes. In the `Буква Щ` text,
/// site 2 refers to the Cyrillic character `к`.
///
/// This index is zero-based, such that index 0 denotes the first char.
///
/// Usually, this type denotes the absolute index from the beginning of
/// the [SourceCode] text content.
pub type Site = usize;

/// An index of the byte within the UTF-8 encoding of the unicode text.
pub type ByteIndex = usize;

/// A [SiteRef] index object that does not point to any character within any
/// source code text.
///
/// The value of this static equals to the [SiteRef::nil] value.
pub static NIL_SITE_REF: SiteRef = SiteRef::nil();

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

/// A relative index of the Unicode character within the source code text.
///
/// In contrast to the [Site] index, which denotes character's absolute offset
/// from the beginning of the [source code](SourceCode), the SiteRef is relative
/// to the tokens surrounding this index.
///
/// Whenever the end user edits the compilation unit source code (e.g., via
/// the [Document::write](crate::units::Document::write)) function), this index
/// object is automatically adjusted to the changes.
///
/// In other words, the SiteRef "pins" particular site of the source code.
///
/// For example, if the SiteRef points to the `f` character in the `bar foo baz`
/// string, corresponding to the Site 4, and the the user changes the `bar`
/// substring (3 chars) of this text to `bar2` (4 chars), the SiteRef will
/// be adjusted automatically to this edit, still pointing to the `f` character
/// and resolving to the Site 5 (because this character has been shifted to the
/// right by one char after the edit).
///
/// However, if the user rewrites a substring that would touch the pointed
/// character, the SiteRef object would become invalid.
///
/// The SiteRef can either point to the start [site](TokenRef::site) of
/// the source code token, or to the [end](SourceCode::length) site of
/// the source code text.
///
/// When the SiteRef points to the end of the source code, this index object
/// cannot become obsolete (it will always be valid for this source code
/// instance).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SiteRef(SiteRefInner);

impl Debug for SiteRef {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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

impl Default for SiteRef {
    #[inline(always)]
    fn default() -> Self {
        Self::nil()
    }
}

unsafe impl ToSite for SiteRef {
    #[inline(always)]
    fn to_site(&self, code: &impl SourceCode) -> Option<Site> {
        match &self.0 {
            SiteRefInner::ChunkStart(token_ref) => code.get_site(&token_ref.entry),

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
    /// Returns a SiteRef that intentionally does not point to any character
    /// within any source code text.
    ///
    /// If you need just a static reference to the nil SiteRef, use
    /// the predefined [NIL_SITE_REF] static.
    #[inline(always)]
    pub const fn nil() -> Self {
        Self(SiteRefInner::ChunkStart(TokenRef::nil()))
    }

    /// Creates a SiteRef that points to the [start site](TokenRef::site) of
    /// the token.
    ///
    /// The returning object would be [valid](ToSite::is_valid_site) if and
    /// only if the [TokenRef::is_valid_ref] returns true.
    #[inline(always)]
    pub const fn start_of(token_ref: TokenRef) -> Self {
        Self(SiteRefInner::ChunkStart(token_ref))
    }

    /// Creates a SiteRef that points to the [end site](SourceCode::length)
    /// of the source code.
    ///
    /// The `code_id` parameter is an identifier of the source code. You can
    /// get this value from the compilation unit using
    /// the [id](Identifiable::id) function.
    #[inline(always)]
    pub const fn end_of(code_id: Id) -> Self {
        Self(SiteRefInner::CodeEnd(code_id))
    }

    /// Returns a [TokenRef] of the token if this SiteRef points to the start
    /// site of a token. Otherwise, returns a [nil](TokenRef::nil) TokenRef.
    #[inline(always)]
    pub fn token_ref(&self) -> &TokenRef {
        match &self.0 {
            SiteRefInner::ChunkStart(token_ref) => token_ref,
            SiteRefInner::CodeEnd(_) => &NIL_TOKEN_REF,
        }
    }

    /// Returns a SiteRef that points to the [start site](TokenRef::site) of
    /// the token preceding the Site to which this SiteRef points to.
    ///
    /// Returns a [nil](Self::nil) SiteRef if this SiteRef points to
    /// the beginning of the source code or if the SiteRef is not
    /// [valid](ToSite::is_valid_site) for the specified `code`.
    pub fn prev(&self, code: &impl SourceCode) -> Self {
        let site = match self.to_site(code) {
            Some(site) => site,
            None => return Self::nil(),
        };

        code.cursor(site..site).site_ref(0)
    }

    /// Returns a SiteRef that points to the [start site](TokenRef::site) of
    /// the token following after the Site to which this SiteRef points to, or
    /// points to end of the code if there are no tokens in front of this
    /// SiteRef.
    ///
    /// Returns [nil](Self::nil) if the SiteRef is not
    /// [valid](ToSite::is_valid_site) for the specified `code`.
    pub fn next(&self, code: &impl SourceCode) -> Self {
        let site = match self.to_site(code) {
            Some(site) => site,
            None => return Self::nil(),
        };

        code.cursor(site..).site_ref(2)
    }

    /// Returns true if this SiteRef has been created as [nil](SiteRef::nil),
    /// or if the SiteRef has been created from the [nil](TokenRef::nil)
    /// TokenRef.
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        match &self.0 {
            SiteRefInner::ChunkStart(reference) => reference.is_nil(),
            SiteRefInner::CodeEnd(_) => false,
        }
    }

    /// Returns true if this SiteRef points to the end of the source code text.
    #[inline(always)]
    pub fn is_code_end(&self) -> bool {
        match &self.0 {
            SiteRefInner::CodeEnd(_) => true,
            _ => false,
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

/// An object that addresses a character of the source code text.
///
/// In Lady Deirdre, a minimal unit of text indexing is a Unicode character
/// ([Site]).
///
/// Addressing code characters just in terms of the absolute indices of Unicode
/// characters would be inconvenient.
///
/// The ToSite trait is a generic interface that provides conversion between
/// custom types of indices and the Site.
///
/// In particular, Lady Deirdre provides the following custom index types that
/// implement the ToSite trait:
///
///  - The [Site] itself.
///  - The [Position], which is a line-column index.
///  - The [SiteRef], which denotes a bound of the [TokenRef].
///
/// You are encouraged to provide your own implementations of the [ToSite] on
/// custom site types depending on the needs.
///
/// For convenience, the ToSite implementation of any [Site] value
/// (any Unicode numeric index within the source code text) is considered valid.
///
/// If the Site value exceeds the source code text [length](SourceCode::length),
/// the [ToSite::to_site] function clamps this value.
///
/// **Safety**
///
/// The implementor of the trait guarantees the following:
///
///  1. If the [ToSite::to_site] function returns Some site, this value is
///     less than or equal to the `code`'s [length](SourceCode::length).
///
///  2. If the [ToSite::to_byte_index] function returns Some value, this number
///     represent valid index into the UTF-8 code point start byte of the
///     `code`'s text content encoding.
///
///  3. The [ToSite::to_site] and [ToSite::to_position] functions return Some
///     value if and only if the [ToSite::is_valid_site] returns true for
///     the same source code.
pub unsafe trait ToSite {
    /// Returns a [Site] representation of this index object.
    ///
    /// The `code` parameter specifies a source code to which this index object
    /// belongs.
    ///
    /// The returning Site will not exceed the [SourceCode::length] value.
    ///
    /// Returns None, if the index object is not [valid](Self::is_valid_site).
    fn to_site(&self, code: &impl SourceCode) -> Option<Site>;

    /// Returns a [line-column](PositionSpan) representation of this index
    /// object.
    ///
    /// The `code` parameter specifies a source code to which this index object
    /// belongs.
    ///
    /// Returns None, if the index object is not [valid](Self::is_valid_site).
    #[inline(always)]
    fn to_position(&self, code: &impl SourceCode) -> Option<Position> {
        let site = self.to_site(code)?;

        let line = code.lines().line_of(site);
        let line_start = code.lines().line_start(line);
        let column = site - line_start + 1;

        Some(Position { line, column })
    }

    /// Returns a UTF-8 code point start byte index of the character to which
    /// this index object points in the `code`'s text.
    ///
    /// Returns None, if the index object is not [valid](Self::is_valid_site).
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

    /// Returns true if this index object considered valid within the `code`
    /// [SourceCode].
    ///
    /// The index validity is implementation dependent.
    fn is_valid_site(&self, code: &impl SourceCode) -> bool;

    /// Returns a displayable object that prints the underlying site object
    /// for debugging purposes.
    #[inline(always)]
    fn display<'a>(&self, code: &'a impl SourceCode) -> impl Debug + Display + 'a {
        DisplaySite {
            code,
            site: self.to_site(code),
        }
    }
}

struct DisplaySite<'a, Code: SourceCode> {
    code: &'a Code,
    site: Option<Site>,
}

impl<'a, Code> Debug for DisplaySite<'a, Code>
where
    Code: SourceCode,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl<'a, Code> Display for DisplaySite<'a, Code>
where
    Code: SourceCode,
{
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        let site = match &self.site {
            None => return formatter.write_str("?"),
            Some(site) => *site,
        };

        let position = match site.to_position(self.code) {
            Some(span) => span,

            // Safety: Sites are always valid to resolve.
            None => unsafe { ld_unreachable!("Invalid site.") },
        };

        if !formatter.alternate() {
            return formatter.write_fmt(format_args!("{}", position));
        }

        formatter
            .snippet(self.code)
            .set_caption(format!("Unit({})", self.code.id()))
            .set_summary(format!("Site: {site}\nPosition: {position}"))
            .annotate(site..site, AnnotationPriority::Default, "")
            .finish()
    }
}
