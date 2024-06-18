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

use std::fmt::{Debug, Formatter};

use crate::{
    lexis::{Length, Line, Site, SiteSpan},
    mem::{slice_copy_to, slice_shift},
    report::{ld_assert, ld_unreachable},
};

const LINE_LENGTH: Length = 40;
const SEARCH_THRESHOLD: usize = 10;
const CAPACITY_THRESHOLD: usize = 100;

/// An index of the text line spans.
///
/// This object represents a bidirectional map between the text line numbers and
/// the absolute [sites](Site) of the line beginnings.
///
/// The object provides functions to quickly fetch the line of the character
/// site, and the line start and end sites (line spans) by the line number.
///
/// The [write](LineIndex::write) function provides a way to modify the index.
///
/// The inner algorithm considers the `\n` character as the line delimiter
/// and includes both the `\r` and `\n` characters as parts of the line endings.
#[derive(Clone)]
pub struct LineIndex {
    index: Vec<Site>,
    length: Length,
}

impl Debug for LineIndex {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let width = (self.index.len().ilog10() + 1) as usize;

        let total = self.index.len();

        let mut debug_struct = match total == 1 {
            true => formatter.debug_struct("LineIndex(1 line)"),
            false => formatter.debug_struct(&format!("LineIndex({total} lines)")),
        };

        for mut line in 0..self.index.len() {
            line += 1;

            let length = self.line_length(line);
            let span = self.line_span(line);

            debug_struct.field(
                &format!("{line:0width$}", width = width),
                &format_args!("{{ span: {span:?}, length: {length} }}",),
            );
        }

        debug_struct.finish()
    }
}

impl Default for LineIndex {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl LineIndex {
    /// Constructs a new LineIndex for the empty string.
    #[inline(always)]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Constructs a new LineIndex for the empty string, but preallocates
    /// the inner index to address the text of `capacity` length.
    ///
    /// The allocation is amortized based on heuristics assumptions about
    /// the average line lengths.
    #[inline(always)]
    pub fn with_capacity(capacity: Length) -> Self {
        Self::with_capacity_from(capacity, 0, 0)
    }

    #[inline(always)]
    fn with_capacity_from(capacity: Length, from: Site, length: Length) -> Self {
        let mut index = Vec::with_capacity((capacity / LINE_LENGTH).max(CAPACITY_THRESHOLD));

        index.push(from);

        Self { index, length }
    }

    /// Returns the start [site](Site) of the `line`.
    ///
    /// See [Line] specification for details.
    #[inline(always)]
    pub fn line_start(&self, mut line: Line) -> Site {
        line = line
            .min(self.index.len())
            .checked_sub(1)
            .unwrap_or_default();

        ld_assert!(line < self.index.len(), "Empty index.");

        // Safety: `self.index` is never empty.
        unsafe { *self.index.get_unchecked(line) }
    }

    /// Returns the end [site](Site) of the `line`.
    ///
    /// The end sites points to the first character of the next line or points
    /// to the end of the text content.
    ///
    /// See [Line] specification for details.
    #[inline(always)]
    pub fn line_end(&self, mut line: Line) -> Site {
        line = line.clamp(1, self.index.len());

        self.index.get(line).copied().unwrap_or(self.length)
    }

    /// Returns a [site span](SiteSpan) of the line.
    ///
    /// The returning value equals the range between
    /// the [line_start](Self::line_start) and the [line_end](Self::line_end).
    #[inline(always)]
    pub fn line_span(&self, line: Line) -> SiteSpan {
        self.line_start(line)..self.line_end(line)
    }

    /// Returns the [length](Length) of the line.
    ///
    /// The returning value equals the difference between
    /// the [line_start](Self::line_start) and the [line_end](Self::line_end).
    #[inline(always)]
    pub fn line_length(&self, line: Line) -> Length {
        self.line_end(line) - self.line_start(line)
    }

