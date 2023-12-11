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

mod branch;
mod cache;
mod child;
mod item;
mod nesting;
mod page;
mod references;
mod string;
mod tree;
mod utils;

pub(crate) use crate::units::storage::{
    cache::ClusterCache,
    child::ChildCursor,
    references::References,
    tree::Tree,
};
use crate::{lexis::CHUNK_SIZE, units::storage::utils::capacity};

const BRANCH_B: usize = 6;
const BRANCH_CAP: usize = capacity(BRANCH_B);

const PAGE_B: usize = 16;
const PAGE_CAP: usize = capacity(PAGE_B);

const STRING_INLINE: usize = PAGE_CAP * CHUNK_SIZE;

#[cfg(test)]
mod tests {
    use crate::{
        analysis::{FeatureInitializer, FeatureInvalidator},
        lexis::{LexisSession, Token, TokenRule, TokenSet},
        std::*,
        sync::SyncBuildHasher,
        syntax::{Children, Node, NodeRef, NodeRule, ParseError, SyntaxSession},
        units::storage::{
            child::ChildIndex,
            item::{ItemRef, ItemRefVariant},
            nesting::{BranchLayer, Height, PageLayer},
            references::References,
            tree::Tree,
        },
    };
    #[cfg(not(debug_assertions))]
    use crate::{
        lexis::{Length, Site},
        units::storage::{branch::Branch, item::Item, page::Page},
    };

    #[test]
    fn test_bulk_load_1() {
        let mut tree = Tree::<TestNode>::default();

        assert_eq!(TreeDisplay(&tree).to_string(), r#"height: 0, length: 0"#,);

        unsafe {
            tree.free();
        }
    }

    #[test]
    fn test_bulk_load_2() {
        let mut references = References::<TestNode>::default();

        let mut tree = gen(&mut references, 1..=1);

        assert_eq!(
            TreeDisplay(&tree).to_string(),
            r#"height: 1, length: 1, [
    1,
]"#,
        );

        unsafe {
            tree.free();
        }
    }

    #[test]
    fn test_bulk_load_3() {
        let mut references = References::<TestNode>::default();

        let mut tree = gen(&mut references, 1..=20);

        assert_eq!(
            TreeDisplay(&tree).to_string().as_str(),
            r#"height: 2, length: 20, {
    6: [1, 2, 3, 4, 5, 6],
    7: [7, 8, 9, 10, 11, 12, 13],
    7: [14, 15, 16, 17, 18, 19, 20],
}"#
        );

