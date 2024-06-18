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

///////////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of David Tolnay's                  //
// "prettyplease" work.                                                                  //
//                                                                                       //
// David Tolnay's original work available here:                                          //
// https://github.com/dtolnay/prettyplease/tree/6f7a9eebd7052fd5c37a84135e1daa7599176e7e //
//                                                                                       //
// David Tolnay grants me with a license to his work under the following terms:          //
//                                                                                       //
//   Permission is hereby granted, free of charge, to any                                //
//   person obtaining a copy of this software and associated                             //
//   documentation files (the "Software"), to deal in the                                //
//   Software without restriction, including without                                     //
//   limitation the rights to use, copy, modify, merge,                                  //
//   publish, distribute, sublicense, and/or sell copies of                              //
//   the Software, and to permit persons to whom the Software                            //
//   is furnished to do so, subject to the following                                     //
//   conditions:                                                                         //
//                                                                                       //
//   The above copyright notice and this permission notice                               //
//   shall be included in all copies or substantial portions                             //
//   of the Software.                                                                    //
//                                                                                       //
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF                               //
//   ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED                             //
//   TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A                                 //
//   PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT                                 //
//   SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY                            //
//   CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION                             //
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR                             //
//   IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER                                 //
//   DEALINGS IN THE SOFTWARE.                                                           //
//                                                                                       //
// Kindly be advised that the terms governing the distribution of my work are            //
// distinct from those pertaining to the original work of David Tolnay.                  //
///////////////////////////////////////////////////////////////////////////////////////////

use std::collections::VecDeque;

use crate::{
    lexis::Length,
    report::{ld_unreachable, system_panic},
};

/// A configuration of the [PrettyPrinter] defaults.
///
/// This structure is non-exhaustive; new configuration options may be added
/// in future minor versions of this crate.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub struct PrettyPrintConfig {
    /// If the line absolute length exceeds the `margin` value, the printer may
    /// attempt to break it into multiple lines.
    ///
    /// The default value is 80.
    pub margin: u16,

    /// If the printer breaks the line into multiple lines, it should attempt to
    /// keep at least the `inline` number of characters in line relative to the
    /// current indentation.
    ///
    /// The default value is 60.
    pub inline: u16,

    /// A number of whitespaces in a single indentation step.
    ///
    /// The default value is 4.
    pub indent: u16,

    /// If set to true, the printer prints debug symbols directly into the output.
    ///
    /// The default value is false.
    pub debug: bool,
}

impl Default for PrettyPrintConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl PrettyPrintConfig {
    /// Creates a new configuration object with all fields set to defaults.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            margin: 80,
            inline: 60,
            indent: 4,
            debug: false,
        }
    }
}

/// A core component of the source code formatter.
///
/// Under the hood, this object provides an implementation of
/// the [Derek C. Oppen, "Pretty Printing" (1979)](http://i.stanford.edu/pub/cstr/reports/cs/tr/79/770/CS-TR-79-770.pdf)
/// language-agnostic algorithm, which efficiently breaks a string of the source
/// code into multiple lines with indentation according to the provided
/// rules. Adaptions of this algorithm are used by many modern code formatters,
/// including the default Rust code formatter.
///
/// The input of the algorithm is a stream of words, blank tokens, and
/// tokens that denote enclosed word groups. As output, the algorithm prints
/// these words and decides for each blank token whether it should be printed
/// as whitespace or a line break, ensuring that each output line length
/// is limited by a margin.
///
/// For input you can use a [ParseTree](crate::syntax::ParseTree) that preserves
/// all words of the source code with the blank tokens and the comments.
pub struct PrettyPrinter {
    output: String,
    debug: bool,
    margin: LengthSigned,
    inline: LengthSigned,
    step: LengthSigned,
    space: LengthSigned,
    right: LengthSigned,
    left: LengthSigned,
    scan_queue: VecDeque<ScanEntry>,
    scan_stack: VecDeque<usize>,
    scan_consumed: usize,
    print_stack: Vec<PrintFrame>,
    indent: LengthSigned,
    pending_whitespace: usize,
    whitespaces: Vec<String>,
}

impl PrettyPrinter {
    /// Creates a new pretty printer.
    ///
    /// The `config` parameter specifies output defaults, such as the breaking margin.
    #[inline(always)]
    pub fn new(config: PrettyPrintConfig) -> Self {
        let margin = config.margin as LengthSigned;
        let inline = config.inline.min(config.margin) as LengthSigned;
        let step = config.indent as LengthSigned;

        Self {
            output: String::new(),
            debug: config.debug,
            margin,
            inline,
            step,
            space: margin,
            right: 0,
            left: 0,
            scan_queue: VecDeque::new(),
            scan_stack: VecDeque::new(),
            scan_consumed: 0,
            print_stack: Vec::new(),
            indent: 0,
            pending_whitespace: 0,
            whitespaces: Vec::new(),
        }
    }

