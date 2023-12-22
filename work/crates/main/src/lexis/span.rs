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
    lexis::{Position, Site, SiteRef, SourceCode, ToSite},
    report::debug_unreachable,
    std::*,
};

/// A range of [Sites](crate::lexis::Site).
///
/// The [ToSpan](crate::lexis::ToSpan) trait is auto-implemented for this object, and as such it
/// can be used to specify Spans.
///
/// Also, SiteSpan is a base Span representation. Any valid ToSpan object casts to SiteSpan.
///
/// SiteSpan bounds are zero-based index ranges. Bound value `0` means the first character, bound
/// value `1` is the second character, and so on up to the source code text
/// [length](crate::lexis::SourceCode::length).
///
/// SiteSpan may be an empty span(start bound equals to end bound). In this case the Span
/// represents a single [Site](crate::lexis::Site) inside the source code text. For example, if
/// an API user [writes](crate::Document::write) a text into the Document specifying empty span,
/// the Write operation becomes Insertion operations to specified Site.
///
/// SiteSpan is a valid span for any [SourceCode](crate::lexis::SourceCode) as long as its start
/// bound is lesser or equal to the end bound. If any of the Range bounds greater than the source
/// code text length, this bound will be clamped to the text character length value. This behavior
/// stems from the [ToSite](crate::lexis::ToSite) trait specification.
///
/// You can think about SiteSpan as a range of the text selected inside the code editor.
///
/// ```rust
/// use lady_deirdre::{units::Document, syntax::SimpleNode, lexis::SourceCode};
///
/// let doc = Document::<SimpleNode>::from("foo bar baz");
///
/// /// A substring of all characters starting from character number 4 inclusive till character
/// /// 7 exclusive.
/// assert_eq!(doc.substring(4..7), "bar");
/// ```
pub type SiteSpan = Range<Site>;

/// A range of [SiteRefs](crate::lexis::SiteRef).
///
/// The [ToSpan](crate::lexis::ToSpan) trait is auto-implemented for this object, and as such it
/// can be used to specify Spans.
///
/// ```rust
/// use lady_deirdre::{
///     units::Document,
///     syntax::SimpleNode,
///     lexis::{SourceCode, TokenCursor},
/// };
///
/// let doc = Document::<SimpleNode>::from("foo bar baz");
///
/// let start = doc.cursor(..).site_ref(2);
/// let end = doc.cursor(..).site_ref(3);
///
/// assert_eq!(doc.substring(start..end), "bar");
/// ```
pub type SiteRefSpan = Range<SiteRef>;

impl Identifiable for SiteRefSpan {
    #[inline(always)]
    fn id(&self) -> Id {
        let id = self.start.id();

        if self.end.id() != id {
            return Id::nil();
        }

        id
    }
}

/// A range of [Positions](crate::lexis::Position).
///
/// The [ToSpan](crate::lexis::ToSpan) trait is auto-implemented for this object, and as such it
/// can be used to specify Spans.
///
/// ```rust
/// use lady_deirdre::{units::Document, syntax::SimpleNode, lexis::{SourceCode, Position}};
///
/// let doc = Document::<SimpleNode>::from("foo bar baz");
///
/// assert_eq!(doc.substring(Position::new(1, 5)..Position::new(1, 8)), "bar");
/// ```
pub type PositionSpan = Range<Position>;

