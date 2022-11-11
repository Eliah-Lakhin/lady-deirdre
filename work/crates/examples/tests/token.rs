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

use lady_deirdre::lexis::{CodeContent, SimpleToken, TokenBuffer};

#[test]
fn test_default_token() {
    let buffer = TokenBuffer::<SimpleToken>::from("public foo() { x = 100 + 2.0 - 'a'[\"bar\"]; }");

    assert_eq!(
        "public| |foo|(|)| |{| |x| |=| |100| |+| |2.0| |-| |'a'|[|\"bar\"|]|;| |}",
        buffer
            .chunks(..)
            .map(|chunk| chunk.string.to_string())
            .collect::<Vec<_>>()
            .join("|"),
    );

    assert_eq!(
        "Identifier|Whitespace|Identifier|ParenOpen|ParenClose|Whitespace|BraceOpen|\
            Whitespace|Identifier|Whitespace|Symbol|Whitespace|Number|Whitespace|Symbol|\
            Whitespace|Number|Whitespace|Symbol|Whitespace|Char|BracketOpen|String|BracketClose|\
            Symbol|Whitespace|BraceClose",
        buffer
            .chunks(..)
            .map(|chunk| chunk.token.to_string())
            .collect::<Vec<_>>()
            .join("|")
    );

    assert_eq!(
        "0|6|7|10|11|12|13|14|15|16|17|18|19|22|23|24|25|28|29|30|31|34|35|40|41|42|43",
        buffer
            .chunks(..)
            .map(|chunk| chunk.site.to_string())
            .collect::<Vec<_>>()
            .join("|")
    );
}
