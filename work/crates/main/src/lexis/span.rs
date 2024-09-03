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
    fmt::{Debug, Display, Formatter},
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
};

use crate::{
    arena::{Id, Identifiable},
    format::{AnnotationPriority, SnippetFormatter},
    lexis::{Position, Site, SiteRef, SourceCode, ToSite},
    report::ld_unreachable,
};

/// A span between two Unicode characters.
///
/// For example, `10..18` is a site span that starts from the 10nth character
/// (inclusive) and lasts until the 18th character (exclusive). The total
/// length of such a span is 8 Unicode chars.
///
/// A **SiteSpan is considered valid for any [SourceCode]** as long as the
/// end bound of the range is greater or equal to the start bound. If the bounds
/// exceed source code length, they will be clamped.
///
/// See [ToSpan] for details.
pub type SiteSpan = Range<Site>;

/// A span between two [pinned sites](SiteRef).
///
/// This kind of span between two SiteRefs resolves to the [SiteSpan] of the
/// sites of these SiteRefs.
///
/// This object allows you to specify a span between two pinned points in the
/// source code rather than the absolute Unicode character [sites](Site).
///
/// Whenever the end user writes text to the underlying compilation unit
/// outside of the span or inside the span, but the text edit does not include
/// the SiteRefSpan bounds, the SiteRefSpan bounds automatically adjust in
/// accordance with the source code edits: the absolute site of the bound will
/// be shifted left or right accordingly.
///
/// However, if the edit affects the SiteRefSpan bound tokens during
/// incremental reparsing, the entire object may become invalid.
///
/// See [ToSpan] for details.
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

/// A span between two Unicode characters addressed by the text
/// [line-column positions](Position).
///
/// For example, `Position::new(2, 10)..Position(2, 18)` is a position span that
/// starts from the 9nth character of the second line (inclusive) and lasts
/// until the 17th character of the second line (exclusive). The total
/// length of such a span is 8 Unicode chars.
///
/// A PositionSpan is considered valid for any [SourceCode] as long as the
/// end bound of the range is greater or equal to the start bound. If the bounds
/// exceed source code length, they will be clamped.
///
/// See [ToSpan] for details.
pub type PositionSpan = Range<Position>;

/// An object that addresses a fragment of the source code text.
///
/// In Lady Deirdre, a minimal unit of text measurement is a Unicode character.
///
/// Addressing code ranges ("spans") just by the ranges of the Unicode
/// absolute indices ([SiteSpan]) would be inconvenient.
///
/// The ToSpan trait is a generic interface that provides conversion between
/// custom types of spans and the SiteSpans.
///
/// In particular, Lady Deirdre provides the following custom span types that
/// implement the ToSpan trait:
///
///  - The [SiteSpan] itself, which is a range of absolute Unicode char
///    indices: `10..20`.
///  - The [PositionSpan], which is a range in terms of the line-column indices:
///    `Position::new(10, 20)..Position::new(15, 28)`
///  - The [SiteRefSpan], which is a range between the
///    [TokenRef](crate::lexis::TokenRef) bounds.
///
/// You are encouraged to provide your own implementations of the [ToSpan] on
/// custom span types depending on the needs.
///
/// For convenient purposes, for any type that implements [ToSite] trait, which
/// is a trait of custom text indices, standard Rust ranges with the bounds
/// of this type implement the ToSpan trait: `10..=20`, `Position::new(8, 6)..`
/// are all valid span types.
///
/// Additionally, the `..` ([RangeFull]) implements the [ToSpan] trait and
/// denotes the full source code text range.
///
/// Also, if the type `T` implements to ToSpan, its referential type `&T`
/// implements ToSpan too.
///
/// **Safety**
///
/// The implementor of the trait guarantees the following:
///
///  1. If the [ToSpan::to_site_span] function returns Some site span, the
///     lower range bound is less than or equal to the upper bound, and
///     the upper bound is less than or equal to the `code`'s
///     [length](SourceCode::length).
///     In other words, the function returns a valid span within the source
///     code text bounds.
///
///  2. The [ToSpan::to_site_span] and [ToSpan::to_position_span] return Some
///     value if and only if the [ToSpan::is_valid_span] returns true for
///     the same source code.
pub unsafe trait ToSpan {
    /// Returns a [SiteSpan] representation of this span object.
    ///
    /// The `code` parameter specifies a source code to which this span object
    /// belongs.
    ///
    /// The returning SiteSpan is a valid range within the [SourceCode] bounds.
    ///
    /// Returns None, if the span object is not [valid](Self::is_valid_span).
    fn to_site_span(&self, code: &impl SourceCode) -> Option<SiteSpan>;

    /// Returns a [line-column range](PositionSpan) representation of this span
    /// object.
    ///
    /// The `code` parameter specifies a source code to which this span object
    /// belongs.
    ///
    /// Returns None, if the span object is not [valid](Self::is_valid_span).
    fn to_position_span(&self, code: &impl SourceCode) -> Option<PositionSpan> {
        let span = self.to_site_span(code)?;

        Some(span.start.to_position(code)?..span.end.to_position(code)?)
    }

    /// Returns true if this span object considered valid within the `code`
    /// [SourceCode].
    ///
    /// The span validity is implementation dependent.
    ///
    /// For the range-like spans (such as [Range], [RangeTo], etc), the range
    /// considered valid as long as the range bounds
    /// are [valid sites](ToSite::is_valid_site), and the start site of
    /// the range does not exceed the range's end site.
    ///
    /// Note that the [SiteSpan] range (with the start bound less than or equal
    /// to the end bound) is always valid span because the [ToSite]
    /// implementation of the [Site] always clamps the site to the SourceCode
    /// [length](SourceCode::length).
    fn is_valid_span(&self, code: &impl SourceCode) -> bool;

    /// Returns a displayable object that prints the underlying span object
    /// for debugging purposes.
    #[inline(always)]
    fn display<'a>(&self, code: &'a impl SourceCode) -> impl Debug + Display + 'a {
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

struct DisplaySpan<'a, Code: SourceCode> {
    code: &'a Code,
    span: Option<SiteSpan>,
}

impl<'a, Code> Debug for DisplaySpan<'a, Code>
where
    Code: SourceCode,
{
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl<'a, Code> Display for DisplaySpan<'a, Code>
where
    Code: SourceCode,
{
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
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
                None => unsafe { ld_unreachable!("Invalid position span.") },
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
            .annotate(span, AnnotationPriority::Default, "")
            .finish()
    }
}
