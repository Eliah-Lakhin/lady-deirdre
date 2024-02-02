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

///////////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of David Tolnay's                  //
// "prettyplease" work.                                                                  //
//                                                                                       //
// David Tolnay's original work available here:                                          //
// https://github.com/dtolnay/prettyplease/tree/6f7a9eebd7052fd5c37a84135e1daa7599176e7e //
//                                                                                       //
// David Tolnay provided his work under the following terms:                             //
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

use crate::{
    format::PrintString,
    lexis::Length,
    report::{debug_unreachable, system_panic},
    std::*,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PrettyPrintConfig {
    pub margin: u16,
    pub inline: u16,
    pub indent: u16,
    pub debug: bool,
}

impl Default for PrettyPrintConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl PrettyPrintConfig {
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

    #[inline(always)]
    pub fn ibox(&mut self, ident: isize) {
        let step = self.step;

        self.scan_begin(Group {
            mode: Mode::Inconsistent,
            indent: step * ident,
        });
    }

    #[inline(always)]
    pub fn cbox(&mut self, ident: isize) {
        let step = self.step;

        self.scan_begin(Group {
            mode: Mode::Consistent,
            indent: step * ident,
        });
    }

    #[inline(always)]
    pub fn end(&mut self) {
        self.scan_end();
    }

    #[inline(always)]
    pub fn word(&mut self, word: impl Into<String>) {
        self.scan_string(word.into());
    }

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

    #[inline(always)]
    pub fn indent(&mut self, indent: isize) {
        let offset = self.step * indent;

        if let Some(blank) = self.blank_token() {
            blank.indent = offset;
        }
    }

    #[inline(always)]
    pub fn pre_break(&mut self, string: impl Into<String>) {
        if let Some(blank) = self.blank_token() {
            blank.pre_break = Some(string.into());
        }
    }

    #[inline(always)]
    pub fn pre_space(&mut self, string: impl Into<String>) {
        if let Some(blank) = self.blank_token() {
            blank.pre_space = Some(string.into());
        }
    }

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
                None => unsafe { debug_unreachable!("Empty scan queue.") },

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
