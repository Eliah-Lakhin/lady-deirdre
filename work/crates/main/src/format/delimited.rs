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

///////////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of David Tolnay's                  //
// "prettyplease" work.                                                                  //
//                                                                                       //
// David Tolnay's original work available here:                                          //
// https://github.com/dtolnay/prettyplease/tree/6f7a9eebd7052fd5c37a84135e1daa7599176e7e //
//                                                                                       //
// David Tolnay provided his work under the following terms:                             //
//                                                                                       //
//   Permission is hereby granted, free of charge, to any                                //
//   person obtaining a copy of this software and associated                             //
//   documentation files (the "Software"), to deal in the                                //
//   Software without restriction, including without                                     //
//   limitation the rights to use, copy, modify, merge,                                  //
//   publish, distribute, sublicense, and/or sell copies of                              //
//   the Software, and to permit persons to whom the Software                            //
//   is furnished to do so, subject to the following                                     //
//   conditions:                                                                         //
//                                                                                       //
//   The above copyright notice and this permission notice                               //
//   shall be included in all copies or substantial portions                             //
//   of the Software.                                                                    //
//                                                                                       //
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF                               //
//   ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED                             //
//   TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A                                 //
//   PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT                                 //
//   SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY                            //
//   CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION                             //
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR                             //
//   IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER                                 //
//   DEALINGS IN THE SOFTWARE.                                                           //
//                                                                                       //
// Kindly be advised that the terms governing the distribution of my work are            //
// distinct from those pertaining to the original work of David Tolnay.                  //
///////////////////////////////////////////////////////////////////////////////////////////

use crate::std::*;

pub trait Delimited: Iterator + Sized {
    #[inline(always)]
    fn delimited(self) -> DelimitedIter<Self> {
        DelimitedIter {
            is_first: true,
            iter: self.peekable(),
        }
    }
}

impl<I: Iterator> Delimited for I {}

pub struct DelimitedItem<T> {
    pub value: T,
    pub is_first: bool,
    pub is_last: bool,
}

impl<T> Deref for DelimitedItem<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for DelimitedItem<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub struct DelimitedIter<I: Iterator> {
    is_first: bool,
    iter: Peekable<I>,
}

impl<I: Iterator> Iterator for DelimitedIter<I> {
    type Item = DelimitedItem<I::Item>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let item = DelimitedItem {
            value: self.iter.next()?,
            is_first: self.is_first,
            is_last: self.iter.peek().is_none(),
        };

        self.is_first = false;

        Some(item)
    }
}
