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
    format::{PrintString, Style},
    lexis::{
        Column,
        Length,
        Line,
        Position,
        PositionSpan,
        Site,
        SiteSpan,
        SourceCode,
        ToSite,
        ToSpan,
        Token,
        TokenBuffer,
    },
    report::debug_unreachable,
    std::*,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub struct SnippetConfig {
    pub show_numbers: bool,
    pub draw_frame: bool,
    pub dim_code: bool,
    pub ascii_drawing: bool,
    pub style: bool,
    pub caption: bool,
    pub summary: bool,
}

impl Default for SnippetConfig {
    #[inline(always)]
    fn default() -> Self {
        Self::verbose()
    }
}

impl SnippetConfig {
    #[inline(always)]
    pub const fn verbose() -> Self {
        Self {
            show_numbers: true,
            draw_frame: true,
            dim_code: true,
            ascii_drawing: false,
            style: true,
            caption: true,
            summary: true,
        }
    }

    #[inline(always)]
    pub const fn minimal() -> Self {
        Self {
            show_numbers: false,
            draw_frame: false,
            dim_code: false,
            ascii_drawing: false,
            style: false,
            caption: false,
            summary: false,
        }
    }

    #[inline(always)]
    fn cover(&self) -> usize {
        2
    }

    #[inline(always)]
    fn continuation(&self) -> usize {
        3
    }

    #[inline(always)]
    fn margin(&self) -> Length {
        80
    }

    #[inline(always)]
    fn code_style(&self, dim: bool) -> Style {
        match self.style && self.dim_code && dim {
            false => Style::default(),
            true => Style::default().bright_black(),
        }
    }

    #[inline(always)]
    fn annotation_style(&self, priority: Priority) -> Style {
        if !self.style {
            return Style::default();
        }

        match priority {
            Priority::Default => Style::default().underline(),
            Priority::Primary => Style::default().bold().red(),
            Priority::Secondary => Style::default().bold().blue(),
            Priority::Note => Style::default().bold().yellow(),
        }
    }

    #[inline(always)]
    fn control(&self) -> char {
        match self.ascii_drawing {
            true => ' ',
            false => '💻',
        }
    }

    #[inline(always)]
    fn placeholder(&self) -> char {
        match self.ascii_drawing {
            true => ' ',
            false => '▒',
        }
    }

