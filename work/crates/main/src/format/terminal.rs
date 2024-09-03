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

use std::cmp::Ordering;

use crate::lexis::{SourceCode, Token, TokenBuffer};

macro_rules! escape {
    ($($code:expr)?) => {
        concat!("\x1B[", $( $code, )? "m")
    };
}

/// A configuration of
/// the [CSI](https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences)
/// style sequence.
///
/// In particular, through this object, you can specify text background and
/// foreground colors, and text emphasis such as bold, italic, underlined or
/// inverted style.
///
/// The Style API implemented as a builder to be used in a call-chain style,
/// such as each function consumes the instance of this object and returns
/// a new instance with the applied configuration option.
///
/// Since Style methods are const functions, you can construct and store an
/// instance of Style in static.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    emphasis: Emphasis,
}

impl Default for Style {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    /// Creates an instance of Style without any style configurations.
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            emphasis: Emphasis::none(),
        }
    }

    /// Sets the text foreground color to Black.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn black(self) -> Self {
        self.fg(Color::Black)
    }

    /// Sets the text foreground color to Red.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn red(self) -> Self {
        self.fg(Color::Red)
    }

    /// Sets the text foreground color to Green.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn green(self) -> Self {
        self.fg(Color::Green)
    }

    /// Sets the text foreground color to Yellow.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn yellow(self) -> Self {
        self.fg(Color::Yellow)
    }

    /// Sets the text foreground color to Blue.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn blue(self) -> Self {
        self.fg(Color::Blue)
    }

    /// Sets the text foreground color to Magenta.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn magenta(self) -> Self {
        self.fg(Color::Magenta)
    }

    /// Sets the text foreground color to Cyan.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn cyan(self) -> Self {
        self.fg(Color::Cyan)
    }

    /// Sets the text foreground color to White.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn white(self) -> Self {
        self.fg(Color::White)
    }

    /// Sets the text foreground color to Bright Black.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_black(self) -> Self {
        self.fg(Color::BrightBlack)
    }

    /// Sets the text foreground color to Bright Red.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_red(self) -> Self {
        self.fg(Color::BrightRed)
    }

    /// Sets the text foreground color to Bright Green.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_green(self) -> Self {
        self.fg(Color::BrightGreen)
    }

    /// Sets the text foreground color to Bright Yellow.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_yellow(self) -> Self {
        self.fg(Color::BrightYellow)
    }

    /// Sets the text foreground color to Bright Blue.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_blue(self) -> Self {
        self.fg(Color::BrightBlue)
    }

    /// Sets the text foreground color to Bright Magenta.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_magenta(self) -> Self {
        self.fg(Color::BrightMagenta)
    }

    /// Sets the text foreground color to Bright Cyan.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_cyan(self) -> Self {
        self.fg(Color::BrightCyan)
    }

    /// Sets the text foreground color to Bright White.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn bright_white(self) -> Self {
        self.fg(Color::BrightWhite)
    }

    /// Sets the text foreground color to 8-bit RGB color.
    ///
    /// The scale of each parameter is from 0.0 to 1.0. Values outside of
    /// this range will be clamped. The final 8-bit representation of the color
    /// will be inferred to be as close to the floating-point components
    /// representation as possible.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn rgb(self, red: f64, green: f64, blue: f64) -> Self {
        self.fg(Color::RGB { red, green, blue })
    }

    /// Sets the text foreground color to 8-bit Grayscale color.
    ///
    /// The scale of the `shade` parameter is from 0.0 to 1.0. Values outside of
    /// this range will be clamped. The final 8-bit representation of
    /// the shade will be inferred to be as close to the floating-point shade
    /// as possible.
    ///
    /// See [Ansi Color Table](https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit)
    /// for details.
    #[inline(always)]
    pub const fn grayscale(self, shade: f64) -> Self {
        self.fg(Color::Grayscale(shade))
    }

    /// Enables bold emphasis of the text.
    #[inline(always)]
    pub const fn bold(mut self) -> Self {
        self.emphasis.bold = true;

        self
    }

    /// Enables italic emphasis of the text.
    #[inline(always)]
    pub const fn italic(mut self) -> Self {
        self.emphasis.italic = true;

        self
    }

    /// Enables underlined emphasis of the text.
    #[inline(always)]
    pub const fn underline(mut self) -> Self {
        self.emphasis.underline = true;

        self
    }

    /// Enables inverted emphasis of the text.
    #[inline(always)]
    pub const fn invert(mut self) -> Self {
        self.emphasis.invert = true;

        self
    }

    /// Sets the text foreground color.
    #[inline(always)]
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);

        self
    }

    /// Sets the text background color.
    #[inline(always)]
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);

        self
    }

    pub(super) fn change(from: &Self, to: &Self, target: &mut String) {
        if Emphasis::change(&from.emphasis, &to.emphasis, target) {
            match (&from.fg, &to.fg) {
                (Some(_), None) => Color::reset_fg(target),
                (None, Some(to)) => to.apply_fg(target),
                (Some(from), Some(to)) if from != to => to.apply_fg(target),
                _ => (),
            }

            match (&from.bg, &to.bg) {
                (Some(_), None) => Color::reset_bg(target),
                (None, Some(to)) => to.apply_bg(target),
                (Some(from), Some(to)) if from != to => to.apply_bg(target),
                _ => (),
            }

            return;
        }

        if let Some(to) = &to.fg {
            to.apply_fg(target);
        }

        if let Some(to) = &to.bg {
            to.apply_bg(target);
        }
    }

    #[inline(always)]
    pub(super) fn no_emphasis(mut self) -> Self {
        self.emphasis = Emphasis::none();

        self
    }
}