    /// Starts a new **inconsistent** word group.
    ///
    /// If the content of this group does not fit in line, the blank tokens and
    /// the inner groups of where the content exceeds the line limit receive
    /// line breaks. Other blank tokens receive whitespaces.
    ///
    /// This type of group is useful for printing a chain of binary operators
    /// with simple operands (`10 + 20 + 30`), or a list of simple items
    /// (`10, 20, 30`) when most of the inner items are assumed to stay in line.
    ///
    /// The group **must be finished** by calling the [end](Self::end) function.
    #[inline(always)]
    pub fn ibox(&mut self, ident: isize) {
        let step = self.step;

        self.scan_begin(Group {
            mode: Mode::Inconsistent,
            indent: step * ident,
        });
    }

    /// Starts a new **consistent** word group.
    ///
    /// If the content of this group does not fit in line, **all** blank tokens
    /// and the inner groups receive line breaks.
    ///
    /// This type of group is useful for printing blocks of code and similar
    /// elements.
    ///
    /// The group **must be finished** by calling the [end](Self::end) function.
    #[inline(always)]
    pub fn cbox(&mut self, ident: isize) {
        let step = self.step;

        self.scan_begin(Group {
            mode: Mode::Consistent,
            indent: step * ident,
        });
    }

    /// Finishes the previously started [consistent](Self::cbox) or
    /// [inconsistent](Self::ibox) word group.
    #[inline(always)]
    pub fn end(&mut self) {
        self.scan_end();
    }

    /// Feeds a word into the input stream.
    ///
    /// The `word` parameter is a string that should be treated as a whole
    /// unbreakable element in the output. Normally, this string should not
    /// have line breaks. To enforce the line breaking inside the "word",
    /// split this word's string into inline words, and put
    /// [hardbreaks](Self::hardbreak) between them.
    #[inline(always)]
    pub fn word(&mut self, word: impl Into<String>) {
        self.scan_string(word.into());
    }

    /// Feeds a normal blank token into the input stream.
    ///
    /// This token will receive either a whitespace or a line break in
    /// the output depending on the algorithm's decision.
    #[inline(always)]
    pub fn blank(&mut self) {
        self.scan_blank(Blank {
            space: 1,
            indent: 0,
            pre_break: None,
            pre_space: None,
            neverbreak: false,
        });
    }

    /// Feeds a blank token into the input stream that enforces line break.
    ///
    /// This token will always receive line break in the output.
    #[inline(always)]
    pub fn hardbreak(&mut self) {
        self.scan_blank(Blank {
            space: SIZE_INFINITY as Length,
            indent: 0,
            pre_break: None,
            pre_space: None,
            neverbreak: false,
        });
    }

    /// Feeds a blank token of zero length into the input stream.
    ///
    /// This token will receive a line break when the algorithm breaks
    /// the line, but does not receive any character when the algorithm
    /// keeps it inline.
    #[inline(always)]
    pub fn softbreak(&mut self) {
        self.scan_blank(Blank {
            space: 0,
            indent: 0,
            pre_break: None,
            pre_space: None,
            neverbreak: false,
        });
    }

    /// Feeds a blank token that extends the inline content.
    ///
    /// When the algorithm decides that the incoming content is too long and
    /// needs to be split into multiple lines, it assigns the pending blank
    /// tokens the line break characters.
    ///
    /// As a result, the algorithm splits the content as early as possible
    /// keeping the tails inline. Which is desirable in most situations.
    ///
    /// However, sometimes we want to keep the heading of the content inline
    /// splitting the tails instead.
    ///
    /// The "neverbreak" token enforces the algorithm to reset its inner
    /// line length counter when the token is encountered. The algorithm assume
    /// that this token breaks the line without actually breaking it.
    #[inline(always)]
    pub fn neverbreak(&mut self) {
        self.scan_blank(Blank {
            space: 0,
            indent: 0,
            pre_break: None,
            pre_space: None,
            neverbreak: true,
        });
    }

    /// Changes the indentation step of the blank token.
    ///
    /// Whenever the blank token receives a line break, each new line after
    /// the break and within the current group receive whitespace indentations.
    ///
    /// By default, the blank tokens don't change lines indentation, but you
    /// can explicitly increase or decrease the indentation using this function.
    ///
    /// The `indent` parameter, depending on the value sign, increases or
    /// decreases lines indentations that follow after the current blank token.
    ///
    /// This function should be called immediately after the blank token
    /// submission; otherwise it has no effect.
    #[inline(always)]
    pub fn indent(&mut self, indent: isize) {
        let offset = self.step * indent;

        if let Some(blank) = self.blank_token() {
            blank.indent = offset;
        }
    }

