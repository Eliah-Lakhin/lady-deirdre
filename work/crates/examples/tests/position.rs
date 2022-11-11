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
    lexis::{Position, SimpleToken, SourceCode, ToPosition, ToSite, TokenBuffer},
    syntax::NoSyntax,
    Document,
};

#[test]
fn test_position_to_site() {
    tests(TokenBuffer::<SimpleToken>::from("foo \n bar \r\nbaz"));
    tests(Document::<NoSyntax<SimpleToken>>::from(
        "foo \n bar \r\nbaz",
    ));

    fn tests(code: impl SourceCode) {
        assert_eq!(0, Position::new(0, 10).to_site(&code).unwrap());
        assert_eq!(0, Position::new(1, 1).to_site(&code).unwrap());
        assert_eq!(1, Position::new(1, 2).to_site(&code).unwrap());
        assert_eq!(4, Position::new(1, 10).to_site(&code).unwrap());
        assert_eq!(5, Position::new(2, 1).to_site(&code).unwrap());
        assert_eq!(9, Position::new(2, 5).to_site(&code).unwrap());
        assert_eq!(10, Position::new(2, 10).to_site(&code).unwrap());
        assert_eq!(12, Position::new(3, 0).to_site(&code).unwrap());
        assert_eq!(12, Position::new(3, 1).to_site(&code).unwrap());
        assert_eq!(13, Position::new(3, 2).to_site(&code).unwrap());
        assert_eq!(15, Position::new(3, 4).to_site(&code).unwrap());
    }
}

#[test]
fn test_site_to_position() {
    tests(TokenBuffer::<SimpleToken>::from("foo \n bar \r\nbaz"));
    tests(Document::<NoSyntax<SimpleToken>>::from(
        "foo \n bar \r\nbaz",
    ));

    fn tests(code: impl SourceCode) {
        assert_eq!(Position::new(1, 1), 0.to_position(&code).unwrap());
        assert_eq!(Position::new(1, 2), 1.to_position(&code).unwrap());
        assert_eq!(Position::new(1, 4), 3.to_position(&code).unwrap());
        assert_eq!(Position::new(1, 5), 4.to_position(&code).unwrap());
        assert_eq!(Position::new(2, 1), 5.to_position(&code).unwrap());
        assert_eq!(Position::new(2, 2), 6.to_position(&code).unwrap());
        assert_eq!(Position::new(2, 3), 7.to_position(&code).unwrap());
        assert_eq!(Position::new(2, 4), 8.to_position(&code).unwrap());
        assert_eq!(Position::new(2, 5), 9.to_position(&code).unwrap());
        assert_eq!(Position::new(2, 6), 10.to_position(&code).unwrap());
        assert_eq!(Position::new(2, 7), 11.to_position(&code).unwrap());
        assert_eq!(Position::new(3, 1), 12.to_position(&code).unwrap());
        assert_eq!(Position::new(3, 2), 13.to_position(&code).unwrap());
        assert_eq!(Position::new(3, 3), 14.to_position(&code).unwrap());
        assert_eq!(Position::new(3, 4), 15.to_position(&code).unwrap());
        assert_eq!(Position::new(3, 4), 16.to_position(&code).unwrap());
    }
}