/// An extension of a string with functions that apply or erase
/// [CSI](https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_(Control_Sequence_Introducer)_sequences)
/// style sequences.
///
/// This trait is auto-implemented for any object which is `AsRef<str>`.
pub trait TerminalString: AsRef<str> {
    /// Returns a new string from this one by surrounding it with CSI sequences
    /// that apply the specified `style` at the beginning of the string and
    /// erase these styles in the end of the string.
    fn apply(&self, style: Style) -> String {
        let source = self.as_ref();
        let mut target = String::with_capacity(source.len() + 20);

        if let Some(color) = &style.fg {
            color.apply_fg(&mut target);
        }

        if let Some(color) = &style.bg {
            color.apply_bg(&mut target);
        }

        style.emphasis.apply(&mut target);

        target.push_str(source);

        if style.fg.is_some() || style.bg.is_some() || style.emphasis.is_some() {
            reset_all(&mut target);
        }

        target
    }

    /// Returns a new string from this one, removing any valid CSI sequence from
    /// the string content.
    fn sanitize(&self) -> String {
        let mut target = String::with_capacity(self.as_ref().len());

        let buffer = TokenBuffer::<Escaped>::from(self);

        for chunk in buffer.chunks(..) {
            if chunk.token != Escaped::Text {
                continue;
            }

            target.push_str(chunk.string);
        }

        target
    }
}

impl<S: AsRef<str>> TerminalString for S {}

/// An ANSI terminal color.
///
/// This object is capable of addressing 3-bit, 4-bit, and 8-bit
/// [ANSI colors](https://en.wikipedia.org/wiki/ANSI_escape_code#Colors).
///
/// This enum is a part of the [Style] interface.
#[derive(Clone, Copy, Debug)]
pub enum Color {
    /// A 3-bit black color.
    Black,

    /// A 3-bit red color.
    Red,

    /// A 3-bit green color.
    Green,

    /// A 3-bit yellow color.
    Yellow,

    /// A 3-bit blue color.
    Blue,

    /// A 3-bit magenta color.
    Magenta,

    /// A 3-bit cyan color.
    Cyan,

    /// A 3-bit white color.
    White,

    /// A 4-bit bright black color.
    BrightBlack,

    /// A 4-bit bright red color.
    BrightRed,

    /// A 4-bit bright green color.
    BrightGreen,

    /// A 4-bit bright yellow color.
    BrightYellow,

    /// A 4-bit bright blue color.
    BrightBlue,

    /// A 4-bit bright magenta color.
    BrightMagenta,

    /// A 4-bit bright cyan color.
    BrightCyan,

    /// A 4-bit bright white color.
    BrightWhite,