    /// Assigns a word to the blank token that will be inserted before the
    /// blank token, if the algorithm assigns a line break character to
    /// the token.
    ///
    /// This function should be called immediately after the blank token
    /// submission; otherwise it has no effect.
    #[inline(always)]
    pub fn pre_break(&mut self, string: impl Into<String>) {
        if let Some(blank) = self.blank_token() {
            blank.pre_break = Some(string.into());
        }
    }

    /// Assigns a word to the blank token that will be inserted before the
    /// blank token, if the algorithm decides to keep the content inline.
    ///
    /// This function should be called immediately after the blank token
    /// submission; otherwise it has no effect.
    #[inline(always)]
    pub fn pre_space(&mut self, string: impl Into<String>) {
        if let Some(blank) = self.blank_token() {
            blank.pre_space = Some(string.into());
        }
    }

    /// Finishes content formatting and returns a final output string.
    pub fn finish(mut self) -> String {
        if !self.scan_stack.is_empty() {
            self.handle_scan_stack();
            self.consume();
        }

        self.output
    }

    #[inline(always)]
    fn blank_token(&mut self) -> Option<&mut Blank> {
        match self.scan_queue.back_mut() {
            Some(ScanEntry {
                token: ScanToken::Blank(blank),
                ..
            }) => Some(blank),

            _ => None,
        }
    }
}

impl PrettyPrinter {
    fn scan_begin(&mut self, group: Group) {
        if self.scan_stack.is_empty() {
            self.left = 1;
            self.right = 1;
            self.scan_queue.clear();
        }

        let index = self.scan_consumed + self.scan_queue.len();

        self.scan_queue.push_back(ScanEntry {
            token: ScanToken::Begin(group),
            size: -self.right,
        });

        self.scan_stack.push_back(index);
    }

    fn scan_end(&mut self) {
        if self.scan_stack.is_empty() {
            self.print_end();
            return;
        }

        let index = self.scan_consumed + self.scan_queue.len();

        self.scan_queue.push_back(ScanEntry {
            token: ScanToken::End,
            size: -1,
        });

        self.scan_stack.push_back(index);
    }

    fn scan_blank(&mut self, blank: Blank) {
        if self.scan_stack.is_empty() {
            self.left = 1;
            self.right = 1;
            self.scan_queue.clear();
        } else {
            self.handle_scan_stack();
        }

        let mut space = blank.space;

        if let Some(pre) = &blank.pre_space {
            space += pre.len();
        }

        let index = self.scan_consumed + self.scan_queue.len();

        self.scan_queue.push_back(ScanEntry {
            token: ScanToken::Blank(blank),
            size: -self.right,
        });

        self.scan_stack.push_back(index);

        self.right += space as LengthSigned;
    }

    fn scan_string(&mut self, string: String) {
        if self.scan_stack.is_empty() {
            self.print_string(string);
            return;
        }

        let size = string.len() as LengthSigned;

        self.scan_queue.push_back(ScanEntry {
            token: ScanToken::String(string),
            size,
        });

        self.right += size;

        self.handle_stream();
    }

    fn handle_stream(&mut self) {
        while self.right - self.left > self.space {
            if let Some(index) = self.scan_stack.front() {
                if index == &self.scan_consumed {
                    let _ = self.scan_stack.pop_front();
                    if let Some(entry) = self.scan_queue.front_mut() {
                        entry.size = SIZE_INFINITY;
                    }
                }
            }

            self.consume();

            if self.scan_queue.is_empty() {
                return;
            }
        }
    }

    fn consume(&mut self) {
        while !self.scan_queue.is_empty() {
            match self.scan_queue.front() {
                Some(entry) if entry.size >= 0 => (),
                _ => break,
            }

            let entry = match self.scan_queue.pop_front() {
                // Safety: Non-emptiness checked above
                None => unsafe { ld_unreachable!("Empty scan queue.") },

                Some(entry) => entry,
            };

            self.scan_consumed += 1;

            match entry.token {
                ScanToken::String(string) => {
                    self.left += entry.size;
                    self.print_string(string);
                }

                ScanToken::Blank(blank) => {
                    self.left += blank.space as LengthSigned;

                    if let Some(pre) = &blank.pre_space {
                        self.left += pre.len() as LengthSigned;
                    }

                    self.print_blank(blank, entry.size);
                }

                ScanToken::Begin(group) => self.print_begin(&group, entry.size),

                ScanToken::End => self.print_end(),
            }
        }
    }