/// An interface of the source code character index range("Span").
///
/// The underlying object may be a valid or invalid Span representation for particular
/// [SourceCode](crate::lexis::SourceCode) instance. If the object considered to be not valid,
/// [is_valid_span](ToSpan::is_valid_span) function returns `false`, and
/// [to_span](ToSpan::to_site_span) function returns [None]. Otherwise "is_valid_span" returns `true`,
/// and "to_span" returns meaningful [SiteSpan](crate::lexis::SiteSpan) value.
///
/// It is up to implementation to decide whether particular instance considered to be valid or not.
///
/// This trait is implemented for the [RangeFull](::std::ops::RangeFull) object(a `..` shortcut)
/// that is always valid and always resolves to the SiteSpan covering the entire source code text.
///
/// For any type that implements a [ToSite](crate::lexis::ToSite) trait, the ToSpan trait is
/// auto-implemented for all variants of Rust's standard ranges(Range, RangeFrom, etc) over
/// this type. As such if an API user implements ToSite trait, the user receives ToSpan range
/// implementations over this type out of the box. Such ToSpan auto-implementations considered to be
/// valid Spans as long as the original range bounds are
/// [`site-valid`](crate::lexis::ToSite::is_valid_site) values, and if the start Site bound
/// does not exceed the end Site bound.
///
/// ```rust
/// use lady_deirdre::{
///     units::Document,
///     syntax::SimpleNode,
///     lexis::{ToSpan, ToSite, SourceCode, SiteSpan, Site},
/// };
///
/// let doc = Document::<SimpleNode>::from("foo bar baz");
///
/// /// A substring of all characters starting from character number 4 inclusive till character
/// /// 7 exclusive.
/// assert_eq!(doc.substring(4..7), "bar");
///
/// /// A custom Span implementation that resolves to the first half of the source code text.
/// struct Half;
///
/// // This is safe because "to_span" always returns SiteSpan within the source code text
/// // character bounds.
/// unsafe impl ToSpan for Half {
///     fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
///         Some(0..code.length() / 2)
///     }
///
///     fn is_valid_span(&self, _code: &impl SourceCode) -> bool { true }
/// }
///
/// assert_eq!(doc.substring(Half), "foo b");
///
/// // A custom one-based Site index.
/// struct OneBaseSite(usize);
///
/// // This is safe because "to_site" implementation checks underlying value .
/// unsafe impl ToSite for OneBaseSite {
///     fn to_site(&self, code: &impl SourceCode) -> Option<Site> {
///         if self.0 == 0 || self.0 > code.length() { return None; }
///
///         Some(self.0 - 1)
///     }
///
///     fn is_valid_site(&self, code: &impl SourceCode) -> bool {
///         self.0 > 0 && self.0 <= code.length()
///     }
/// }
///
/// // Since ToSite implemented for the OneBaseSite object, all types of ranges over this
/// // object are ToSpan as well.
/// assert_eq!(doc.substring(OneBaseSite(5)..OneBaseSite(8)), "bar");
/// assert_eq!(doc.substring(OneBaseSite(5)..=OneBaseSite(7)), "bar");
/// ```
///
/// **Safety:**
///   - If the [to_span](ToSite::to_span) function returns [Some] range, the range start bound value
///     does not exceed range end bound value, and the range's end bound value does not exceed
///     [`SourceCode::length`](crate::lexis::SourceCode::length) value.
pub unsafe trait ToSpan {
    /// Returns valid [SiteSpan](crate::lexis::SiteSpan) representation of this Span object
    /// if the Span object is valid Span for specified `code` parameter. Otherwise returns [None].
    ///
    /// The validity of the Span object is implementation specific.
    ///
    /// The returning SiteSpan [Range](::std::ops::Range) start bound value does not exceed end
    /// bound value, and the range's end bound does not exceed `SourceCode::length(code)` value.   
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan>;

    fn to_position_span(&self, code: &impl SourceCode) -> Option<PositionSpan> {
        let span = self.to_site_span(code)?;

        Some(span.start.to_position(code)?..span.end.to_position(code)?)
    }

    /// Returns `true` if and only if the [to_span](ToSpan::to_site_span) function would return
    /// [Some] value for specified `code` parameter.
    fn is_valid_span(&self, code: &impl SourceCode) -> bool;

    /// A helper function to format specified Span.
    ///
    /// This function tries to resolve spanned object and to format its bounds in form of
    /// [PositionSpan](crate::lexis::PositionSpan). If resolution is not possible the function
    /// returns `"?"` string.
    ///
    /// ```rust
    /// use lady_deirdre::{units::Document, syntax::SimpleNode, lexis::{ToSpan, SourceCode}};
    ///
    /// let doc = Document::<SimpleNode>::from("foo\nbar baz");
    ///
    /// assert_eq!(doc.substring(2..7), "o\nbar");
    /// assert_eq!((2..7).display(&doc).to_string(), "1:3 (5 chars, 1 line break)");
    /// ```
    #[inline(always)]
    fn display<'a, Code: SourceCode>(&self, code: &'a Code) -> DisplaySpan<'a, Code> {
        DisplaySpan {
            code,
            span: self.to_site_span(code),
        }
    }
}

unsafe impl<S: ToSpan> ToSpan for &S {
    #[inline(always)]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        (*self).to_site_span(code)
    }

    #[inline(always)]
    fn is_valid_span(&self, code: &impl SourceCode) -> bool {
        (*self).is_valid_span(code)
    }
}