        unsafe {
            tree.free();
        }
    }

    #[test]
    fn test_bulk_load_4() {
        let mut references = References::<TestNode>::default();

        let mut tree = gen(&mut references, 1..=300);

        assert_eq!(
            TreeDisplay(&tree).to_string().as_str(),
            r#"height: 3, length: 300, {
    36: {
        6: [1, 2, 3, 4, 5, 6],
        6: [7, 8, 9, 10, 11, 12],
        6: [13, 14, 15, 16, 17, 18],
        6: [19, 20, 21, 22, 23, 24],
        6: [25, 26, 27, 28, 29, 30],
        6: [31, 32, 33, 34, 35, 36],
    },
    36: {
        6: [37, 38, 39, 40, 41, 42],
        6: [43, 44, 45, 46, 47, 48],
        6: [49, 50, 51, 52, 53, 54],
        6: [55, 56, 57, 58, 59, 60],
        6: [61, 62, 63, 64, 65, 66],
        6: [67, 68, 69, 70, 71, 72],
    },
    36: {
        6: [73, 74, 75, 76, 77, 78],
        6: [79, 80, 81, 82, 83, 84],
        6: [85, 86, 87, 88, 89, 90],
        6: [91, 92, 93, 94, 95, 96],
        6: [97, 98, 99, 100, 101, 102],
        6: [103, 104, 105, 106, 107, 108],
    },
    36: {
        6: [109, 110, 111, 112, 113, 114],
        6: [115, 116, 117, 118, 119, 120],
        6: [121, 122, 123, 124, 125, 126],
        6: [127, 128, 129, 130, 131, 132],
        6: [133, 134, 135, 136, 137, 138],
        6: [139, 140, 141, 142, 143, 144],
    },
    36: {
        6: [145, 146, 147, 148, 149, 150],
        6: [151, 152, 153, 154, 155, 156],
        6: [157, 158, 159, 160, 161, 162],
        6: [163, 164, 165, 166, 167, 168],
        6: [169, 170, 171, 172, 173, 174],
        6: [175, 176, 177, 178, 179, 180],
    },
    36: {
        6: [181, 182, 183, 184, 185, 186],
        6: [187, 188, 189, 190, 191, 192],
        6: [193, 194, 195, 196, 197, 198],
        6: [199, 200, 201, 202, 203, 204],
        6: [205, 206, 207, 208, 209, 210],
        6: [211, 212, 213, 214, 215, 216],
    },
    42: {
        6: [217, 218, 219, 220, 221, 222],
        6: [223, 224, 225, 226, 227, 228],
        6: [229, 230, 231, 232, 233, 234],
        6: [235, 236, 237, 238, 239, 240],
        6: [241, 242, 243, 244, 245, 246],
        6: [247, 248, 249, 250, 251, 252],
        6: [253, 254, 255, 256, 257, 258],
    },
    42: {
        6: [259, 260, 261, 262, 263, 264],
        6: [265, 266, 267, 268, 269, 270],
        6: [271, 272, 273, 274, 275, 276],
        6: [277, 278, 279, 280, 281, 282],
        6: [283, 284, 285, 286, 287, 288],
        6: [289, 290, 291, 292, 293, 294],
        6: [295, 296, 297, 298, 299, 300],
    },
}"#
        );

        unsafe {
            tree.free();
        }
    }

    #[test]
    fn test_join_roots_1() {
        let mut references = References::<TestNode>::default();
        let mut left = gen(&mut references, 1..=300);
        let right = gen(&mut references, 301..=600);

        unsafe { left.join(&mut references, right) };

        assert_eq!(
            TreeDisplay(&left).to_string().as_str(),
            r#"height: 4, length: 600, {
    300: {
        36: {
            6: [1, 2, 3, 4, 5, 6],
            6: [7, 8, 9, 10, 11, 12],
            6: [13, 14, 15, 16, 17, 18],
            6: [19, 20, 21, 22, 23, 24],
            6: [25, 26, 27, 28, 29, 30],
            6: [31, 32, 33, 34, 35, 36],
        },
        36: {
            6: [37, 38, 39, 40, 41, 42],
            6: [43, 44, 45, 46, 47, 48],
            6: [49, 50, 51, 52, 53, 54],
            6: [55, 56, 57, 58, 59, 60],
            6: [61, 62, 63, 64, 65, 66],
            6: [67, 68, 69, 70, 71, 72],
        },
        36: {
            6: [73, 74, 75, 76, 77, 78],
            6: [79, 80, 81, 82, 83, 84],
            6: [85, 86, 87, 88, 89, 90],
            6: [91, 92, 93, 94, 95, 96],
            6: [97, 98, 99, 100, 101, 102],
            6: [103, 104, 105, 106, 107, 108],
        },
        36: {
            6: [109, 110, 111, 112, 113, 114],
            6: [115, 116, 117, 118, 119, 120],
            6: [121, 122, 123, 124, 125, 126],
            6: [127, 128, 129, 130, 131, 132],
            6: [133, 134, 135, 136, 137, 138],
            6: [139, 140, 141, 142, 143, 144],
        },
        36: {
            6: [145, 146, 147, 148, 149, 150],
            6: [151, 152, 153, 154, 155, 156],
            6: [157, 158, 159, 160, 161, 162],
            6: [163, 164, 165, 166, 167, 168],
            6: [169, 170, 171, 172, 173, 174],
            6: [175, 176, 177, 178, 179, 180],
        },
        36: {
            6: [181, 182, 183, 184, 185, 186],
            6: [187, 188, 189, 190, 191, 192],
            6: [193, 194, 195, 196, 197, 198],
            6: [199, 200, 201, 202, 203, 204],
            6: [205, 206, 207, 208, 209, 210],
            6: [211, 212, 213, 214, 215, 216],
        },
        42: {
            6: [217, 218, 219, 220, 221, 222],
            6: [223, 224, 225, 226, 227, 228],
            6: [229, 230, 231, 232, 233, 234],
            6: [235, 236, 237, 238, 239, 240],
            6: [241, 242, 243, 244, 245, 246],
            6: [247, 248, 249, 250, 251, 252],
            6: [253, 254, 255, 256, 257, 258],
        },
        42: {
            6: [259, 260, 261, 262, 263, 264],
            6: [265, 266, 267, 268, 269, 270],
            6: [271, 272, 273, 274, 275, 276],
            6: [277, 278, 279, 280, 281, 282],
            6: [283, 284, 285, 286, 287, 288],
            6: [289, 290, 291, 292, 293, 294],
            6: [295, 296, 297, 298, 299, 300],
        },
    },
    300: {
        36: {
            6: [301, 302, 303, 304, 305, 306],
            6: [307, 308, 309, 310, 311, 312],
            6: [313, 314, 315, 316, 317, 318],
            6: [319, 320, 321, 322, 323, 324],
            6: [325, 326, 327, 328, 329, 330],
            6: [331, 332, 333, 334, 335, 336],
        },
        36: {
            6: [337, 338, 339, 340, 341, 342],
            6: [343, 344, 345, 346, 347, 348],
            6: [349, 350, 351, 352, 353, 354],
            6: [355, 356, 357, 358, 359, 360],
            6: [361, 362, 363, 364, 365, 366],
            6: [367, 368, 369, 370, 371, 372],
        },
        36: {
            6: [373, 374, 375, 376, 377, 378],
            6: [379, 380, 381, 382, 383, 384],
            6: [385, 386, 387, 388, 389, 390],
            6: [391, 392, 393, 394, 395, 396],
            6: [397, 398, 399, 400, 401, 402],
            6: [403, 404, 405, 406, 407, 408],
        },
        36: {
            6: [409, 410, 411, 412, 413, 414],
            6: [415, 416, 417, 418, 419, 420],
            6: [421, 422, 423, 424, 425, 426],
            6: [427, 428, 429, 430, 431, 432],
            6: [433, 434, 435, 436, 437, 438],
            6: [439, 440, 441, 442, 443, 444],
        },
        36: {
            6: [445, 446, 447, 448, 449, 450],
            6: [451, 452, 453, 454, 455, 456],
            6: [457, 458, 459, 460, 461, 462],
            6: [463, 464, 465, 466, 467, 468],
            6: [469, 470, 471, 472, 473, 474],
            6: [475, 476, 477, 478, 479, 480],
        },
        36: {
            6: [481, 482, 483, 484, 485, 486],
            6: [487, 488, 489, 490, 491, 492],
            6: [493, 494, 495, 496, 497, 498],
            6: [499, 500, 501, 502, 503, 504],
            6: [505, 506, 507, 508, 509, 510],
            6: [511, 512, 513, 514, 515, 516],
        },
        42: {
            6: [517, 518, 519, 520, 521, 522],
            6: [523, 524, 525, 526, 527, 528],
            6: [529, 530, 531, 532, 533, 534],
            6: [535, 536, 537, 538, 539, 540],
            6: [541, 542, 543, 544, 545, 546],
            6: [547, 548, 549, 550, 551, 552],
            6: [553, 554, 555, 556, 557, 558],
        },
        42: {
            6: [559, 560, 561, 562, 563, 564],
            6: [565, 566, 567, 568, 569, 570],
            6: [571, 572, 573, 574, 575, 576],
            6: [577, 578, 579, 580, 581, 582],
            6: [583, 584, 585, 586, 587, 588],
            6: [589, 590, 591, 592, 593, 594],
            6: [595, 596, 597, 598, 599, 600],
        },
    },
}"#
        );

        unsafe {
            left.free();
        }
    }

    #[test]
    fn test_join_roots_2() {
        let mut references = References::<TestNode>::default();
        let mut left = gen(&mut references, 1..=61);
        let right = gen(&mut references, 101..=115);

        unsafe { left.join(&mut references, right) };

        assert_eq!(
            TreeDisplay(&left).to_string().as_str(),
            r#"height: 3, length: 76, {
    36: {
        6: [1, 2, 3, 4, 5, 6],
        6: [7, 8, 9, 10, 11, 12],
        6: [13, 14, 15, 16, 17, 18],
        6: [19, 20, 21, 22, 23, 24],
        6: [25, 26, 27, 28, 29, 30],
        6: [31, 32, 33, 34, 35, 36],
    },
    40: {
        6: [37, 38, 39, 40, 41, 42],
        6: [43, 44, 45, 46, 47, 48],
        6: [49, 50, 51, 52, 53, 54],
        7: [55, 56, 57, 58, 59, 60, 61],
        7: [101, 102, 103, 104, 105, 106, 107],
        8: [108, 109, 110, 111, 112, 113, 114, 115],
    },
}"#
        );

        unsafe {
            left.free();
        }
    }

    #[test]
    fn test_join_roots_3() {
        let mut references = References::<TestNode>::default();
        let mut left = gen(&mut references, 1..=14);
        let right = gen(&mut references, 101..=161);

        unsafe { left.join(&mut references, right) };

        assert_eq!(
            TreeDisplay(&left).to_string().as_str(),
            r#"height: 3, length: 75, {
    38: {
        7: [1, 2, 3, 4, 5, 6, 7],
        7: [8, 9, 10, 11, 12, 13, 14],
        6: [101, 102, 103, 104, 105, 106],
        6: [107, 108, 109, 110, 111, 112],
        6: [113, 114, 115, 116, 117, 118],
        6: [119, 120, 121, 122, 123, 124],
    },
    37: {
        6: [125, 126, 127, 128, 129, 130],
        6: [131, 132, 133, 134, 135, 136],
        6: [137, 138, 139, 140, 141, 142],
        6: [143, 144, 145, 146, 147, 148],
        6: [149, 150, 151, 152, 153, 154],
        7: [155, 156, 157, 158, 159, 160, 161],
    },
}"#
        );

        unsafe {
            left.free();
        }
    }

    #[test]
    fn test_join_right_1() {
        let mut references = References::<TestNode>::default();
        let mut left = gen(&mut references, 1..=70);
        let right = gen(&mut references, 101..=103);

        unsafe { left.join(&mut references, right) };

        assert_eq!(
            TreeDisplay(&left).to_string().as_str(),
            r#"height: 2, length: 73, {
    6: [1, 2, 3, 4, 5, 6],
    6: [7, 8, 9, 10, 11, 12],
    6: [13, 14, 15, 16, 17, 18],
    6: [19, 20, 21, 22, 23, 24],
    6: [25, 26, 27, 28, 29, 30],
    6: [31, 32, 33, 34, 35, 36],
    6: [37, 38, 39, 40, 41, 42],
    7: [43, 44, 45, 46, 47, 48, 49],
    7: [50, 51, 52, 53, 54, 55, 56],
    7: [57, 58, 59, 60, 61, 62, 63],
    10: [64, 65, 66, 67, 68, 69, 70, 101, 102, 103],
}"#
        );

        unsafe {
            left.free();
        }
    }

    #[test]
    fn test_join_right_2() {
        let mut references = References::<TestNode>::default();
        let mut left = gen(&mut references, 1..=65);
        let right = gen(&mut references, 101..=105);

        unsafe { left.join(&mut references, right) };

        assert_eq!(
            TreeDisplay(&left).to_string().as_str(),
            r#"height: 2, length: 70, {
    6: [1, 2, 3, 4, 5, 6],
    6: [7, 8, 9, 10, 11, 12],
    6: [13, 14, 15, 16, 17, 18],
    6: [19, 20, 21, 22, 23, 24],
    6: [25, 26, 27, 28, 29, 30],
    7: [31, 32, 33, 34, 35, 36, 37],
    7: [38, 39, 40, 41, 42, 43, 44],
    7: [45, 46, 47, 48, 49, 50, 51],
    7: [52, 53, 54, 55, 56, 57, 58],
    6: [59, 60, 61, 62, 63, 64],
    6: [65, 101, 102, 103, 104, 105],
}"#
        );

        unsafe {
            left.free();
        }
    }

    #[test]
    fn test_join_right_3() {
        let mut references = References::<TestNode>::default();
        let mut left = gen(&mut references, 1..=70);
        let right = gen(&mut references, 101..=105);

        unsafe { left.join(&mut references, right) };

        assert_eq!(
            TreeDisplay(&left).to_string().as_str(),
            r#"height: 3, length: 75, {
    36: {
        6: [1, 2, 3, 4, 5, 6],
        6: [7, 8, 9, 10, 11, 12],
        6: [13, 14, 15, 16, 17, 18],
        6: [19, 20, 21, 22, 23, 24],
        6: [25, 26, 27, 28, 29, 30],
        6: [31, 32, 33, 34, 35, 36],
    },
    39: {
        6: [37, 38, 39, 40, 41, 42],
        7: [43, 44, 45, 46, 47, 48, 49],
        7: [50, 51, 52, 53, 54, 55, 56],
        7: [57, 58, 59, 60, 61, 62, 63],
        6: [64, 65, 66, 67, 68, 69],
        6: [70, 101, 102, 103, 104, 105],
    },
}"#
        );

        unsafe {
            left.free();
        }
    }

    #[test]
    fn test_join_left_1() {
        let mut references = References::<TestNode>::default();
        let mut left = gen(&mut references, 101..=106);
        let right = gen(&mut references, 1..=70);

        unsafe { left.join(&mut references, right) };

        assert_eq!(
            TreeDisplay(&left).to_string().as_str(),
            r#"height: 3, length: 76, {
    36: {
        6: [101, 102, 103, 104, 105, 106],
        6: [1, 2, 3, 4, 5, 6],
        6: [7, 8, 9, 10, 11, 12],
        6: [13, 14, 15, 16, 17, 18],
        6: [19, 20, 21, 22, 23, 24],
        6: [25, 26, 27, 28, 29, 30],
    },
    40: {
        6: [31, 32, 33, 34, 35, 36],
        6: [37, 38, 39, 40, 41, 42],
        7: [43, 44, 45, 46, 47, 48, 49],
        7: [50, 51, 52, 53, 54, 55, 56],
        7: [57, 58, 59, 60, 61, 62, 63],
        7: [64, 65, 66, 67, 68, 69, 70],
    },
}"#
        );

        unsafe {
            left.free();
        }
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_tree_release() {
        let empty = Tree::<TestNode>::default();

        assert!(empty.lookup(&mut 0).is_dangling());
        assert!(empty.lookup(&mut 10).is_dangling());

        for high in 0..4 {
            for low in 1..20 {
                let mut references = References::<TestNode>::default();

                let child_count = high * 1000 + low;
                let mut tree = gen(&mut references, 1..=child_count);

                check_tree_structure(&tree);
                check_tree_data(&tree, 1);

                let length = tree.length();

                for site in 1..length {
                    let chunk_cursor = {
                        let mut site = site;

                        let chunk_cursor = tree.lookup(&mut site);

                        assert_eq!(site, 0);

                        chunk_cursor
                    };

                    let start = unsafe { chunk_cursor.token().0 };

                    let right = unsafe { tree.split(&mut references, chunk_cursor) };

                    assert_eq!(tree.length(), site);
                    assert_eq!(right.length(), length - site);

                    check_tree_structure(&tree);
                    check_tree_data(&tree, 1);

                    check_tree_structure(&right);
                    check_tree_data(&right, start);

                    unsafe { tree.join(&mut references, right) };

                    check_tree_structure(&tree);
                    check_tree_data(&tree, 1);

                    assert_eq!(tree.length(), length);
                }

                let _ = unsafe { tree.free() };
            }
        }
    }

    fn gen(
        references: &mut References<TestNode>,
        range: RangeInclusive<ChildIndex>,
    ) -> Tree<TestNode> {
        let count = range.end() - range.start() + 1;

        let mut spans = Vec::with_capacity(count);
        let mut indices = Vec::with_capacity(count);
        let mut tokens = Vec::with_capacity(count);
        let mut text = String::with_capacity(count);

        for index in range {
            spans.push(1);
            indices.push(text.len());
            tokens.push(TestToken(index));
            text.push_str(index.to_string().as_str());
        }

        unsafe {
            Tree::from_chunks(
                references,
                count,
                spans.into_iter(),
                indices.into_iter(),
                tokens.into_iter(),
                text.as_str(),
            )
        }
    }

    #[cfg(not(debug_assertions))]
    fn check_tree_structure(tree: &Tree<TestNode>) {
        fn check_page(page_variant: &ItemRefVariant<TestNode>, outer_span: Length) {
            let page = unsafe { page_variant.as_page_ref().as_ref() };

            assert!(page.occupied >= Page::<TestNode>::B);

            assert!(!page.parent.is_dangling());

            let mut inner_span = 0;

            for index in 0..page.occupied {
                let child_span = page.spans[index];

                inner_span += child_span;
            }

            assert_eq!(outer_span, inner_span);
        }

        fn check_branch(
            branch_variant: &ItemRefVariant<TestNode>,
            depth: Height,
            outer_span: Length,
        ) {
            let branch = unsafe { branch_variant.as_branch_ref::<()>().as_ref() };

            assert!(branch.inner.occupied >= Branch::<(), TestNode>::B);

            assert!(!branch.inner.parent.is_dangling());

            let mut inner_span = 0;

            if depth > 2 {
                for index in 0..branch.inner.occupied {
                    let child_span = branch.inner.spans[index];

                    check_branch(&branch.inner.children[index], depth - 1, child_span);

                    inner_span += child_span;
                }

                assert_eq!(outer_span, inner_span);

                return;
            }

            for index in 0..branch.inner.occupied {
                let child_span = branch.inner.spans[index];

                check_page(&branch.inner.children[index], child_span);

                inner_span += child_span;
            }

            assert_eq!(outer_span, inner_span);
        }

        match tree.height {
            0 => (),

            1 => {
                let root = unsafe { tree.root.as_page_ref().as_ref() };

                assert!(root.parent.is_dangling());

                let mut inner_span = 0;

                for index in 0..root.occupied {
                    let child_span = root.spans[index];

                    inner_span += child_span;
                }

                assert_eq!(tree.length, inner_span);
            }

            2 => {
                let root = unsafe { tree.root.as_branch_ref::<()>().as_ref() };

                assert!(root.inner.parent.is_dangling());

                let mut inner_span = 0;

                for index in 0..root.inner.occupied {
                    let child_span = root.inner.spans[index];

                    check_page(&root.inner.children[index], child_span);

                    inner_span += child_span;
                }

                assert_eq!(tree.length, inner_span);
            }

            _ => {
                let root = unsafe { tree.root.as_branch_ref::<()>().as_ref() };

                assert!(root.inner.parent.is_dangling());

                let mut inner_span = 0;

                for index in 0..root.inner.occupied {
                    let child_span = root.inner.spans[index];

                    check_branch(&root.inner.children[index], tree.height - 1, child_span);

                    inner_span += child_span;
                }

                assert_eq!(tree.length, inner_span);
            }
        }
    }

    #[cfg(not(debug_assertions))]
    fn check_tree_data(tree: &Tree<TestNode>, start: Site) {
        if tree.height > 0 {
            let mut first = tree.first();
            let mut last = tree.last();

            assert!(!first.is_dangling());
            assert!(unsafe { first.is_first() });

            assert!(!last.is_dangling());
            assert!(unsafe { last.is_last() });

            match tree.length > 1 {
                true => {
                    assert!(!unsafe { first.is_last() });
                    assert!(!unsafe { last.is_first() });
                }

                false => {
                    assert!(unsafe { first.is_last() });
                    assert!(unsafe { last.is_first() });
                }
            }

            for index in 0..tree.length {
                assert_eq!(*unsafe { first.span() }, 1);
                assert_eq!(
                    unsafe { first.string() },
                    format!("{}", index + start).as_str(),
                );
                assert_eq!(unsafe { first.token() }, TestToken(index + start));

                assert_eq!(*unsafe { last.span() }, 1);
                assert_eq!(
                    unsafe { last.string() },
                    format!("{}", (tree.length + start - index - 1)).as_str(),
                );
                assert_eq!(
                    unsafe { last.token() },
                    TestToken(tree.length + start - index - 1),
                );

                unsafe { first.next() };
                unsafe { last.back() };
            }

            assert!(first.is_dangling());
            assert!(last.is_dangling());
        }

        for site in 0..tree.length {
            let chunk_cursor = {
                let mut site = site;

                let chunk_cursor = tree.lookup(&mut site);

                assert_eq!(site, 0);

                chunk_cursor
            };

            assert_eq!(
                unsafe { chunk_cursor.string() },
                format!("{}", (site + start)).as_str(),
            );
        }
    }

    struct TestNode;

    impl Node for TestNode {
        type Token = TestToken;
        type Error = ParseError;

        fn parse<'code>(
            _session: &mut impl SyntaxSession<'code, Node = Self>,
            _rule: NodeRule,
        ) -> Self {
            unimplemented!()
        }

        fn rule(&self) -> NodeRule {
            unimplemented!()
        }

        fn node_ref(&self) -> NodeRef {
            unimplemented!()
        }

        fn parent_ref(&self) -> NodeRef {
            unimplemented!()
        }

        fn set_parent_ref(&mut self, _parent: NodeRef) {}

        fn children(&self) -> Children {
            unimplemented!()
        }

        fn initialize<S: SyncBuildHasher>(
            &mut self,
            _initializer: &mut FeatureInitializer<Self, S>,
        ) {
        }

        fn invalidate<S: SyncBuildHasher>(&self, _invalidator: &mut FeatureInvalidator<Self, S>) {}

        fn name(_rule: NodeRule) -> Option<&'static str> {
            unimplemented!()
        }

        fn describe(_rule: NodeRule, _verbose: bool) -> Option<&'static str> {
            unimplemented!()
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct TestToken(usize);

    impl Token for TestToken {
        fn parse(_session: &mut impl LexisSession) -> Self {
            unimplemented!()
        }

        fn eoi() -> Self {
            unimplemented!()
        }

        fn mismatch() -> Self {
            unimplemented!()
        }

        fn rule(self) -> TokenRule {
            unimplemented!()
        }

        fn name(_index: TokenRule) -> Option<&'static str> {
            unimplemented!()
        }

        fn describe(_index: TokenRule, _verbose: bool) -> Option<&'static str> {
            unimplemented!()
        }

        fn blanks() -> &'static TokenSet {
            unimplemented!()
        }
    }

    struct TreeDisplay<'a, N: Node>(&'a Tree<N>);

    impl<'a, N: Node> Display for TreeDisplay<'a, N> {
        #[inline]
        fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
            struct PageDisplay<'a, N: Node> {
                page: &'a ItemRefVariant<N>,
            }

            impl<'a, N: Node> Debug for PageDisplay<'a, N> {
                fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
                    let page = unsafe { self.page.as_page_ref().as_ref() };

                    let mut list = formatter.debug_list();

                    for index in 0..page.occupied {
                        let string = match from_utf8(unsafe {
                            page.string.byte_slice(page.occupied, index)
                        }) {
                            Ok(string) => string,

                            Err(_) => unreachable!("Bad Unicode sequence."),
                        };

                        list.entry(&format_args!("{}", string));
                    }

                    list.finish()
                }
            }

            struct BranchDisplay<'a, N: Node> {
                height: Height,
                branch: &'a ItemRefVariant<N>,
            }

            impl<'a, N: Node> Debug for BranchDisplay<'a, N> {
                fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
                    match self.height {
                        0 => unreachable!("Incorrect height"),

                        1 => {
                            let branch =
                                unsafe { self.branch.as_branch_ref::<PageLayer>().as_ref() };

                            let mut list = formatter.debug_map();

                            for index in 0..branch.inner.occupied {
                                let span = branch.inner.spans[index];
                                let page = unsafe { branch.inner.children[index] };

                                list.entry(
                                    &span,
                                    &format_args!("{:?}", PageDisplay { page: &page }),
                                );
                            }

                            list.finish()
                        }

                        _ => {
                            let branch =
                                unsafe { self.branch.as_branch_ref::<BranchLayer>().as_ref() };

                            let mut list = formatter.debug_map();

                            for index in 0..branch.inner.occupied {
                                let span = branch.inner.spans[index];
                                let branch = unsafe { branch.inner.children[index] };

                                list.entry(
                                    &span,
                                    &BranchDisplay {
                                        height: self.height - 1,
                                        branch: &branch,
                                    },
                                );
                            }

                            list.finish()
                        }
                    }
                }
            }

            formatter.write_str(&format!(
                "height: {}, length: {}",
                &self.0.height, &self.0.length
            ))?;

            match self.0.height {
                0 => (),

                1 => {
                    formatter.write_str(&format!(", {:#?}", PageDisplay { page: &self.0.root }))?;
                }

                _ => {
                    formatter.write_str(&format!(
                        ", {:#?}",
                        BranchDisplay {
                            height: self.0.height - 1,
                            branch: &self.0.root
                        }
                    ))?;
                }
            }

            Ok(())
        }
    }
}
