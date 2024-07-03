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

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of the code from the "Rust" work.                                  //
//                                                                                                                       //
// The original work available here:                                                                                     //
// https://github.com/rust-lang/rust/tree/488598c183ac55f6970bef34a1ed5404ae1d5088/src/tools/unicode-table-generator/src //
//                                                                                                                       //
// The authors of the original work grant me with a license to their work under the following terms:                     //
//                                                                                                                       //
//   Permission is hereby granted, free of charge, to any                                                                //
//   person obtaining a copy of this software and associated                                                             //
//   documentation files (the "Software"), to deal in the                                                                //
//   Software without restriction, including without                                                                     //
//   limitation the rights to use, copy, modify, merge,                                                                  //
//   publish, distribute, sublicense, and/or sell copies of                                                              //
//   the Software, and to permit persons to whom the Software                                                            //
//   is furnished to do so, subject to the following                                                                     //
//   conditions:                                                                                                         //
//                                                                                                                       //
//   The above copyright notice and this permission notice                                                               //
//   shall be included in all copies or substantial portions                                                             //
//   of the Software.                                                                                                    //
//                                                                                                                       //
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF                                                               //
//   ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED                                                             //
//   TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A                                                                 //
//   PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT                                                                 //
//   SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY                                                            //
//   CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION                                                             //
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR                                                             //
//   IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER                                                                 //
//   DEALINGS IN THE SOFTWARE.                                                                                           //
//                                                                                                                       //
// Kindly be advised that the terms governing the distribution of my work are                                            //
// distinct from those pertaining to the original work.                                                                  //
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

//TODO check warnings regularly
#![allow(warnings)]

use std::{
    env::args,
    fs::{create_dir_all, write},
    ops::Range,
    path::{Path, PathBuf},
    process::{exit, Command},
};

use ahash::AHashMap;
use ucd_parse::{parse, Codepoints, UnicodeDataExpander};

static UCD_DOWNLOADS_DIR: &str = "downloads";

static UCD_URL: &str = "https://www.unicode.org/Public/UCD/latest/ucd/";

static UCD_RESOURCES: &[&str] = &[
    "DerivedCoreProperties.txt",
    "PropList.txt",
    "UnicodeData.txt",
    "SpecialCasing.txt",
];

static GENERATED_FILE: &str = "ucd_gen.txt";

static RAW_PROPERTIES: &[&str] = &[
    "Alphabetic",
    "Lowercase",
    "Uppercase",
    "White_Space",
    "N",
    "XID_Start",
    "XID_Continue",
];

fn main() {
    let mut arg = match args().skip(1).next() {
        Some(arg) => arg,

        None => {
            eprintln!("Missing command. Available commands are: \"download\", \"generate\"");
            exit(1);
        }
    };

    match arg.as_str() {
        "download" => download(),
        "generate" => generate(),

        other => {
            eprintln!(
                "Unknown command {other}. Available commands are: \"download\", \"generate\"",
            );
            exit(1);
        }
    }
}

fn download() {
    println!("Downloading UCD data...");

    let downloads_dir = Path::new(UCD_DOWNLOADS_DIR);

    if downloads_dir.exists() {
        eprintln!(
            "Downloads dir {downloads_dir:?} already exists. Delete this directory manually.",
        );
        exit(1);
    }

    if let Err(error) = create_dir_all(downloads_dir) {
        eprintln!("Failed to created downloads dir {downloads_dir:?}: {error}");
        exit(1);
    }

    println!("Downloads dir {downloads_dir:?} created.");

    for resource in UCD_RESOURCES {
        let url = UCD_URL.to_owned() + resource;

        let output = match Command::new("curl").arg(&url).output() {
            Ok(output) => output,

            Err(error) => {
                eprintln!("Curl failed to fetch {url:?}: {error}",);
                exit(1);
            }
        };

        let file_name = downloads_dir.join(resource);

        match write(file_name.as_path(), output.stdout) {
            Ok(()) => {
                println!("Remote file {url} saved to {file_name:?}.");
            }

            Err(error) => {
                eprintln!("Field to save remote file {url} to {file_name:?}: {error}");
                exit(1);
            }
        };
    }

    println!("UCD data downloading finished.");
}

