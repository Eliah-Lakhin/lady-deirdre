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

use std::process::exit;

use ahash::{AHashMap, AHashSet};
use ucd_parse::{parse, UnicodeDataExpander};

use crate::{PropDesc, RAW_PROPERTIES, UCD_DOWNLOADS_DIR};

pub(super) fn parse_raw_data() -> Vec<(&'static PropDesc, AHashSet<u32>)> {
    println!("Parsing raw UCD data...");

    let mut data = AHashMap::new();

    let raw_core_properties = match parse::<_, ucd_parse::CoreProperty>(&UCD_DOWNLOADS_DIR) {
        Ok(props) => {
            println!("Raw Core Properties parsed.");
            props
        }
        Err(error) => {
            eprintln!("Core Properties parse error: {error}");
            exit(1);
        }
    };

    for raw in raw_core_properties {
        let Some(desc) = RAW_PROPERTIES
            .iter()
            .find(|desc| desc.raw_names.contains(&raw.property.as_str()))
        else {
            continue;
        };

        let code_points = data.entry(desc).or_insert_with(AHashSet::new);

        for code_point in raw.codepoints {
            let _ = code_points.insert(code_point.value());
        }
    }

    let raw_properties = match parse::<_, ucd_parse::Property>(&UCD_DOWNLOADS_DIR) {
        Ok(props) => {
            println!("Raw Properties parsed.");
            props
        }
        Err(error) => {
            eprintln!("Properties parse error: {error}");
            exit(1);
        }
    };

    for raw in raw_properties {
        let Some(desc) = RAW_PROPERTIES
            .iter()
            .find(|desc| desc.raw_names.contains(&raw.property.as_str()))
        else {
            continue;
        };

        let code_points = data.entry(desc).or_insert_with(AHashSet::new);

        for code_point in raw.codepoints {
            let _ = code_points.insert(code_point.value());
        }
    }

    let raw_unicode_data = match parse::<_, ucd_parse::UnicodeData>(&UCD_DOWNLOADS_DIR) {
        Ok(props) => {
            println!("Unicode Data parsed.");
            props
        }
        Err(error) => {
            eprintln!("Unicode Data parse error: {error}");
            exit(1);
        }
    };

    for raw in UnicodeDataExpander::new(raw_unicode_data) {
        let Some(desc) = RAW_PROPERTIES
            .iter()
            .find(|desc| desc.raw_names.contains(&raw.general_category.as_str()))
        else {
            continue;
        };

        let code_points = data.entry(desc).or_insert_with(AHashSet::new);

        let _ = code_points.insert(raw.codepoint.value());
    }

    println!("Raw UCD data parsing finished.");

    let mut sorted = data.into_iter().collect::<Vec<_>>();

    sorted.sort_by_key(|(prop, _)| prop.field_name);

    sorted
}
