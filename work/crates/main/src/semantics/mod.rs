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

mod attr;
mod db;
mod record;
mod result;

pub use crate::semantics::{
    attr::{Attr, AttrContext, AttrReadGuard, AttrRef},
    db::Db,
    result::{AttrError, AttrResult},
};

#[cfg(test)]
mod tests {
    use crate::{
        semantics::{Attr, AttrContext, AttrError, Db},
        std::*,
        sync::Lazy,
        syntax::NodeRef,
    };

    #[test]
    fn test_semantics_framework() {
        static DB: Lazy<Db> = Lazy::new(|| Db::new());

        static ATTR_1: Lazy<Attr<usize>> = Lazy::new(|| {
            Attr::new(&DB, NodeRef::nil(), &|_ctx: &mut AttrContext| {
                LOG.write().unwrap().push(1);
                Ok(*INPUT.read().unwrap().deref())
            })
        });

        static ATTR_2: Lazy<Attr<usize>> = Lazy::new(|| {
            Attr::new(&DB, NodeRef::nil(), &|ctx: &mut AttrContext| {
                LOG.write().unwrap().push(2);
                let attr1 = ATTR_1.read(ctx)?;
                Ok(*attr1 + 50)
            })
        });

        static INPUT: Lazy<RwLock<usize>> = Lazy::new(|| RwLock::new(100));
        static LOG: Lazy<RwLock<Vec<u8>>> = Lazy::new(|| RwLock::new(Vec::new()));

        let _ = ATTR_1.deref();
        let _ = ATTR_2.deref();

        {
            let value = ATTR_2.read(&mut AttrContext::new()).unwrap();

            assert_eq!(value.deref(), &150);
            assert_eq!(LOG.read().unwrap().deref(), &[2, 1]);
        }

        ATTR_1.as_ref().invalidate().unwrap();

        {
            let value = ATTR_2.read(&mut AttrContext::new()).unwrap();

            assert_eq!(value.deref(), &150);
            assert_eq!(LOG.read().unwrap().deref(), &[2, 1, 1]);
        }

        *INPUT.write().unwrap() = 30;
        ATTR_1.as_ref().invalidate().unwrap();

        {
            let value = ATTR_2.read(&mut AttrContext::new()).unwrap();

            assert_eq!(value.deref(), &80);
            assert_eq!(LOG.read().unwrap().deref(), &[2, 1, 1, 1, 2]);
        }
    }

    #[test]
    fn test_cycle_detection() {
        static DB: Lazy<Db> = Lazy::new(|| Db::new());

        static ATTR: Lazy<Attr<usize>> = Lazy::new(|| {
            Attr::new(&DB, NodeRef::nil(), &|ctx: &mut AttrContext| {
                Ok(*ATTR.read(ctx)?.deref())
            })
        });

        let _ = ATTR.deref();

        assert!(matches!(
            ATTR.read(&mut AttrContext::new()),
            Err(AttrError::CycleDetected)
        ))
    }
}
