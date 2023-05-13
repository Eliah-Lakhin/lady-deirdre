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
    arena::{Id, Identifiable, RepositoryIterator},
    incremental::storage::ChildRefIndex,
    std::*,
    syntax::Node,
};

pub struct DocumentErrorIterator<'document, N: Node> {
    pub(super) id: Id,
    pub(super) cursor: ChildRefIndex<N>,
    pub(super) current: RepositoryIterator<'document, N::Error>,
}

impl<'document, N: Node> Identifiable for DocumentErrorIterator<'document, N> {
    #[inline(always)]
    fn id(&self) -> Id {
        self.id
    }
}

impl<'document, N: Node> Iterator for DocumentErrorIterator<'document, N> {
    type Item = &'document N::Error;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(error) = self.current.next() {
            return Some(error);
        }

        while !self.cursor.is_dangling() {
            if let Some(cache) = unsafe { self.cursor.cache() } {
                let mut iterator = (&cache.cluster.errors).into_iter();

                match iterator.next() {
                    None => (),

                    Some(result) => {
                        unsafe { self.cursor.next() };

                        self.current = iterator;

                        return Some(result);
                    }
                }
            }

            unsafe { self.cursor.next() };
        }

        None
    }
}

impl<'document, N: Node> FusedIterator for DocumentErrorIterator<'document, N> {}
