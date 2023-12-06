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

use crate::std::*;

macro_rules! escape {
    ($($code:expr)?) => {
        concat!("\x1B[", $( $code, )? "m")
    };
}

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
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            emphasis: Emphasis::none(),
        }
    }

    #[inline(always)]
    pub const fn black(self) -> Self {
        self.fg(Color::Black)
    }

    #[inline(always)]
    pub const fn red(self) -> Self {
        self.fg(Color::Red)
    }

    #[inline(always)]
    pub const fn green(self) -> Self {
        self.fg(Color::Green)
    }

    #[inline(always)]
    pub const fn yellow(self) -> Self {
        self.fg(Color::Yellow)
    }

    #[inline(always)]
    pub const fn blue(self) -> Self {
        self.fg(Color::Blue)
    }

    #[inline(always)]
    pub const fn magenta(self) -> Self {
        self.fg(Color::Magenta)
    }

    #[inline(always)]
    pub const fn cyan(self) -> Self {
        self.fg(Color::Cyan)
    }

    #[inline(always)]
    pub const fn white(self) -> Self {
        self.fg(Color::White)
    }

    #[inline(always)]
    pub const fn bright_black(self) -> Self {
        self.fg(Color::BrightBlack)
    }

    #[inline(always)]
    pub const fn bright_red(self) -> Self {
        self.fg(Color::BrightRed)
    }

    #[inline(always)]
    pub const fn bright_green(self) -> Self {
        self.fg(Color::BrightGreen)
    }

    #[inline(always)]
    pub const fn bright_yellow(self) -> Self {
        self.fg(Color::BrightYellow)
    }

    #[inline(always)]
    pub const fn bright_blue(self) -> Self {
        self.fg(Color::BrightBlue)
    }

    #[inline(always)]
    pub const fn bright_magenta(self) -> Self {
        self.fg(Color::BrightMagenta)
    }

    #[inline(always)]
    pub const fn bright_cyan(self) -> Self {
        self.fg(Color::BrightCyan)
    }

    #[inline(always)]
    pub const fn bright_white(self) -> Self {
        self.fg(Color::BrightWhite)
    }

    #[inline(always)]
    pub const fn rgb(self, red: f64, green: f64, blue: f64) -> Self {
        self.fg(Color::RGB { red, green, blue })
    }

    #[inline(always)]
    pub const fn grayscale(self, shade: f64) -> Self {
        self.fg(Color::Grayscale(shade))
    }

    #[inline(always)]
    pub const fn bold(mut self) -> Self {
        self.emphasis.bold = true;

        self
    }

    #[inline(always)]
    pub const fn italic(mut self) -> Self {
        self.emphasis.italic = true;

        self
    }

    #[inline(always)]
    pub const fn underline(mut self) -> Self {
        self.emphasis.underline = true;

        self
    }

    #[inline(always)]
    pub const fn invert(mut self) -> Self {
        self.emphasis.invert = true;

        self
    }

    #[inline(always)]
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);

        self
    }

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

pub trait TerminalString: AsRef<str> {
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
}

impl<S: AsRef<str>> TerminalString for S {}

#[derive(Clone, Copy, Debug)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    RGB { red: f64, green: f64, blue: f64 },
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
