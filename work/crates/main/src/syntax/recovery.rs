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

use crate::{
    lexis::{Token, TokenRule, TokenSet, EOI},
    syntax::SyntaxSession,
};

/// A static syntax errors recovery configuration without any halting rules.
///
/// The value of this static equals to the [Recovery::unlimited] value.
pub static UNLIMITED_RECOVERY: Recovery = Recovery::unlimited();

/// A configuration object of the Lady Deirdre canonical syntax errors recovery
/// algorithm.
///
/// When the syntax parser encounters a token that is not supposed to be in
/// the current parse position, the typical strategy to recover from this syntax
/// error is to enter so-called "panic mode" (do not be confused with the Rust
/// panic).
///
/// In this mode, the recovery strategy is to consume and ignore all unexpected
/// tokens ahead until the token cursor encounters a token from the expected
/// set, and the parser would continue normally.
///
/// The main drawback of this strategy is that it could consume too many
/// erroneous tokens inside the innermost syntax rules, while it would be
/// more efficient to return to the parental rules earlier.
///
/// For instance, if the expression parser parses the erroneous expression
/// in the `let x = a + ; let y = b;` source code, it would consume the `; let `
/// part until it encounters the next `y` identifier, interpreting it as
/// the right-hand side of the "a + y" binary operator. This further damages the
/// following "let" statement that is well-formed by itself.
///
/// To avoid such cases we should configure a predefined set of tokens that
/// would halt the panic mode. In the example above it would be a `;` token.
///
/// Furthermore, if the panic recovery mode encounters a _group_ of tokens
/// enclosed by an open/close pair of tokens, we probably would prefer to
/// interpret this group as whole, ignoring any halting tokens inside the group.
///
/// For instance, in the source code `let x = a + {foo;} b;`, we would prefer to
/// erase the `{foo;}` part, interpreting the expression as "a + b" instead of
/// halting the panic mode on the first encountered semicolon inside the
/// "{...}" group.
///
/// The Recovery object configures both kinds of the panic recovery rules:
///  - The [unlimited](Recovery::unlimited) function creates a configuration
///    without any halting rules.
///  - The [unexpected](Recovery::unexpected) and
///    the [unexpected_set](Recovery::unexpected) functions include the tokens
///    on which the panic recovery is assumed to be halted earlier if the tokens
///    are encountered outside of any group.
///  - The [group](Recovery::group) function specifies a pair of tokens that
///    denote the bounds of a sequence of grouped tokens that should be
///    interpreted as a whole; under which the halting rules should not
///    be applied.
///
/// Note that the panic recovery algorithm takes into account groups nesting,
/// and if a particular group is not properly balanced by the open and the close
/// tokens, its content will be separated.
///
/// For example, in the `{foo [ (bar) baz}` code the `{foo [ ` and the ` baz}`
/// parts will be treated as sequences of separated tokens because the the "["
/// token is not enclosed. But the `(bar)` part will be treated as a whole group
/// because it is properly enclosed.
///
/// The construction methods of the Recovery object are the const functions.
///
/// The object is assumed to be constructed in a const context as a static value
/// upfront to reduce the runtime overhead.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct Recovery {
    groups: [(TokenRule, TokenRule); Self::GROUPS_LIMIT as usize],
    groups_len: u8,
    unexpected: TokenSet,
}

impl Recovery {
    /// The maximum number of groups this recovery configuration can address.
    ///
    /// This number may be increased in future minor versions of Lady Deirdre.
    pub const GROUPS_LIMIT: u8 = 4;

    /// Creates a new recovery configuration without any halting rules.
    ///
    /// If you need just a static reference to the unlimited recovery
    /// configuration, use the predefined [UNLIMITED_RECOVERY] static.
    #[inline(always)]
    pub const fn unlimited() -> Self {
        Self {
            groups: [(0, 0); Self::GROUPS_LIMIT as usize],
            groups_len: 0,
            unexpected: TokenSet::empty(),
        }
    }

    /// Specifies a pair of tokens that denote the bounds of a group of tokens
    /// that should be treated as a whole; under which the halting rules should
    /// not be applied.
    ///
    /// The `open` and the `close` tokens must be two different tokens.
    ///
    /// The `open` and the `close` tokens must differ from to the bounds of the
    /// previously configured groups.
    ///
    /// **Panic**
    ///
    /// Panics if the Recovery already has configured
    /// [GROUPS_LIMIT](Self::GROUPS_LIMIT) groups.
    ///
    /// Panics if `open` equals `close`.
    ///
    /// Panics if the Recovery already has a group with a bound that equal to
    /// `open` or `close`.
    #[inline(always)]
    pub const fn group(mut self, open: TokenRule, close: TokenRule) -> Self {
        if open == close {
            panic!("Group open and close tokens must be different.");
        }

        let mut group_id = 0;
        while group_id < self.groups_len {
            let (other_open, other_close) = self.groups[group_id as usize];

            if other_open == open {
                panic!("Duplicate group open token.")
            }

            if other_close == open {
                panic!("Duplicate group open token.")
            }

            if other_open == close {
                panic!("Duplicate group close token.")
            }

            if other_close == close {
                panic!("Duplicate group close token.")
            }

            group_id += 1;
        }

        if self.groups_len >= Self::GROUPS_LIMIT {
            panic!("Groups limit exceeded.");
        }

        self.groups[self.groups_len as usize] = (open, close);
        self.groups_len += 1;

        self
    }

    /// Adds a token to the halting token set configuration that
    /// terminates panic recovery if the recovery algorithm encounters any token
    /// from the halting set outside of a group.
    ///
    /// The recovery algorithm **does not** consume termination tokens.
    #[inline(always)]
    pub const fn unexpected(mut self, rule: TokenRule) -> Self {
        self.unexpected = self.unexpected.include(rule);

        self
    }