    /// Returns the [line index](Line) of a character addressed by the `site`
    /// parameter.
    ///
    /// If the `site` exceeds the text content length, this function returns
    /// the last line index.
    #[inline(always)]
    pub fn line_of(&self, site: Site) -> Line {
        match self.index.binary_search(&site) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    /// Returns the total number of lines in the text.
    ///
    /// This value is always positive because an empty text is a text of
    /// one line.
    #[inline(always)]
    pub fn lines_count(&self) -> usize {
        self.index.len()
    }

    /// Reserves capacity for the inner index to address the lines of a text of
    /// at least `additional` length.
    ///
    /// The allocation is amortized based on heuristics assumptions about
    /// the average line lengths.
    #[inline(always)]
    pub fn reserve(&mut self, additional: Length) {
        self.index
            .reserve((additional / LINE_LENGTH).max(CAPACITY_THRESHOLD));
    }

    /// Modifies this LineIndex to reflect the edit in the source code content.
    ///
    /// The meaning of the `span` and the `text` parameters is the same as
    /// in the [Document::write](crate::units::Document::write) function. Refer
    /// to its specification for details.
    ///
    /// Note that the LineIndex does not store the text content internally, but
    /// the write function iterates through the `text` string to look up for the
    /// line delimiter sites.
    #[inline(always)]
    pub fn write(&mut self, span: SiteSpan, text: impl AsRef<str>) {
        if span.start > span.end || span.end > self.length {
            panic!("Invalid span.");
        }

        unsafe { self.write_unchecked(span, text.as_ref()) }
    }

    /// Shrinks the allocation capacity of the LineIndex as much as possible.
    #[inline(always)]
    pub fn shrink_to_fit(&mut self) {
        self.index.shrink_to_fit();
    }

    /// Clears the LineIndex while preserving allocated memory.
    #[inline(always)]
    pub fn clear(&mut self) {
        unsafe { self.index.set_len(1) }
        self.length = 0;
    }

    pub(crate) fn append(&mut self, text: &str) {
        for byte in text.as_bytes() {
            match byte & 0xC0 {
                0x80 => continue,

                0xc0 | 0x40 => {
                    self.length += 1;
                    continue;
                }

                _ => (),
            }

            self.length += 1;

            if byte == &b'\n' {
                self.index.push(self.length);
            }
        }
    }

    #[allow(dead_code)]
    // Safety: `span <= self.length()`
    pub(crate) unsafe fn shrink_unchecked(&mut self, span: Length) {
        ld_assert!(span <= self.length, "Shrink overflow.");

        self.length -= span;

        loop {
            let Some(last) = self.index.last() else {
                // Safety: index is never empty.
                unsafe { ld_unreachable!("Empty index.") }
            };

            if *last <= self.length {
                break;
            }

            unsafe { self.index.set_len(self.index.len() - 1) };

            ld_assert!(self.index.len() > 0, "Empty index.");
        }
    }

    // Safety:
    //   1. `span.start() <= span.end()`
    //   2. `span.end() <= self.length()`
    pub(crate) unsafe fn write_unchecked(&mut self, span: SiteSpan, text: &str) {
        ld_assert!(
            span.start <= span.end && span.end <= self.length,
            "Invalid span.",
        );

        if span.start == self.length {
            self.append(text);
            return;
        }

        let remove_length = span.end - span.start;

        let start_line = self.line_of(span.start);

        ld_assert!(start_line >= 1, "Invalid index.");
        ld_assert!(start_line <= self.index.len(), "Invalid index.");

        if start_line == self.index.len() {
            self.length -= remove_length;
            self.append(text);
            return;
        }

        let remove_lines = match remove_length == 0 {
            true => 1,

            false => {
                let right = unsafe { self.index.get_unchecked((start_line - 1)..) };

                match remove_length < SEARCH_THRESHOLD * LINE_LENGTH {
                    true => {
                        let mut counter = 0;

                        for site in right {
                            if site > &span.end {
                                break;
                            }

                            counter += 1;
                        }

                        counter
                    }

                    false => match right.binary_search(&span.end) {
                        Ok(index) => index + 1,
                        Err(index) => index,
                    },
                }
            }
        };

        ld_assert!(
            start_line + remove_lines - 1 <= self.index.len(),
            "Invalid index.",
        );

        if start_line + remove_lines - 1 == self.index.len() {
            self.length -= remove_length;
            unsafe { self.index.set_len(start_line) };

            self.append(text);

            return;
        }

        let start_line_site = self.line_start(start_line);

        let mut replacement = Self::with_capacity_from(text.len(), start_line_site, span.start);
        replacement.append(text);

        let replace_length = replacement.length - span.start;
        let replace_lines = replacement.index.len();

        if replace_lines > remove_lines {
            let diff = replace_lines - remove_lines;

            self.index.reserve(diff);

            unsafe { self.index.set_len(self.index.len() + diff) };

            let index_len = self.index.len();

            unsafe {
                slice_shift(
                    self.index.as_mut_slice(),
                    start_line + remove_lines - 1,
                    start_line + replace_lines - 1,
                    index_len - start_line - replace_lines + 1,
                );
            }
        } else if replace_lines < remove_lines {
            let index_len = self.index.len();

            unsafe {
                slice_shift(
                    self.index.as_mut_slice(),
                    start_line + remove_lines - 1,
                    start_line + replace_lines - 1,
                    index_len - start_line - remove_lines + 1,
                );
            }

            let diff = remove_lines - replace_lines;

            unsafe { self.index.set_len(self.index.len() - diff) };

            if diff > CAPACITY_THRESHOLD * 2 {
                self.index.shrink_to(self.index.len() + CAPACITY_THRESHOLD);
            }
        }

        unsafe {
            slice_copy_to(
                replacement.index.as_slice(),
                self.index.as_mut_slice(),
                0,
                start_line - 1,
                replace_lines,
            );
        }

        unsafe { replacement.index.set_len(0) };
        drop(replacement);

        if remove_length != replace_length {
            self.length -= remove_length;
            self.length += replace_length;

            ld_assert!(
                start_line + replace_lines - 1 < self.index.len(),
                "Invalid index.",
            );

            let rest = unsafe {
                self.index
                    .get_unchecked_mut(start_line + replace_lines - 1..)
            };

            for site in rest {
                *site -= remove_length;
                *site += replace_length;
            }
        }
    }

    #[inline(always)]
    pub(crate) fn code_length(&self) -> Length {
        self.length
    }
}

#[cfg(test)]
mod tests {
    use crate::lexis::LineIndex;