    /// A 8-bit RGB color.
    ///
    /// The scale of each component is from 0.0 to 1.0. Values outside of
    /// this range will be clamped. The final 8-bit representation of the color
    /// will be inferred to be as close to the floating-point components
    /// representation as possible.
    RGB {
        /// A red component of the RGB color in range from 0.0 to 1.0.
        red: f64,

        /// A green component of the RGB color in range from 0.0 to 1.0.
        green: f64,

        /// A blue component of the RGB color in range from 0.0 to 1.0.
        blue: f64,
    },

    /// A 8-bit Grayscale color.
    ///
    /// The scale of the inner shade component is from 0.0 to 1.0. Values
    /// outside of this range will be clamped. The final 8-bit representation of
    /// the grayscale color will be inferred to be as close to
    /// the floating-point shade as possible.
    Grayscale(f64),
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Black, Self::Black) => true,
            (Self::Red, Self::Red) => true,
            (Self::Green, Self::Green) => true,
            (Self::Yellow, Self::Yellow) => true,
            (Self::Blue, Self::Blue) => true,
            (Self::Magenta, Self::Magenta) => true,
            (Self::Cyan, Self::Cyan) => true,
            (Self::White, Self::White) => true,
            (Self::BrightBlack, Self::BrightBlack) => true,
            (Self::BrightRed, Self::BrightRed) => true,
            (Self::BrightGreen, Self::BrightGreen) => true,
            (Self::BrightYellow, Self::BrightYellow) => true,
            (Self::BrightBlue, Self::BrightBlue) => true,
            (Self::BrightMagenta, Self::BrightMagenta) => true,
            (Self::BrightCyan, Self::BrightCyan) => true,
            (Self::BrightWhite, Self::BrightWhite) => true,

            (
                Self::RGB {
                    red: this_red,
                    green: this_green,
                    blue: this_blue,
                },
                Self::RGB {
                    red: other_red,
                    green: other_green,
                    blue: other_blue,
                },
            ) => match (
                this_red.partial_cmp(other_red),
                this_green.partial_cmp(other_green),
                this_blue.partial_cmp(other_blue),
            ) {
                (Some(Ordering::Equal), Some(Ordering::Equal), Some(Ordering::Equal))
                | (None, None, None) => true,
                _ => false,
            },

            (Self::Grayscale(this), Self::Grayscale(other)) => match this.partial_cmp(other) {
                Some(Ordering::Equal) | None => true,
                _ => false,
            },

            _ => false,
        }
    }
}

impl Eq for Color {}

impl Color {
    #[inline]
    fn apply_fg(&self, target: &mut String) {
        macro_rules! escape_fg {
            ($code:expr) => {
                concat!("\x1B[38;5;", $code, "m")
            };
        }

        match self {
            Self::Black => target.push_str(escape_fg!(0)),
            Self::Red => target.push_str(escape_fg!(1)),
            Self::Green => target.push_str(escape_fg!(2)),
            Self::Yellow => target.push_str(escape_fg!(3)),
            Self::Blue => target.push_str(escape_fg!(4)),
            Self::Magenta => target.push_str(escape_fg!(5)),
            Self::Cyan => target.push_str(escape_fg!(6)),
            Self::White => target.push_str(escape_fg!(7)),
            Self::BrightBlack => target.push_str(escape_fg!(8)),
            Self::BrightRed => target.push_str(escape_fg!(9)),
            Self::BrightGreen => target.push_str(escape_fg!(10)),
            Self::BrightYellow => target.push_str(escape_fg!(11)),
            Self::BrightBlue => target.push_str(escape_fg!(12)),
            Self::BrightMagenta => target.push_str(escape_fg!(13)),
            Self::BrightCyan => target.push_str(escape_fg!(14)),
            Self::BrightWhite => target.push_str(escape_fg!(15)),

            Self::RGB { red, green, blue } => {
                let red = ((red.clamp(0.0, 1.0) * 5.0) as u8).min(5);
                let green = ((green.clamp(0.0, 1.0) * 5.0) as u8).min(5);
                let blue = ((blue.clamp(0.0, 1.0) * 5.0) as u8).min(5);

                target.push_str(&format!("\x1B[38;5;{}m", 36 * red + 6 * green + blue + 16));
            }

            Self::Grayscale(shade) => {
                let shade = ((shade.clamp(0.0, 1.0) * 23.0) as u8).min(23);

                target.push_str(&format!("\x1B[38;5;{}m", shade + 232));
            }
        }
    }