fn generate() {
    let raw_data = parse_raw_data();

    println!("Starting UCD module generation...");

    let mut output = String::new();

    output.push_str(
        r#"// This file is generated by "crates/ucd". Do not edit manually.

struct UCDTrie {
    r1: [u64; 32],
    r2: [u8; 992],
    r3: &'static [u64],
    r4: [u8; 256],
    r5: &'static [u8],
    r6: &'static [u64],
}

impl UCDTrie {
    #[inline(always)]
    fn lookup(&self, ch: char) -> bool {
        let code_point = ch as usize;

        if ch < 0x800 {
            return Self::check_chunk(self.r1[code_point >> 6], code_point);
        }

        if ch < 0x10000 {
            let child = self.r2[(code_point >> 6) - 0x20];

            return Self::check_chunk(self.r3[child as usize], code_point);
        }

        let child = self.r4[(code_point >> 12) - 0x10];
        let leaf = self.r5[((child as usize) << 6) + ((code_point >> 6) & 0x3f)];

        Self::check_chunk(self.r6[leaf as usize], code_point)
    }

    #[inline(always)]
    fn check_chunk(chunk: u64, code_point: usize) -> bool {
        ((chunk >> (code_point & 63)) & 1) != 0
    }
}

"#,
    );

    for (category, ranges) in raw_data {
        const TOTAL_RANGE: usize = 0x110000;
        const CHUNK_SIZE: usize = 64;
        const TOTAL_CHUNKS: usize = TOTAL_RANGE / CHUNK_SIZE;

        let mut flags = [false; TOTAL_RANGE];

        for range in &ranges {
            for code_point in range.start..range.end {
                flags[code_point as usize] = true;
            }
        }

        let mut chunks = Vec::new();

        for chunk_index in 0..TOTAL_CHUNKS {
            let mut chunk = 0u64;

            for bit in 0..CHUNK_SIZE {
                if flags[chunk_index * CHUNK_SIZE + bit] {
                    chunk |= 1 << bit;
                }
            }

            chunks.push(chunk);
        }

        let mut line_length = 0;

        let trie_static_name = category.to_ascii_uppercase();

        output.push_str(&format!(
            r#"#[rustfmt::skip]
static {trie_static_name}: UCDTrie = UCDTrie {{
"#
        ));

        output.push_str("    r1: ");
        print_chunk_array(&mut output, &chunks[0..(0x800 / CHUNK_SIZE)]);
        output.push_str(",\n    r2: ");
        output.push_str("}\n");
    }

    let path = PathBuf::from(GENERATED_FILE);

    match write(&path, output) {
        Ok(()) => {
            println!("Generated module saved to file {path:?}.");
        }
        Err(error) => {
            eprintln!("Field to save generated module to {path:?}: {error}");
            exit(1);
        }
    }

    println!("UCD module generation finished.");
}

fn parse_raw_data() -> AHashMap<&'static str, Vec<Range<u32>>> {
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
        let Some(name) = RAW_PROPERTIES
            .iter()
            .find(|prop| **prop == raw.property.as_str())
        else {
            continue;
        };

        data.entry(*name)
            .or_insert_with(Vec::new)
            .push(raw.codepoints);
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
        let Some(name) = RAW_PROPERTIES
            .iter()
            .find(|prop| **prop == raw.property.as_str())
        else {
            continue;
        };

        data.entry(*name)
            .or_insert_with(Vec::new)
            .push(raw.codepoints);
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

    for row in UnicodeDataExpander::new(raw_unicode_data) {
        let general_category = match row.general_category.as_str() {
            "Nd" => "N",
            "Nl" => "N",
            "No" => "N",
            other => other,
        };

        let Some(name) = RAW_PROPERTIES
            .iter()
            .find(|prop| **prop == general_category)
        else {
            continue;
        };

        data.entry(*name)
            .or_insert_with(Vec::new)
            .push(Codepoints::Single(row.codepoint));
    }

    println!("Raw UCD data parsing finished.");

    let mut data = data
        .into_iter()
        .map(|(prop_name, codepoints)| {
            (
                prop_name,
                codepoints
                    .into_iter()
                    .flat_map(|codepoints| match codepoints {
                        Codepoints::Single(codepoint) => codepoint
                            .scalar()
                            .map(|ch| (ch as u32..ch as u32 + 1))
                            .into_iter()
                            .collect::<Vec<_>>(),
                        Codepoints::Range(c) => c
                            .into_iter()
                            .flat_map(|codepoint| {
                                codepoint.scalar().map(|ch| (ch as u32..ch as u32 + 1))
                            })
                            .collect::<Vec<_>>(),
                    })
                    .collect::<Vec<Range<u32>>>(),
            )
        })
        .collect::<AHashMap<_, _>>();

    for ranges in data.values_mut() {
        loop {
            let mut result = Vec::new();
            let mut indices = 0..(ranges.len() - 1);
            let mut insert_last = true;

            while let Some(index) = indices.next() {
                let current = ranges[index].clone();
                let next = ranges[index + 1].clone();

                if next.start == current.end {
                    if indices.next().is_none() {
                        // We're merging the last element
                        insert_last = false;
                    }

                    result.push(current.start..next.end);
                } else {
                    insert_last = true;
                    result.push(current);
                }
            }

            if insert_last {
                result.push(ranges.last().unwrap().clone());
            }

            if result.len() == ranges.len() {
                *ranges = result;
                break;
            } else {
                *ranges = result;
            }
        }

        let mut last_end = None;
        for range in ranges {
            if let Some(last) = last_end {
                assert!(range.start > last, "{:?}", range);
            }

            last_end = Some(range.end);
        }
    }

    data
}

fn print_chunk_array(output: &mut String, chunks: &[u64]) {
    output.push_str("[\n");

    let mut line_length = 8;
    output.push_str("        ");

    for chunk in chunks {
        match line_length >= 80 {
            true => {
                line_length = 8;
                output.push_str("\n        ");
            }

            false => {
                if line_length > 8 {
                    line_length += 1;
                    output.push_str(" ");
                }
            }
        }

        let chunk = format!("0x{:016x},", chunk);

        line_length += chunk.len();
        output.push_str(&chunk);
    }

    output.push_str("\n    ]");
}

/*

def compute_trie(rawdata, chunksize):
    root = []
    childmap = {}
    child_data = []
    for i in range(len(rawdata) / chunksize):
        data = rawdata[i * chunksize: (i + 1) * chunksize]
        child = '|'.join(map(str, data))
        if child not in childmap:
            childmap[child] = len(childmap)
            child_data.extend(data)
        root.append(childmap[child])
    return (root, child_data)
 */