    #[test]
    fn test_line_index() {
        let mut index = LineIndex::new();
        index.append("555554444\n55555444щ\n\n333\n");

        assert_eq!(index.index, [0, 10, 20, 21, 25]);
        assert_eq!(index.length, 25);

        let mut index = LineIndex::new();

        index.append("55555");
        index.append("4444\n");
        index.append("55555");
        index.append("444щ\n");
        index.append("\n333\n");

        assert_eq!(index.index, [0, 10, 20, 21, 25]);
        assert_eq!(index.length, 25);

        index.write(25..25, "55555");

        assert_eq!(index.index, [0, 10, 20, 21, 25]);
        assert_eq!(index.length, 30);

        index.write(0..0, "55555");

        assert_eq!(index.index, [0, 15, 25, 26, 30]);
        assert_eq!(index.length, 35);

        index.write(0..10, "");

        assert_eq!(index.index, [0, 5, 15, 16, 20]);
        assert_eq!(index.length, 25);

        index.write(0..6, "");

        assert_eq!(index.index, [0, 9, 10, 14]);
        assert_eq!(index.length, 19);

        index.write(9..12, "\n\n");

        assert_eq!(index.index, [0, 9, 10, 11, 13]);
        assert_eq!(index.length, 18);

        unsafe {
            index.shrink_unchecked(5);
        }

        assert_eq!(index.index, [0, 9, 10, 11, 13]);
        assert_eq!(index.length, 13);

        unsafe {
            index.shrink_unchecked(1);
        }

        assert_eq!(index.index, [0, 9, 10, 11]);
        assert_eq!(index.length, 12);

        unsafe {
            index.shrink_unchecked(3);
        }

        assert_eq!(index.index, [0, 9]);
        assert_eq!(index.length, 9);

        unsafe {
            index.shrink_unchecked(9);
        }

        assert_eq!(index.index, [0]);
        assert_eq!(index.length, 0);
    }
}
