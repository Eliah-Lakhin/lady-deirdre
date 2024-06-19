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

use lady_deirdre::{
    lexis::TokenSet,
    syntax::{NodeRef, NodeRule, PolyRef, Recovery, SyntaxError, SyntaxSession, EMPTY_NODE_SET},
};

use crate::expr_parser::{lexis::BoolToken, syntax::BoolNode};

pub fn parse_expr<'a>(session: &mut impl SyntaxSession<'a, Node = BoolNode>) -> BoolNode {
    let node = session.node_ref();
    let parent = session.parent_ref();
    let content = parse_operator(session, BoolNode::EXPR, 0);

    BoolNode::Expr {
        node,
        parent,
        content,
    }
}

static OPERAND_RECOVERY: Recovery = Recovery::unlimited()
    .group(BoolToken::ParenOpen as u8, BoolToken::ParenClose as u8)
    .unexpected(BoolToken::ParenClose as u8);

static OPERAND_TOKENS: TokenSet = TokenSet::inclusive(&[
    BoolToken::True as u8,
    BoolToken::False as u8,
    BoolToken::ParenOpen as u8,
]);

static GROUP_RECOVERY: Recovery = Recovery::unlimited().unexpected(BoolToken::ParenOpen as u8);

static GROUP_TOKENS: TokenSet = TokenSet::inclusive(&[BoolToken::ParenClose as u8]);

static OPERATOR_RECOVERY: Recovery =
    Recovery::unlimited().group(BoolToken::ParenOpen as u8, BoolToken::ParenClose as u8);

static OPERATOR_TOKENS: TokenSet = TokenSet::inclusive(&[
    BoolToken::And as u8,
    BoolToken::Or as u8,
    BoolToken::ParenClose as u8,
]);

fn parse_operator<'a>(
    session: &mut impl SyntaxSession<'a, Node = BoolNode>,
    context: NodeRule,
    binding: u8,
) -> NodeRef {
    let mut accumulator = parse_operand(session, context);

    loop {
        skip_trivia(session);

        let token = session.token(0);

        match token {
            BoolToken::And => {
                if binding >= 2 {
                    return accumulator;
                }

                let left = accumulator;

                let node = session.enter(BoolNode::AND);

                if !left.is_nil() {
                    session.lift(&left);
                }

                let parent = session.parent_ref();

                session.advance();
                skip_trivia(session);

                let right = parse_operator(session, BoolNode::AND, 2);

                accumulator = session.leave(BoolNode::And {
                    node,
                    parent,
                    left,
                    right,
                });
            }

            BoolToken::Or => {
                if binding >= 10 {
                    return accumulator;
                }

                let left = accumulator;

                let node = session.enter(BoolNode::OR);

                if !left.is_nil() {
                    session.lift(&left);
                }

                let parent = session.parent_ref();

                session.advance();
                skip_trivia(session);

                let right = parse_operator(session, BoolNode::OR, 1);

                accumulator = session.leave(BoolNode::Or {
                    node,
                    parent,
                    left,
                    right,
                });
            }

            BoolToken::ParenClose | BoolToken::EOI => return accumulator,

            _ => {
                let start_site_ref = session.site_ref(0);

                let result = OPERATOR_RECOVERY.recover(session, &OPERATOR_TOKENS);

                let end_site_ref = session.site_ref(0);

                session.failure(SyntaxError {
                    span: start_site_ref..end_site_ref,
                    context,
                    recovery: result,
                    expected_tokens: &OPERATOR_TOKENS,
                    expected_nodes: &EMPTY_NODE_SET,
                });

                if !result.recovered() {
                    return accumulator;
                }
            }
        }
    }
}

fn parse_operand<'a>(
    session: &mut impl SyntaxSession<'a, Node = BoolNode>,
    context: NodeRule,
) -> NodeRef {
    loop {
        let token = session.token(0);

        match token {
            BoolToken::True => return parse_true_operand(session),

            BoolToken::False => return parse_false_operand(session),

            BoolToken::ParenOpen => return parse_group(session),

            _ => {
                let start_site_ref = session.site_ref(0);

                let result = OPERAND_RECOVERY.recover(session, &OPERAND_TOKENS);

                let end_site_ref = session.site_ref(0);

                session.failure(SyntaxError {
                    span: start_site_ref..end_site_ref,
                    context,
                    recovery: result,
                    expected_tokens: &OPERAND_TOKENS,
                    expected_nodes: &EMPTY_NODE_SET,
                });

                if !result.recovered() {
                    return NodeRef::nil();
                }
            }
        }
    }
}

fn parse_true_operand<'a>(session: &mut impl SyntaxSession<'a, Node = BoolNode>) -> NodeRef {
    session.enter(BoolNode::TRUE);

    session.advance();

    let node = session.node_ref();
    let parent = session.parent_ref();

    return session.leave(BoolNode::True { node, parent });
}

fn parse_false_operand<'a>(session: &mut impl SyntaxSession<'a, Node = BoolNode>) -> NodeRef {
    session.enter(BoolNode::FALSE);

    session.advance();

    let node = session.node_ref();
    let parent = session.parent_ref();

    return session.leave(BoolNode::False { node, parent });
}

fn parse_group<'a>(session: &mut impl SyntaxSession<'a, Node = BoolNode>) -> NodeRef {
    session.advance();

    skip_trivia(session);

    let inner = session.descend(BoolNode::EXPR);

    skip_trivia(session);

    loop {
        let token = session.token(0);

        match token {
            BoolToken::ParenClose => {
                session.advance();
                return inner;
            }

            _ => {
                let start_site_ref = session.site_ref(0);

                let result = GROUP_RECOVERY.recover(session, &GROUP_TOKENS);

                let end_site_ref = session.site_ref(0);

                session.failure(SyntaxError {
                    span: start_site_ref..end_site_ref,
                    context: BoolNode::EXPR,
                    recovery: result,
                    expected_tokens: &GROUP_TOKENS,
                    expected_nodes: &EMPTY_NODE_SET,
                });

                if !result.recovered() {
                    return inner;
                }
            }
        }
    }
}

fn skip_trivia<'a>(session: &mut impl SyntaxSession<'a, Node = BoolNode>) {
    loop {
        let token = session.token(0);

        if token != BoolToken::Whitespace {
            break;
        }

        session.advance();
    }
}
