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

use crate::{
    arena::{Id, Repo},
    syntax::{Node, SyntaxError},
    units::storage::child::ChildCursor,
};

pub(crate) struct TreeRefs<N: Node> {
    pub(crate) id: Id,
    pub(crate) chunks: Repo<ChildCursor<N>>,
    pub(crate) nodes: Repo<N>,
    pub(crate) errors: Repo<SyntaxError>,
}

impl<N: Node> TreeRefs<N> {
    #[inline(always)]
    pub(crate) fn new(id: Id) -> Self {
        Self {
            id,
            chunks: Repo::new(),
            nodes: Repo::new(),
            errors: Repo::new(),
        }
    }

    #[inline(always)]
    pub(crate) fn with_capacity(id: Id, mut capacity: usize) -> Self {
        capacity = (capacity + 1).next_power_of_two();

        Self {
            id,
            chunks: Repo::with_capacity(capacity),
            nodes: Repo::new(),
            errors: Repo::new(),
        }
    }
}