    fn handle_scan_stack(&mut self) {
        let mut depth = 0usize;

        while let Some(index) = self.scan_stack.back() {
            let index = match index.checked_sub(self.scan_consumed) {
                Some(index) => index,
                None => {
                    system_panic!("Inconsistent scan stack indices.");
                    return;
                }
            };

            let entry = match self.scan_queue.get_mut(index) {
                Some(entry) => entry,
                _ => {
                    system_panic!("Inconsistent scan stack indices.");
                    return;
                }
            };

            match &entry.token {
                ScanToken::Begin(..) => {
                    depth = match depth.checked_sub(1) {
                        Some(depth) => depth,
                        None => break,
                    };

                    let _ = self.scan_stack.pop_back();
                    entry.size += self.right;
                }

                ScanToken::End => {
                    let _ = self.scan_stack.pop_back();
                    entry.size = 1;
                    depth += 1;
                }

                ScanToken::Blank(..) => {
                    let _ = self.scan_stack.pop_back();
                    entry.size += self.right;

                    if depth == 0 {
                        break;
                    }
                }

                ScanToken::String(..) => system_panic!("Inconsistent scan stack."),
            }
        }
    }

    fn print_begin(&mut self, group: &Group, size: LengthSigned) {
        if self.debug {
            self.output.push(match group.mode {
                Mode::Consistent => '«',
                Mode::Inconsistent => '‹',
            });

            self.output
                .extend(group.indent.to_string().chars().map(|ch| match ch {
                    '0'..='9' => ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉']
                        [(ch as u8 - b'0') as usize],
                    '-' => '₋',

                    _ => {
                        system_panic!("Non-numeric character.");
                        ' '
                    }
                }));
        }

        if size <= self.space {
            self.print_stack.push(PrintFrame::Inline(group.mode));
            return;
        }

        self.print_stack
            .push(PrintFrame::Break(group.mode, self.indent));
        self.indent += group.indent;
    }

    fn print_end(&mut self) {
        let mode = match self.print_stack.pop().unwrap() {
            PrintFrame::Inline(mode) => mode,
            PrintFrame::Break(mode, indent) => {
                self.indent = indent;
                mode
            }
        };

        if self.debug {
            self.output.push(match mode {
                Mode::Consistent => '»',
                Mode::Inconsistent => '›',
            });
        }
    }

    fn print_blank(&mut self, blank: Blank, size: LengthSigned) {
        let inline = blank.neverbreak
            || match self.frame() {
                PrintFrame::Inline(..) => true,
                PrintFrame::Break(Mode::Consistent, ..) => false,
                PrintFrame::Break(Mode::Inconsistent, ..) => size <= self.space,
            };

        if self.debug {
            self.output.push('·');
        }

        if inline {
            if let Some(pre) = blank.pre_space {
                self.print_string(pre)
            }

            self.pending_whitespace = blank.space;
            self.space -= blank.space as LengthSigned;

            return;
        }

        if let Some(pre) = blank.pre_break {
            self.print_whitespace();
            self.output.push_str(&pre);
        }

        self.output.push('\n');

        let indent = self.indent + blank.indent;
        self.pending_whitespace = usize::try_from(indent).unwrap_or(0);
        self.space = self.inline.max(self.margin - indent);
    }

    fn print_string(&mut self, string: String) {
        let string_length = string.len();

        self.print_whitespace();
        self.output.push_str(&string);
        self.space -= string_length as LengthSigned;
    }

    fn print_whitespace(&mut self) {
        if self.pending_whitespace == 0 {
            return;
        }

        loop {
            match self.whitespaces.get(self.pending_whitespace) {
                None => (),
                Some(whitespaces) => {
                    self.output.push_str(whitespaces);
                    break;
                }
            }

            match self.whitespaces.last() {
                None => self.whitespaces.push(String::new()),
                Some(previous) => {
                    let mut string = previous.clone();
                    string.push(' ');
                    self.whitespaces.push(string);
                }
            }
        }

        self.pending_whitespace = 0;
    }

    #[inline(always)]
    fn frame(&self) -> &PrintFrame {
        self.print_stack
            .last()
            .unwrap_or(&PrintFrame::Break(Mode::Inconsistent, 0))
    }
}

type LengthSigned = isize;

const SIZE_INFINITY: LengthSigned = 0x10000;

struct Blank {
    space: Length,
    indent: LengthSigned,
    pre_break: Option<String>,
    pre_space: Option<String>,
    neverbreak: bool,
}

enum ScanToken {
    String(String),
    Blank(Blank),
    Begin(Group),
    End,
}

struct Group {
    mode: Mode,
    indent: LengthSigned,
}

enum PrintFrame {
    Inline(Mode),
    Break(Mode, LengthSigned),
}

#[derive(Clone, Copy)]
enum Mode {
    Consistent,
    Inconsistent,
}

struct ScanEntry {
    token: ScanToken,
    size: LengthSigned,
}
