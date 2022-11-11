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

use lady_deirdre::{
    lexis::{CodeContent, SimpleToken, SourceCode, ToSite, TokenBuffer, TokenCursor},
    syntax::NoSyntax,
    Document,
};

#[test]
fn test_chunk_iterator() {
    tests(TokenBuffer::<SimpleToken>::from(
        "public foo() { x = 100 + 2.0 - 'a'[\"bar\"]; }",
    ));
    tests(Document::<NoSyntax<SimpleToken>>::from(
        "public foo() { x = 100 + 2.0 - 'a'[\"bar\"]; }",
    ));

    fn tests(code: impl SourceCode) {
        assert_eq!(
            "public| |foo|(|)| |{| |x| |=| |100| |+| |2.0| |-| |'a'|[|\"bar\"|]|;| |}",
            code.chunks(..)
                .map(|chunk| chunk.string.to_string())
                .collect::<Vec<_>>()
                .join("|"),
        );

        assert_eq!(
            "public| |foo",
            code.chunks(2..7)
                .map(|chunk| chunk.string.to_string())
                .collect::<Vec<_>>()
                .join("|"),
        );

        let mut cursor = code.cursor(2..7);

        assert_eq!(cursor.site_ref(0).to_site(&code).unwrap(), 0);
        assert_eq!(cursor.token_ref(0).string(&code).unwrap(), "public");
        assert!(matches!(cursor.string(0), Some(string) if string == "public"));

        assert_eq!(cursor.site_ref(1).to_site(&code).unwrap(), 6);
        assert_eq!(cursor.token_ref(1).string(&code).unwrap(), " ");
        assert!(matches!(cursor.string(1), Some(string) if string == " "));

        assert_eq!(cursor.site_ref(2).to_site(&code).unwrap(), 7);
        assert_eq!(cursor.token_ref(2).string(&code).unwrap(), "foo");
        assert!(matches!(cursor.string(2), Some(string) if string == "foo"));

        assert_eq!(cursor.site_ref(3).to_site(&code).unwrap(), 10);
        assert!(!cursor.token_ref(3).is_valid_ref(&code));
        assert!(matches!(cursor.string(3), None));

        assert_eq!(cursor.site_ref(1).to_site(&code).unwrap(), 6);
        assert_eq!(cursor.token_ref(1).string(&code).unwrap(), " ");
        assert!(matches!(cursor.string(1), Some(string) if string == " "));

        assert!(cursor.advance());

        assert_eq!(cursor.site_ref(0).to_site(&code).unwrap(), 6);
        assert_eq!(cursor.token_ref(0).string(&code).unwrap(), " ");
        assert!(matches!(cursor.string(0), Some(string) if string == " "));

        assert_eq!(cursor.site_ref(1).to_site(&code).unwrap(), 7);
        assert_eq!(cursor.token_ref(1).string(&code).unwrap(), "foo");
        assert!(matches!(cursor.string(1), Some(string) if string == "foo"));

        assert_eq!(cursor.site_ref(2).to_site(&code).unwrap(), 10);
        assert!(!cursor.token_ref(2).is_valid_ref(&code));
        assert!(matches!(cursor.string(2), None));

        assert!(cursor.advance());
        assert!(cursor.advance());

        assert_eq!(cursor.site_ref(0).to_site(&code).unwrap(), 10);
        assert!(!cursor.token_ref(0).is_valid_ref(&code));
        assert!(matches!(cursor.string(0), None));
    }
}

#[test]
fn test_empty_chunk_iterator() {
    tests(Document::<NoSyntax<SimpleToken>>::default());
    tests(TokenBuffer::<SimpleToken>::default());

    fn tests(code: impl SourceCode) {
        assert!(code.chunks(..).collect::<Vec<_>>().is_empty());
        assert!(code.chunks(2..7).collect::<Vec<_>>().is_empty());

        let mut cursor = code.cursor(2..7);

        assert_eq!(cursor.site_ref(0).to_site(&code).unwrap(), 0);
        assert_eq!(cursor.site_ref(1).to_site(&code).unwrap(), 0);
        assert!(!cursor.token_ref(0).is_valid_ref(&code));
        assert!(!cursor.token_ref(1).is_valid_ref(&code));
        assert!(matches!(cursor.string(0), None));
        assert!(matches!(cursor.string(1), None));

        assert!(!cursor.advance());

        assert_eq!(cursor.site_ref(0).to_site(&code).unwrap(), 0);
        assert_eq!(cursor.site_ref(1).to_site(&code).unwrap(), 0);
        assert!(!cursor.token_ref(0).is_valid_ref(&code));
        assert!(!cursor.token_ref(1).is_valid_ref(&code));
        assert!(matches!(cursor.string(0), None));
        assert!(matches!(cursor.string(1), None));
    }
}

#[test]
fn test_char_iterator() {
    tests(TokenBuffer::<SimpleToken>::from("foo bar baz"));
    tests(Document::<NoSyntax<SimpleToken>>::from("foo bar baz"));

    fn tests(code: impl SourceCode) {
        assert_eq!("foo bar baz", code.substring(..));
        assert_eq!("oo bar b", code.substring(1..9));
        assert_eq!("", code.substring(100..));
        assert_eq!("", code.substring(2..2));
    }
}