    #[inline(always)]
    fn etc(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("...");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("…");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn tab(&self) -> &'static PrintString<'static> {
        static STRING: PrintString<'static> = PrintString::borrowed("    ");

        &STRING
    }

    #[inline(always)]
    fn box_vertical(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("|");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("│");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn box_horizontal(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("-");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("─");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn box_top_left(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed(" ");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("╭");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn box_top_right(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("╮");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn box_bottom_left(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed(" ");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("╰");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn box_bottom_right(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("╯");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn box_middle_delimiter(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("|=");
        static ASCII_ALONE: PrintString<'static> = PrintString::borrowed("|==");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("╞═");
        static NON_ASCII_ALONE: PrintString<'static> = PrintString::borrowed("╞══");

        match (self.ascii_drawing, self.draw_frame) {
            (true, true) => &ASCII,
            (true, false) => &ASCII_ALONE,
            (false, true) => &NON_ASCII,
            (false, false) => &NON_ASCII_ALONE,
        }
    }

    #[inline(always)]
    fn box_middle_left(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("|");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("├");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn box_middle_right(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("|");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("┤");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn caption_start(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("-[ ");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("─╢ ");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn caption_end(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed(" ]");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed(" ╟");
        static NON_ASCII_ALONE: PrintString<'static> = PrintString::borrowed(" ║");

        match self.ascii_drawing {
            true => &ASCII,
            false => match self.draw_frame {
                true => &NON_ASCII,
                false => &NON_ASCII_ALONE,
            },
        }
    }

    #[inline(always)]
    fn arrow_up_right(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("|- ");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("╭╴ ");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn arrow_down_right(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("|- ");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("╰╴ ");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }

    #[inline(always)]
    fn arrow_down_middle(&self) -> &'static PrintString<'static> {
        static ASCII: PrintString<'static> = PrintString::borrowed("|");
        static NON_ASCII: PrintString<'static> = PrintString::borrowed("│");

        match self.ascii_drawing {
            true => &ASCII,
            false => &NON_ASCII,
        }
    }
}

pub trait SnippetFormatter<'f> {
    fn snippet<'a, C: SourceCode>(&'a mut self, code: &'a C) -> Snippet<'a, 'f, C>;
}

impl<'f> SnippetFormatter<'f> for Formatter<'f> {
    #[inline(always)]
    fn snippet<'a, C: SourceCode>(&'a mut self, code: &'a C) -> Snippet<'a, 'f, C> {
        static VERBOSE: SnippetConfig = SnippetConfig::verbose();
        static MINIMAL: SnippetConfig = SnippetConfig::minimal();

        let config = match self.alternate() {
            true => &VERBOSE,
            false => &MINIMAL,
        };

        Snippet {
            formatter: self,
            code,
            config,
            caption: PrintString::empty(),
            summary: PrintString::empty(),
            highlighter: None,
            annotations: Vec::with_capacity(4),
        }
    }
}

pub struct Snippet<'a, 'f, C: SourceCode> {
    formatter: &'a mut Formatter<'f>,
    code: &'a C,
    config: &'a SnippetConfig,
    caption: PrintString<'a>,
    summary: PrintString<'a>,
    highlighter: Option<Box<dyn Highlighter<C::Token> + 'a>>,
    annotations: Vec<Annotation<'a>>,
}

impl<'a, 'f, C: SourceCode> Snippet<'a, 'f, C> {
    #[inline(always)]
    pub fn set_config(&mut self, config: &'a SnippetConfig) -> &mut Self {
        self.config = config;

        self
    }

    #[inline(always)]
    pub fn set_caption(&mut self, caption: impl Into<PrintString<'a>>) -> &mut Self {
        let caption = caption.into();

        if caption.as_ref().contains('\n') {
            panic!("Multiline captions not supported.");
        }

        self.caption = caption;

        self
    }

    #[inline(always)]
    pub fn set_summary(&mut self, summary: impl Into<PrintString<'a>>) -> &mut Self {
        self.summary = summary.into();

        self
    }

    #[inline(always)]
    pub fn set_highlighter(&mut self, highlighter: impl Highlighter<C::Token> + 'a) -> &mut Self {
        self.highlighter = Some(Box::new(highlighter));

        self
    }

    pub fn annotate(
        &mut self,
        span: impl ToSpan,
        priority: Priority,
        message: impl Into<PrintString<'a>>,
    ) -> &mut Self {
        let message = message.into();

        if message.as_ref().contains('\n') {
            panic!("Multiline annotation messages not supported.");
        }

        let span = match span.to_site_span(self.code) {
            Some(span) => span,

            None => panic!("Invalid annotation span."),
        };

        self.annotations.push(Annotation {
            span,
            priority,
            message,
        });

        self
    }

    pub fn finish(&mut self) -> FmtResult {
        // PREPARE

        let (cover, mut lines) = self.scan();

        let mut code_length = 0;

        for print_line in &mut lines {
            for string in &print_line.before {
                code_length = code_length.max(string.length);
            }

            code_length = code_length.max(print_line.code.length);

            for string in &print_line.after {
                code_length = code_length.max(string.length);
            }
        }

        let caption = match self.config.caption {
            false => StyleString::empty(),
            true => StyleString::from_str(self.config, self.caption.as_ref()),
        };

        let summary = match self.config.summary {
            false => Vec::new(),

            true => {
                let mut summary = Vec::with_capacity(4);

                if !self.summary.is_empty() {
                    for summary_line in self.summary.as_ref().lines() {
                        summary.push(StyleString::from_str(self.config, summary_line));
                    }
                }

                summary
            }
        };

        if self.config.draw_frame && caption.length > 0 {
            code_length = code_length.max(
                caption.length
                    + self.config.caption_start().length()
                    + self.config.caption_end().length(),
            );
        }

        if self.config.draw_frame && !self.summary.is_empty() {
            for summary_line in &summary {
                code_length = code_length.max(summary_line.length);
            }
        }

        let numbers_length = (cover.end.line.checked_ilog10().unwrap_or(0) as usize + 1)
            .max(self.config.etc().length());

        let mut margin: usize = self.config.margin();

        if self.config.draw_frame {
            margin = margin
                .checked_sub(2 + self.config.box_vertical().length() * 2)
                .unwrap_or_default();
        }

        if self.config.show_numbers {
            margin = margin.checked_sub(numbers_length + 2).unwrap_or_default();
        }

        code_length = code_length.max(margin);

        // RENDER

        let dim = !self.annotations.is_empty();
        let has_caption = caption.length > 0;
        let has_summary = !summary.is_empty();
        let mut is_first = true;

        if self.config.draw_frame || has_caption || has_summary {
            StyleString::start(is_first)
                .with_header_blank(self.config, numbers_length)
                .with_caption(self.config, code_length, caption)
                .end(&mut is_first, self.formatter)?;
        }

        let mut back_distance: usize = 0;
        let mut skip = false;
        let mut distances = Vec::with_capacity(lines.len());

        for line in lines.iter().rev() {
            match line.annotated {
                false => back_distance += 1,
                true => back_distance = 0,
            }

            distances.push(back_distance);
        }

        back_distance = 0;

        for (forward_distance, line) in distances.into_iter().rev().zip(lines) {
            if line.annotated || !self.config.show_numbers || !dim {
                back_distance = 0;
                skip = false;

                for string in line.before {
                    StyleString::start(is_first)
                        .with_header_blank(self.config, numbers_length)
                        .with_code(
                            self.config,
                            dim,
                            has_caption,
                            has_summary,
                            code_length,
                            string,
                        )
                        .end(&mut is_first, self.formatter)?;
                }

                StyleString::start(is_first)
                    .with_header_number(self.config, numbers_length, line.number)
                    .with_code(
                        self.config,
                        dim,
                        has_caption,
                        has_summary,
                        code_length,
                        line.code,
                    )
                    .end(&mut is_first, self.formatter)?;

                for string in line.after {
                    StyleString::start(is_first)
                        .with_header_blank(self.config, numbers_length)
                        .with_code(
                            self.config,
                            dim,
                            has_caption,
                            has_summary,
                            code_length,
                            string,
                        )
                        .end(&mut is_first, self.formatter)?;
                }

                continue;
            }

            back_distance += 1;

            let min_distance = forward_distance.min(back_distance);

            if skip {
                match min_distance <= self.config.cover() {
                    true => skip = false,
                    false => continue,
                }
            }

            if min_distance > self.config.cover() {
                if forward_distance >= self.config.continuation() {
                    StyleString::start(is_first)
                        .with_header_etc(self.config, numbers_length)
                        .with_code_blank(self.config, dim, has_caption, has_summary, code_length)
                        .end(&mut is_first, self.formatter)?;
                    skip = true;
                    continue;
                }
            }

            StyleString::start(is_first)
                .with_header_number(self.config, numbers_length, line.number)
                .with_code(
                    self.config,
                    dim,
                    has_caption,
                    has_summary,
                    code_length,
                    line.code,
                )
                .end(&mut is_first, self.formatter)?;
        }

        if has_summary {
            StyleString::start(is_first)
                .with_header_blank(self.config, numbers_length)
                .with_delimiter(self.config, code_length)
                .end(&mut is_first, self.formatter)?;

            for summary in summary {
                StyleString::start(is_first)
                    .with_header_blank(self.config, numbers_length)
                    .with_summary(self.config, code_length, summary)
                    .end(&mut is_first, self.formatter)?;
            }
        }

        if self.config.draw_frame || has_caption || has_summary {
            StyleString::start(is_first)
                .with_header_blank(self.config, numbers_length)
                .with_footer(self.config, code_length)
                .end(&mut is_first, self.formatter)?;
        }

        Ok(())
    }

    fn scan(&mut self) -> (PositionSpan, Vec<ScanLine>) {
        struct Scanner {
            position_cover: PositionSpan,
            site_cover: SiteSpan,
            buffer: Vec<ScanLine>,
            empty: bool,
            line: Line,
            pending: ScanLine,
            stack: Vec<usize>,
        }

        impl Scanner {
            fn new<C: SourceCode>(snippet: &Snippet<C>) -> Self {
                let position_cover = snippet
                    .annotations
                    .iter()
                    .map(|annotation| annotation.span.clone())
                    .reduce(|a, b| a.start.min(b.start)..a.end.max(b.end))
                    .map(|cover| {
                        let mut cover = match cover.to_position_span(snippet.code) {
                            Some(span) => span,

                            // Safety: Site spans are always valid to resolve.
                            None => unsafe { debug_unreachable!("Invalid site span.") },
                        };

                        cover.start.line = cover
                            .start
                            .line
                            .checked_sub(snippet.config.cover())
                            .unwrap_or(1)
                            .max(1);

                        cover.start.column = 1;

                        cover.end.line = cover
                            .end
                            .line
                            .checked_add(snippet.config.cover())
                            .unwrap_or(usize::MAX);
                        cover.end.column = Column::MAX;

                        cover
                    })
                    .unwrap_or_else(|| {
                        let end = match Site::MAX.to_position(snippet.code) {
                            Some(mut position) => {
                                position.column = usize::MAX;

                                position
                            }

                            // Safety: Sites are always valid to resolve.
                            None => unsafe { debug_unreachable!("Invalid end site.") },
                        };

                        Position::default()..end
                    });

                let buffer =
                    Vec::with_capacity(position_cover.end.line - position_cover.start.line + 1);
                let line = position_cover.start.line;
                let pending = ScanLine::new(line);
                let stack = Vec::with_capacity(snippet.annotations.len());
                let site_cover = match position_cover.to_site_span(snippet.code) {
                    Some(span) => span,
                    // Safety: Position spans are always valid to resolve.
                    None => unsafe { debug_unreachable!("Invalid position span.") },
                };

                Self {
                    position_cover,
                    site_cover,
                    buffer,
                    empty: true,
                    line,
                    pending,
                    stack,
                }
            }

            #[inline(always)]
            fn submit(&mut self, config: &SnippetConfig) {
                self.line += 1;

                let mut pending = replace(&mut self.pending, ScanLine::new(self.line));

                pending.expand(config);

                pending
                    .messages
                    .sort_by_key(|message| message.priority.order());

                self.buffer.push(pending);
            }

            #[inline(always)]
            fn top(&self) -> Option<usize> {
                self.stack.last().copied()
            }
        }

        let mut scanner = Scanner::new(self);

        let dim = !self.annotations.is_empty();

        let code_style = self.config.code_style(dim);
        let mut token_style = None;

        'chunk_loop: for chunk in self.code.chunks(&scanner.site_cover) {
            let mut site = chunk.site;

            if self.config.style {
                if let Some(highlighter) = &mut self.highlighter {
                    token_style = Some(highlighter.token_style(dim, chunk.token))
                }
            }

            for ch in chunk.string.chars() {
                if site < scanner.site_cover.start {
                    site += 1;
                    continue;
                }

                for (index, annotation) in self.annotations.iter().enumerate() {
                    if annotation.span.end != site {
                        continue;
                    }

                    scanner.stack.retain(|item| *item != index);
                }

                for (index, annotation) in self.annotations.iter().enumerate() {
                    if annotation.span.start != site {
                        continue;
                    }

                    if !annotation.message.is_empty() {
                        scanner
                            .pending
                            .messages
                            .push(annotation.message(self.config, scanner.pending.code.length));
                    }

                    match annotation.span.end == site {
                        true => {
                            scanner.pending.code.style =
                                self.config.annotation_style(annotation.priority);
                            scanner.pending.code.write_placeholder(self.config);
                            scanner.pending.annotated = true;
                        }

                        false => {
                            if ch == '\n' {
                                scanner.pending.code.style =
                                    self.config.annotation_style(annotation.priority);
                                scanner.pending.code.write_placeholder(self.config);
                            }

                            scanner.stack.push(index);
                        }
                    }
                }

                scanner.pending.code.style = match scanner.top() {
                    None => token_style.flatten().unwrap_or(code_style),

                    Some(top) => {
                        let priority = match self.annotations.get(top) {
                            Some(annotation) => annotation.priority,

                            // Safety: Annotation stack is well-formed.
                            None => unsafe { debug_unreachable!("Missing annotation.") },
                        };

                        scanner.pending.annotated = true;

                        self.config.annotation_style(priority)
                    }
                };

                scanner.empty = false;

                match ch {
                    '\n' => scanner.submit(self.config),
                    '\t' => scanner.pending.code.write_tab(self.config),
                    _ => scanner.pending.code.write_code_char(self.config, ch),
                }

                site += 1;

                if site >= scanner.site_cover.end {
                    break 'chunk_loop;
                }
            }
        }

        for annotation in self.annotations.iter() {
            if annotation.span.start != scanner.site_cover.end {
                continue;
            }

            if !annotation.span.is_empty() {
                continue;
            }

            if !annotation.message.is_empty() {
                scanner
                    .pending
                    .messages
                    .push(annotation.message(self.config, scanner.pending.code.length));
            }

            scanner.pending.annotated = true;
            scanner.pending.code.style = self.config.annotation_style(annotation.priority);
            scanner.pending.code.write_placeholder(self.config);

            scanner.empty = false;
        }

        if !scanner.empty {
            scanner.submit(self.config);
        }

        (scanner.position_cover, scanner.buffer)
    }
}

pub trait Highlighter<T: Token> {
    fn token_style(&mut self, dim: bool, token: T) -> Option<Style>;
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Priority {
    #[default]
    Default,
    Primary,
    Secondary,
    Note,
}

impl Priority {
    #[inline(always)]
    fn order(&self) -> usize {
        match self {
            Self::Primary => 1,
            Self::Secondary => 2,
            Self::Note => 3,
            Self::Default => 4,
        }
    }
}

struct ScanLine {
    number: Line,
    before: Vec<StyleString>,
    code: StyleString,
    after: Vec<StyleString>,
    messages: Vec<Message>,
    annotated: bool,
}

impl ScanLine {
    #[inline(always)]
    fn new(number: Line) -> Self {
        Self {
            number,
            before: Vec::new(),
            code: StyleString::new(),
            after: Vec::new(),
            messages: Vec::new(),
            annotated: false,
        }
    }

    fn expand(&mut self, config: &SnippetConfig) {
        enum Segment {
            End(Message),
            Middle { offset: Column, priority: Priority },
        }

        impl Segment {
            #[inline(always)]
            fn span(&self, config: &SnippetConfig) -> SiteSpan {
                match self {
                    Self::Middle { offset, .. } => Message::span_down_middle(config, *offset),
                    Self::End(message) => message.span_down_right(config),
                }
            }
        }

        let mut pending = take(&mut self.messages)
            .into_iter()
            .map(|message| Some(message))
            .collect::<Vec<_>>();

        let mut left = pending.len();

        if left > 1 {
            for message in pending.iter_mut() {
                if let Some(message) = message {
                    if message.priority == Priority::Primary {
                        continue;
                    }
                }

                let message = match take(message) {
                    Some(message) => message,

                    // Safety: All messages initialized in the beginning.
                    None => unsafe { debug_unreachable!("Unset first message.") },
                };

                left -= 1;

                let mut string = StyleString::new();

                string.style = config.code_style(true);
                string.write_blanks(message.offset);

                string.style = config.annotation_style(message.priority).no_emphasis();
                string.write_sanitized(config.arrow_up_right());

                string.style = Style::new();
                string.append(message.string);

                self.before.push(string);

                break;
            }
        }

        let mut segments = Vec::<Segment>::with_capacity(left);

        while left > 0 {
            'outer: for pending in pending.iter_mut().rev() {
                let message = match pending {
                    Some(pending) => pending,
                    None => continue,
                };

                let mut index = 0;

                let span = message.span_down_right(config);

                for probe in segments.iter() {
                    let probe_span = probe.span(config);

                    if span.end > probe_span.start && span.start < probe_span.end {
                        continue 'outer;
                    }

                    if span.start >= probe_span.end {
                        index += 1;
                        continue;
                    }

                    break;
                }

                let segment = match take(pending) {
                    Some(message) => Segment::End(message),

                    // Safety: Discriminant checked above.
                    None => unsafe { debug_unreachable!("Missing pending item.") },
                };

                left -= 1;

                segments.insert(index, segment);
            }

            'outer: for message in pending.iter().flatten() {
                let mut index = 0;

                let span = Message::span_down_middle(config, message.offset);

                for probe in segments.iter() {
                    let probe_span = probe.span(config);

                    if span.end > probe_span.start && span.start < probe_span.end {
                        continue 'outer;
                    }

                    if span.start >= probe_span.end {
                        index += 1;
                        continue;
                    }

                    break;
                }

                segments.insert(
                    index,
                    Segment::Middle {
                        offset: message.offset,
                        priority: message.priority,
                    },
                );
            }

            let mut string = StyleString::new();

            let mut cursor = 0;
            for segment in replace(&mut segments, Vec::with_capacity(left)) {
                match segment {
                    Segment::Middle { offset, priority } => {
                        string.style = config.code_style(true);
                        string.write_blanks(offset - cursor);

                        let drawing = config.arrow_down_middle();

                        string.style = config.annotation_style(priority).no_emphasis();
                        string.write_sanitized(&drawing);

                        cursor = offset + drawing.length();
                    }

                    Segment::End(message) => {
                        let span = message.span_down_right(config);

                        string.style = config.code_style(true);
                        string.write_blanks(span.start - cursor);

                        string.style = config.annotation_style(message.priority).no_emphasis();
                        string.write_sanitized(config.arrow_down_right());

                        string.append(message.string);

                        cursor = span.end;
                    }
                }
            }

            self.after.push(string);
        }
    }
}

struct Annotation<'a> {
    span: SiteSpan,
    priority: Priority,
    message: PrintString<'a>,
}

impl<'a> Annotation<'a> {
    #[inline(always)]
    fn message(&self, config: &SnippetConfig, offset: Column) -> Message {
        Message {
            offset,
            priority: self.priority,
            string: StyleString::from_str(config, self.message.as_ref()),
        }
    }
}

struct Message {
    offset: Column,
    priority: Priority,
    string: StyleString,
}

impl Message {
    #[inline(always)]
    #[allow(unused)]
    fn span_up_right(&self, config: &SnippetConfig) -> SiteSpan {
        let drawing = config.arrow_up_right();

        self.offset..(self.offset + drawing.length() + self.string.length)
    }

    #[inline(always)]
    fn span_down_right(&self, config: &SnippetConfig) -> SiteSpan {
        let drawing = config.arrow_down_right();

        self.offset..(self.offset + drawing.length() + self.string.length)
    }

    #[inline(always)]
    fn span_down_middle(config: &SnippetConfig, offset: Column) -> SiteSpan {
        let drawing = config.arrow_down_middle();

        offset..(offset + drawing.length())
    }
}

struct StyleString {
    text: String,
    length: Length,
    start_style: Style,
    end_style: Style,
    style: Style,
}

impl Display for StyleString {
    #[inline(always)]
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str(&self.text)
    }
}

impl StyleString {
    #[inline(always)]
    fn new() -> Self {
        Self {
            text: String::with_capacity(120),
            length: 0,
            start_style: Style::new(),
            end_style: Style::new(),
            style: Style::new(),
        }
    }

    #[inline(always)]
    fn empty() -> Self {
        Self {
            text: String::new(),
            length: 0,
            start_style: Style::new(),
            end_style: Style::new(),
            style: Style::new(),
        }
    }

    fn from_str(config: &SnippetConfig, source: impl AsRef<str>) -> Self {
        #[derive(Clone, Copy, PartialEq, Eq, Token)]
        #[repr(u8)]
        enum Escaped {
            EOI = 0,
            Text = 1,
            #[rule("\x1B[" ['\x30'..'\x4F']* ['\x20'..'\x2F']* ['\x40'..'\x7E'])]
            CSI,
        }

        let source = source.as_ref();

        let mut target = Self::new();

        let buffer = TokenBuffer::from(source);

        for chunk in buffer.chunks(..) {
            match chunk.token {
                Escaped::CSI => {
                    if !config.style {
                        continue;
                    }
                }
                _ => target.length += chunk.length,
            }

            target.text.push_str(chunk.string);
        }

        target
    }

    #[inline]
    fn start(is_first: bool) -> Self {
        let mut string = Self::new();

        if !is_first {
            string.text.push('\n');
        }

        string
    }

    fn with_header_blank(self, config: &SnippetConfig, alignment: Length) -> Self {
        self.with_header(config, alignment, "")
    }

    fn with_header_etc(self, config: &SnippetConfig, alignment: Length) -> Self {
        self.with_header(config, alignment, config.etc().as_ref())
    }

    fn with_header_number(self, config: &SnippetConfig, alignment: Length, number: Line) -> Self {
        self.with_header(config, alignment, number.to_string().as_str())
    }

    #[inline]
    fn with_header(mut self, config: &SnippetConfig, alignment: Length, text: &str) -> Self {
        if !config.show_numbers {
            return self;
        }

        self.write_blanks(1);
        self.write_sanitized(&PrintString::owned(format!("{: >1$}", text, alignment)));
        self.write_blanks(1);

        self
    }

    fn with_caption(
        mut self,
        config: &SnippetConfig,
        mut alignment: Length,
        caption: Self,
    ) -> Self {
        self.write_sanitized(config.box_top_left());
        self.write_sanitized(config.box_horizontal());

        let has_caption = caption.length > 0;

        if has_caption {
            alignment -= config.caption_start().length();
            self.write_sanitized(config.caption_start());

            alignment -= caption.length;
            self.append(caption);

            alignment -= config.caption_end().length();
            self.write_sanitized(config.caption_end());
        }

        match config.draw_frame {
            true => {
                self.repeat_sanitized(config.box_horizontal(), alignment + 1);
                self.write_sanitized(config.box_top_right());
            }

            false => {
                if !has_caption {
                    self.write_sanitized(config.box_horizontal());
                }
            }
        }

        self
    }

    fn with_code(
        mut self,
        config: &SnippetConfig,
        dim: bool,
        has_caption: bool,
        has_summary: bool,
        mut alignment: Length,
        code: Self,
    ) -> Self {
        let code_style = config.code_style(dim);

        if config.draw_frame || config.show_numbers || has_caption || has_summary {
            self.write_sanitized(config.box_vertical());
            self.style = code_style;
            self.write_blanks(1);
        }

        alignment -= code.length;
        self.append(code);

        if config.draw_frame {
            self.style = code_style;
            self.write_blanks(alignment + 1);

            self.style = Style::new();
            self.write_sanitized(config.box_vertical());
        }

        self
    }

    fn with_code_blank(
        mut self,
        config: &SnippetConfig,
        dim: bool,
        has_caption: bool,
        has_summary: bool,
        alignment: Length,
    ) -> Self {
        if config.draw_frame || config.show_numbers || has_caption || has_summary {
            self.write_sanitized(config.box_vertical());
        }

        if config.draw_frame {
            self.style = config.code_style(dim);
            self.write_blanks(alignment + 2);

            self.style = Style::new();
            self.write_sanitized(config.box_vertical());
        }

        if self.length == 0 {
            self.length = 1;
        }

        self
    }

    fn with_delimiter(mut self, config: &SnippetConfig, alignment: Length) -> Self {
        match config.draw_frame {
            true => {
                self.write_sanitized(config.box_middle_left());
                self.repeat_sanitized(config.box_horizontal(), alignment + 2);
                self.write_sanitized(config.box_middle_right());
            }

            false => {
                self.write_sanitized(config.box_middle_delimiter());
            }
        }

        self
    }

    fn with_summary(
        mut self,
        config: &SnippetConfig,
        mut alignment: Length,
        summary: Self,
    ) -> Self {
        self.write_sanitized(config.box_vertical());
        self.write_blanks(1);

        alignment -= summary.length;
        self.append(summary);

        if config.draw_frame {
            self.write_blanks(alignment + 1);
            self.write_sanitized(config.box_vertical());
        }

        self
    }

    fn with_footer(mut self, config: &SnippetConfig, alignment: Length) -> Self {
        self.write_sanitized(config.box_bottom_left());
        self.write_sanitized(config.box_horizontal());

        match config.draw_frame {
            true => {
                self.repeat_sanitized(config.box_horizontal(), alignment + 1);
                self.write_sanitized(config.box_bottom_right());
            }

            false => {
                self.write_sanitized(config.box_horizontal());
            }
        }

        self
    }

    #[inline]
    fn end(mut self, is_first: &mut bool, formatter: &mut Formatter) -> FmtResult {
        if self.length == 0 {
            return Ok(());
        }

        *is_first = false;

        self.style = Style::new();
        self.submit_style();

        Display::fmt(&self, formatter)
    }

    #[inline(always)]
    fn write_code_char(&mut self, config: &SnippetConfig, mut ch: char) {
        if ch.is_control() {
            ch = config.control();
        }

        self.submit_style();

        self.text.push(ch);
        self.length += 1;
    }

    #[inline(always)]
    fn write_sanitized(&mut self, string: &PrintString) {
        self.submit_style();

        self.text.push_str(string.as_ref());
        self.length += string.length();
    }

    #[inline(always)]
    fn repeat_sanitized(&mut self, string: &PrintString, mut count: usize) {
        if count == 0 {
            return;
        }

        self.submit_style();

        while count > 0 {
            self.text.push_str(string.as_ref());
            self.length += string.length();
            count -= 1;
        }
    }

    #[inline(always)]
    fn write_placeholder(&mut self, config: &SnippetConfig) {
        self.submit_style();

        self.text.push(config.placeholder());
        self.length += 1;
    }

    #[inline(always)]
    fn write_tab(&mut self, config: &SnippetConfig) {
        self.write_sanitized(config.tab());
    }

    #[inline(always)]
    fn write_blanks(&mut self, count: Length) {
        if count == 0 {
            return;
        }

        self.submit_style();

        self.text.extend(repeat(' ').take(count));
        self.length += count;
    }

    fn append(&mut self, other: StyleString) {
        self.style = other.start_style;

        if !other.text.is_empty() {
            self.submit_style();
            self.text.push_str(other.text.as_str());
        }

        self.length += other.length;

        self.end_style = other.end_style;
        self.style = other.style;
    }

    fn submit_style(&mut self) {
        if self.end_style == self.style {
            return;
        }

        Style::change(&self.end_style, &self.style, &mut self.text);

        self.end_style = self.style;

        if self.length == 0 {
            self.start_style = self.end_style;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        format::{snippet::StyleString, SnippetConfig, Style, TerminalString},
        std::*,
    };

    #[test]
    fn test_csi_detection() {
        let string = StyleString::from_str(&SnippetConfig::verbose(), "hello world");
        assert_eq!(string.length, 11);

        let string = StyleString::from_str(
            &SnippetConfig::verbose(),
            &format!("hello{}world", " ".apply(Style::new())),
        );
        assert_eq!(string.length, 11);
        assert_eq!(string.text.len(), 11);

        let string = StyleString::from_str(
            &SnippetConfig::verbose(),
            &format!("hello{}world", " ".apply(Style::new().bold())),
        );
        assert_eq!(string.length, 11);
        assert_ne!(string.text.len(), 11);

        let string = StyleString::from_str(
            &SnippetConfig::minimal(),
            &format!("hello{}world", " ".apply(Style::new().bold())),
        );
        assert_eq!(string.length, 11);
        assert_eq!(string.text.len(), 11);
    }
}
