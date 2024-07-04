////////////////////////////////////////////////////////////////////////////////
// This file is a part of the "Lady Deirdre" work,                            //
// a compiler front-end foundation technology.                                //
//                                                                            //
// This work is proprietary software with source-available code.              //
//                                                                            //
// To copy, use, distribute, and contribute to this work, you must agree to   //
// the terms of the General License Agreement:                                //
//                                                                            //
// https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.          //
//                                                                            //
// The agreement grants you a Commercial-Limited License that gives you       //
// the right to use my work in non-commercial and limited commercial products //
// with a total gross revenue cap. To remove this commercial limit for one of //
// your products, you must acquire an Unrestricted Commercial License.        //
//                                                                            //
// If you contribute to the source code, documentation, or related materials  //
// of this work, you must assign these changes to me. Contributions are       //
// governed by the "Derivative Work" section of the General License           //
// Agreement.                                                                 //
//                                                                            //
// Copying the work in parts is strictly forbidden, except as permitted under //
// the terms of the General License Agreement.                                //
//                                                                            //
// If you do not or cannot agree to the terms of this Agreement,              //
// do not use this work.                                                      //
//                                                                            //
// This work is provided "as is" without any warranties, express or implied,  //
// except to the extent that such disclaimers are held to be legally invalid. //
//                                                                            //
// Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).                 //
// All rights reserved.                                                       //
////////////////////////////////////////////////////////////////////////////////

//TODO check warnings regularly
#![allow(warnings)]

mod download;
mod generate;
mod parse;

use std::{env::args, process::exit};

static UCD_DOWNLOADS_DIR: &str = "downloads";

static UCD_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/";

static UCD_RESOURCES: &[&str] = &[
    "DerivedCoreProperties.txt",
    "PropList.txt",
    "UnicodeData.txt",
    "SpecialCasing.txt",
];

static GENERATED_FILE: &str = "ucd_gen.txt";

static RAW_PROPERTIES: &[PropDesc] = &[
    PropDesc {
        raw_names: &["Alphabetic"],
        table_name: "ALPHABETIC_TABLE",
        field_name: "alpha",
    },
    PropDesc {
        raw_names: &["Lowercase"],
        table_name: "LOWERCASE_TABLE",
        field_name: "lower",
    },
    PropDesc {
        raw_names: &["Uppercase"],
        table_name: "UPPERCASE_TABLE",
        field_name: "upper",
    },
    PropDesc {
        raw_names: &["White_Space"],
        table_name: "WHITE_SPACE_TABLE",
        field_name: "space",
    },
    PropDesc {
        raw_names: &["N", "Nd", "Nl", "No"],
        table_name: "NUM_TABLE",
        field_name: "num",
    },
    PropDesc {
        raw_names: &["XID_Start"],
        table_name: "XID_START_TABLE",
        field_name: "xid_start",
    },
    PropDesc {
        raw_names: &["XID_Continue"],
        table_name: "XID_CONTINUE_TABLE",
        field_name: "xid_continue",
    },
];

#[derive(PartialEq, Eq, Hash)]
struct PropDesc {
    raw_names: &'static [&'static str],
    table_name: &'static str,
    field_name: &'static str,
}

fn main() {
    let mut arg = match args().skip(1).next() {
        Some(arg) => arg,

        None => {
            eprintln!("Missing command. Available commands are: \"download\", \"generate\"");
            exit(1);
        }
    };

    match arg.as_str() {
        "download" => download::download(),
        "generate" => generate::generate(),

        other => {
            eprintln!(
                "Unknown command {other}. Available commands are: \"download\", \"generate\"",
            );
            exit(1);
        }
    }
}