    /// Adds a token set to the halting token set configuration that
    /// terminates panic recovery if the recovery algorithm encounters any token
    /// from the halting set outside of a group.
    ///
    /// The recovery algorithm **does not** consume termination tokens.
    #[inline(always)]
    pub const fn unexpected_set(mut self, set: TokenSet) -> Self {
        self.unexpected = self.unexpected.union(set);

        self
    }

    /// Runs the recovery algorithm with this recovery configuration starting
    /// from the current token in the `syntax` [SyntaxSession].
    ///
    /// The algorithm sequentially consumes all tokens in the [SyntaxSession]
    /// until it finds any token from the `until` [TokenSet], or any token
    /// in the preconfigured set of halting tokens (but outside of token
    /// [groups](Self::group)), or until it reaches the end of the input.
    ///
    /// If the `until` token encountered, this token **will not be** consumed,
    /// and the function returns [RecoveryResult::PanicRecover].
    ///
    /// If the halting token encountered, this token **will not be** consumed,
    /// and the function returns [RecoveryResult::UnexpectedToken].
    ///
    /// If the token cursor reaches the end of the input, the function returns
    /// [RecoveryResult::UnexpectedEOI].
    #[inline]
    pub fn recover<'code>(
        &self,
        session: &mut impl SyntaxSession<'code>,
        until: &TokenSet,
    ) -> RecoveryResult {
        let mut stack = GroupStack::new();

        loop {
            let rule = session.token(0).rule();

            if until.contains(rule) {
                return RecoveryResult::PanicRecover;
            }

            if self.unexpected.contains(rule) {
                return RecoveryResult::UnexpectedToken;
            }

            let mut group_id = 0u8;
            while group_id < self.groups_len {
                let open = self.groups[group_id as usize].0;

                if open == rule {
                    self.try_skip_group(session, &mut stack, group_id);
                }

                group_id += 1;
            }

            if !session.advance() {
                return RecoveryResult::UnexpectedEOI;
            }
        }
    }

    #[inline(always)]
    fn try_skip_group<'code>(
        &self,
        session: &mut impl SyntaxSession<'code>,
        stack: &mut GroupStack,
        mut group_id: u8,
    ) {
        stack.clear();
        stack.push(group_id);

        let mut distance = 0;

        'outer: loop {
            distance += 1;

            let rule = session.token(distance).rule();

            if rule == EOI {
                break;
            }

            group_id = 0;
            while group_id < self.groups_len {
                let (open, close) = self.groups[group_id as usize];

                if open == rule {
                    stack.push(group_id);
                    break;
                }

                if close == rule {
                    let id = match stack.pop() {
                        None => break 'outer,
                        Some(id) => id,
                    };

                    if id != group_id {
                        stack.push(id);
                        break 'outer;
                    }

                    if stack.is_empty() {
                        break 'outer;
                    }

                    break;
                }

                group_id += 1;
            }
        }

        if stack.is_empty() {
            session.skip(distance);
        }
    }
}

/// A syntax error recovery strategy.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum RecoveryResult {
    /// The parser successfully recovered from the syntax error assuming that
    /// the end-user missed a required token.
    InsertRecover,

    /// The parser successfully recovered from the syntax error by skipping
    /// a continuous sequence of unexpected tokens, but finally encountering
    /// the one that was expected and continued the parsing process normally.
    PanicRecover,

    /// The parsing rule has failed to recover from the syntax error by skipping
    /// a continuous sequence of unexpected tokens (possibly an empty sequence)
    /// because the parser has encountered the end of the input.
    ///
    /// In the end, the parsing rule has assembled the product node based on the
    /// data it was able to parse so far and returned control flow.
    UnexpectedEOI,

    /// The parsing rule has failed to recover from the syntax error by skipping
    /// a continuous sequence of unexpected tokens (possibly an empty sequence)
    /// because the parser has encountered the token that possibly belongs
    /// to the parent rule.
    ///
    /// In the end, the parsing rule has assembled the product node based on the
    /// data it was able to parse so far and returned control flow.
    UnexpectedToken,
}

impl RecoveryResult {
    /// Returns true, if the recovery was successful.
    #[inline(always)]
    pub fn recovered(&self) -> bool {
        match self {
            Self::InsertRecover | Self::PanicRecover => true,
            _ => false,
        }
    }
}

enum GroupStack {
    Inline {
        vec: [u8; Self::INLINE],
        length: usize,
    },
    Heap {
        vec: Vec<u8>,
    },
}

impl GroupStack {
    const INLINE: usize = 8;

    #[inline(always)]
    fn new() -> Self {
        Self::Inline {
            vec: [0; Self::INLINE],
            length: 0,
        }
    }

    #[inline(always)]
    fn push(&mut self, index: u8) {
        match self {
            Self::Inline { vec, length } => {
                if *length < Self::INLINE {
                    vec[*length] = index;
                    *length += 1;
                    return;
                }

                let mut vec = Vec::from(*vec);
                vec.push(index);

                *self = Self::Heap { vec }
            }

            Self::Heap { vec } => vec.push(index),
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<u8> {
        match self {
            Self::Inline { vec, length } => match *length > 0 {
                true => {
                    *length -= 1;

                    let item = vec[*length];

                    Some(item)
                }

                false => None,
            },

            Self::Heap { vec } => vec.pop(),
        }
    }

    #[inline(always)]
    fn clear(&mut self) {
        match self {
            Self::Inline { length, .. } => *length = 0,
            Self::Heap { vec } => vec.clear(),
        }
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        match self {
            Self::Inline { length, .. } => *length == 0,
            Self::Heap { vec } => vec.is_empty(),
        }
    }
}
