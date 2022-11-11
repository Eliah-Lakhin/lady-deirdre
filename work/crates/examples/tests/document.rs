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

//TODO check warnings regularly
#![allow(warnings)]

use std::fmt::{Debug, Display, Formatter};

use lady_deirdre::{
    lexis::{CodeContent, LexisSession, SourceCode, Token, TokenCursor},
    syntax::{NoSyntax, SimpleNode},
    Document,
};

#[test]
fn test_document_lexis() {
    #[derive(Clone, Debug)]
    pub enum CustomToken {
        A,
        B,
        C,
        F,
    }

    impl Token for CustomToken {
        #[inline]
        fn new(session: &mut impl LexisSession) -> Self {
            let mut kind = 0;

            loop {
                let current = session.character();
                session.advance();

                if current == '\0' {
                    break;
                }

                let next = session.character();

                match (kind, current, next) {
                    // Token A
                    (0, '1', '1') => {
                        kind = 1;
                    }

                    (0, '1', _) => {
                        kind = 1;
                        session.submit();
                        break;
                    }

                    (1, '1', '1') => (),

                    (1, '1', _) => {
                        session.submit();
                        break;
                    }

                    // Token B
                    (0, '2', '2') => {
                        kind = 2;
                    }

                    (0, '2', _) => {
                        kind = 2;
                        session.submit();
                        break;
                    }

                    (2, '2', '2') => (),

                    (2, '2', _) => {
                        session.submit();
                        break;
                    }

                    // Token C
                    (0, '3', '3') => {
                        kind = 2;
                    }

                    (0, '3', _) => {
                        kind = 2;
                        session.submit();
                        break;
                    }

                    (2, '3', '3') => (),

                    (2, '3', _) => {
                        session.submit();
                        break;
                    }

                    _ => break,
                }
            }

            match kind {
                0 => CustomToken::F,
                1 => CustomToken::A,
                2 => CustomToken::B,
                3 => CustomToken::C,
                _ => unreachable!(),
            }
        }
    }

    impl Display for CustomToken {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            Debug::fmt(self, formatter)
        }
    }

    let mut document = Document::<NoSyntax<CustomToken>>::default();

    document.write(.., "111222111");

    assert_eq!(document.length(), 9);
    assert_eq!(
        "111|222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|3|6",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "111|222",
        document
            .chunks(0..5)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "111|222|111",
        document
            .chunks(3..6)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "222",
        document
            .chunks(4..4)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(0..0, "1");

    assert_eq!(
        "1111|222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|4|7",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(4..4, "1");

    assert_eq!(
        "11111|222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|5|8",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(5..5, "2");

    assert_eq!(
        "11111|2222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|5|9",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(5..5, "$");

    assert_eq!(
        "11111|$|2222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|F|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|5|6|10",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(5..5, "1");

    assert_eq!(
        "111111|$|2222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|F|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|6|7|11",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(7..7, "@");

    assert_eq!(
        "111111|$@|2222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|F|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|6|8|12",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(1..5, "2");

    assert_eq!(
        "1|2|1|$@|2222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A|F|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|1|2|3|5|9",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(2..5, "");

    assert_eq!(
        "1|22222|111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|1|6",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(100..100, "11");

    assert_eq!(
        "1|22222|11111",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|1|6",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(8..11, "");

    assert_eq!(
        "1|22222|11",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|1|6",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    document.write(8..8, "2");

    assert_eq!(
        "1|22222|11|2",
        document
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "A|B|A|B",
        document
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
    assert_eq!(
        "0|1|6|8",
        document
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );
}

#[test]
fn test_document_write() {
    let mut document = Document::<SimpleNode>::default();

    assert_eq!(document.substring(..), "");

    document.write(.., "foo bar");

    assert_eq!(document.substring(..), "foo bar");

    document.write(.., "foo Xbar");

    assert_eq!(document.substring(..), "foo Xbar");

    document.write(0..0, "123 ");

    assert_eq!(document.substring(..), "123 foo Xbar");

    document.write(100.., "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15");

    assert_eq!(
        document.substring(..),
        "123 foo Xbar1 2 3 4 5 6 7 8 9 10 11 12 13 14 15",
    );

    assert_eq!(document.length(), 47);
    assert_eq!(document.token_count(), 33);
    assert_eq!(document.cursor(..).string(0).unwrap(), "123");
    assert_eq!(document.cursor(..).string(1).unwrap(), " ");
    assert_eq!(document.cursor(..).string(2).unwrap(), "foo");
    assert_eq!(document.cursor(..).string(3).unwrap(), " ");
    assert_eq!(document.cursor(..).string(4).unwrap(), "Xbar1");
    assert_eq!(document.cursor(..).string(5).unwrap(), " ");
    assert_eq!(document.cursor(..).string(6).unwrap(), "2");

    document.write(6..10, "");

    assert_eq!(
        document.substring(..),
        "123 foar1 2 3 4 5 6 7 8 9 10 11 12 13 14 15",
    );

    document.write(9..10, "");

    assert_eq!(
        document.substring(..),
        "123 foar12 3 4 5 6 7 8 9 10 11 12 13 14 15",
    );

    assert_eq!(document.length(), 42);
    assert_eq!(document.token_count(), 29);
    assert_eq!(document.cursor(..).string(0).unwrap(), "123");
    assert_eq!(document.cursor(..).string(1).unwrap(), " ");
    assert_eq!(document.cursor(..).string(2).unwrap(), "foar12");
    assert_eq!(document.cursor(..).string(3).unwrap(), " ");
    assert_eq!(document.cursor(..).string(4).unwrap(), "3");
    assert_eq!(document.cursor(..).string(5).unwrap(), " ");
    assert_eq!(document.cursor(..).string(6).unwrap(), "4");
    assert_eq!(document.cursor(6..7).string(0).unwrap(), "foar12");
    assert!(document.cursor(6..7).string(1).is_none());

    document.write(4..36, "");

    assert_eq!(document.length(), 10);
    assert_eq!(document.token_count(), 5);
    assert_eq!(document.substring(..), "123  14 15");
    assert_eq!(document.cursor(..).string(0).unwrap(), "123");
    assert_eq!(document.cursor(..).string(1).unwrap(), "  ");
    assert_eq!(document.cursor(..).string(2).unwrap(), "14");
    assert_eq!(document.cursor(..).string(3).unwrap(), " ");
    assert_eq!(document.cursor(..).string(4).unwrap(), "15");
    assert!(document.cursor(..).string(5).is_none());
}