    #[inline]
    fn apply_bg(&self, target: &mut String) {
        macro_rules! escape_bg {
            ($code:expr) => {
                concat!("\x1B[48;5;", $code, "m")
            };
        }

        match self {
            Self::Black => target.push_str(escape_bg!(0)),
            Self::Red => target.push_str(escape_bg!(1)),
            Self::Green => target.push_str(escape_bg!(2)),
            Self::Yellow => target.push_str(escape_bg!(3)),
            Self::Blue => target.push_str(escape_bg!(4)),
            Self::Magenta => target.push_str(escape_bg!(5)),
            Self::Cyan => target.push_str(escape_bg!(6)),
            Self::White => target.push_str(escape_bg!(7)),
            Self::BrightBlack => target.push_str(escape_bg!(8)),
            Self::BrightRed => target.push_str(escape_bg!(9)),
            Self::BrightGreen => target.push_str(escape_bg!(10)),
            Self::BrightYellow => target.push_str(escape_bg!(11)),
            Self::BrightBlue => target.push_str(escape_bg!(12)),
            Self::BrightMagenta => target.push_str(escape_bg!(13)),
            Self::BrightCyan => target.push_str(escape_bg!(14)),
            Self::BrightWhite => target.push_str(escape_bg!(15)),

            Self::RGB { red, green, blue } => {
                let red = ((red.clamp(0.0, 1.0) * 5.0) as u8).min(5);
                let green = ((green.clamp(0.0, 1.0) * 5.0) as u8).min(5);
                let blue = ((blue.clamp(0.0, 1.0) * 5.0) as u8).min(5);

                target.push_str(&format!("\x1B[48;5;{}m", 36 * red + 6 * green + blue + 16));
            }

            Self::Grayscale(shade) => {
                let shade = ((shade.clamp(0.0, 1.0) * 23.0) as u8).min(23);

                target.push_str(&format!("\x1B[48;5;{}m", shade + 232));
            }
        }
    }

    #[inline(always)]
    fn reset_fg(target: &mut String) {
        target.push_str(escape!(39));
    }

    #[inline(always)]
    fn reset_bg(target: &mut String) {
        target.push_str(escape!(49));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Token)]
#[repr(u8)]
pub(super) enum Escaped {
    EOI = 0,
    Text = 1,
    #[rule("\x1B[" ['\x30'..'\x4F']* ['\x20'..'\x2F']* ['\x40'..'\x7E'])]
    CSI,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Emphasis {
    bold: bool,
    italic: bool,
    underline: bool,
    invert: bool,
}

impl Emphasis {
    #[inline(always)]
    const fn none() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            invert: false,
        }
    }

    #[inline(always)]
    fn is_some(&self) -> bool {
        self.bold || self.italic || self.underline || self.invert
    }

    #[inline(always)]
    fn apply(&self, target: &mut String) {
        if self.bold {
            target.push_str(escape!(1));
        }

        if self.italic {
            target.push_str(escape!(3));
        }

        if self.underline {
            target.push_str(escape!(4));
        }

        if self.invert {
            target.push_str(escape!(7));
        }
    }

    #[inline(always)]
    fn change(from: &Self, to: &Self, target: &mut String) -> bool {
        if from.bold <= to.bold
            && from.italic <= to.italic
            && from.underline <= to.underline
            && from.invert <= to.invert
        {
            if !from.bold && to.bold {
                target.push_str(escape!(1));
            }

            if !from.italic && to.italic {
                target.push_str(escape!(3));
            }

            if !from.underline && to.underline {
                target.push_str(escape!(4));
            }

            if !from.invert && to.invert {
                target.push_str(escape!(7));
            }

            return true;
        }

        reset_all(target);
        to.apply(target);

        false
    }
}

#[inline(always)]
fn reset_all(target: &mut String) {
    target.push_str(escape!(0));
}