unsafe impl ToSpan for RangeFull {
    #[inline(always)]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        Some(0..code.length())
    }

    #[inline(always)]
    fn is_valid_span(&self, _code: &impl SourceCode) -> bool {
        true
    }
}

unsafe impl<Site: ToSite> ToSpan for Range<Site> {
    #[inline]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        let start = self.start.to_site(code);
        let end = self.end.to_site(code);

        match (start, end) {
            (Some(start), Some(end)) if start <= end => Some(start..end),
            _ => None,
        }
    }

    #[inline]
    fn is_valid_span(&self, code: &impl SourceCode) -> bool {
        let start = self.start.to_site(code);
        let end = self.end.to_site(code);

        match (start, end) {
            (Some(start), Some(end)) if start <= end => true,
            _ => false,
        }
    }
}

unsafe impl<Site: ToSite> ToSpan for RangeInclusive<Site> {
    #[inline]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        let start = self.start().to_site(code);
        let end = self.end().to_site(code);

        match (start, end) {
            (Some(start), Some(mut end)) if start <= end => {
                if end < code.length() && end < usize::MAX {
                    end += 1;
                }

                Some(start..end)
            }
            _ => None,
        }
    }

    #[inline]
    fn is_valid_span(&self, code: &impl SourceCode) -> bool {
        let start = self.start().to_site(code);
        let end = self.end().to_site(code);

        match (start, end) {
            (Some(start), Some(end)) if start <= end => true,
            _ => false,
        }
    }
}

unsafe impl<Site: ToSite> ToSpan for RangeFrom<Site> {
    #[inline]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        let start = match self.start.to_site(code) {
            None => return None,
            Some(site) => site,
        };

        let end = code.length();

        Some(start..end)
    }

    #[inline(always)]
    fn is_valid_span(&self, code: &impl SourceCode) -> bool {
        self.start.is_valid_site(code)
    }
}

unsafe impl<Site: ToSite> ToSpan for RangeTo<Site> {
    #[inline]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        let end = match self.end.to_site(code) {
            None => return None,
            Some(site) => site,
        };

        Some(0..end)
    }

    #[inline(always)]
    fn is_valid_span(&self, code: &impl SourceCode) -> bool {
        self.end.is_valid_site(code)
    }
}

unsafe impl<Site: ToSite> ToSpan for RangeToInclusive<Site> {
    #[inline]
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan> {
        let end = match self.end.to_site(code) {
            None => return None,
            Some(site) => {
                if site < code.length() && site < usize::MAX {
                    site + 1
                } else {
                    site
                }
            }
        };

        Some(0..end)
    }

    #[inline(always)]
    fn is_valid_span(&self, code: &impl SourceCode) -> bool {
        self.end.is_valid_site(code)
    }
}

pub struct DisplaySpan<'a, Code: SourceCode> {
    code: &'a Code,
    span: Option<SiteSpan>,
}

impl<'a, Code> Debug for DisplaySpan<'a, Code>
where
    Code: SourceCode,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        Display::fmt(self, formatter)
    }
}

impl<'a, Code> Display for DisplaySpan<'a, Code>
where
    Code: SourceCode,
{
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        let span = match &self.span {
            None => return formatter.write_str("?"),
            Some(span) => span.clone(),
        };

        if !formatter.alternate() {
            let chars = span.end - span.start;
            let breaks = self.code.chars(&span).filter(|ch| *ch == '\n').count();

            let span = match span.to_position_span(self.code) {
                Some(span) => span,

                // Safety: Site spans are always valid to resolve.
                None => unsafe { debug_unreachable!("Invalid position span.") },
            };

            formatter.write_fmt(format_args!("{}", span.start))?;

            if chars > 0 {
                formatter.write_str(" (")?;

                match chars > 1 {
                    false => formatter.write_str("1 char")?,
                    true => formatter.write_fmt(format_args!("{chars} chars"))?,
                }

                match breaks {
                    0 => (),
                    1 => formatter.write_str(", 1 line break")?,
                    _ => formatter.write_fmt(format_args!(", {breaks} line breaks"))?,
                }

                formatter.write_str(")")?;
            }

            return Ok(());
        }

        formatter
            .snippet(self.code)
            .set_caption(format!("Unit({})", self.code.id()))
            .set_summary(format!(
                "Site span: {}..{}\nPosition span: {}",
                span.start,
                span.end,
                span.display(self.code),
            ))
            .annotate(span, Priority::Default, "")
            .finish()
    }
}
