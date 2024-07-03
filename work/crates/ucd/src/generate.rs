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

////////////////////////////////////////////////////////////////////////////////////////////////////////
// A part of this file's source code is an adaptation of the code from the "Rust" work.               //
//                                                                                                    //
// The original work available here:                                                                  //
// https://github.com/raphlinus/rust/blob/cfaf66c94e29a38cd3264b4a55c85b90213543d9/src/etc/unicode.py //
//                                                                                                    //
// The authors of the original work grant me with a license to their work under the following terms:  //
//                                                                                                    //
//   Permission is hereby granted, free of charge, to any                                             //
//   person obtaining a copy of this software and associated                                          //
//   documentation files (the "Software"), to deal in the                                             //
//   Software without restriction, including without                                                  //
//   limitation the rights to use, copy, modify, merge,                                               //
//   publish, distribute, sublicense, and/or sell copies of                                           //
//   the Software, and to permit persons to whom the Software                                         //
//   is furnished to do so, subject to the following                                                  //
//   conditions:                                                                                      //
//                                                                                                    //
//   The above copyright notice and this permission notice                                            //
//   shall be included in all copies or substantial portions                                          //
//   of the Software.                                                                                 //
//                                                                                                    //
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF                                            //
//   ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED                                          //
//   TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A                                              //
//   PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT                                              //
//   SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY                                         //
//   CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION                                          //
//   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR                                          //
//   IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER                                              //
//   DEALINGS IN THE SOFTWARE.                                                                        //
//                                                                                                    //
// Kindly be advised that the terms governing the distribution of my work are                         //
// distinct from those pertaining to the original work.                                               //
////////////////////////////////////////////////////////////////////////////////////////////////////////

use std::{fs::write, path::PathBuf, process::exit};

use ahash::{AHashMap, AHashSet};

use crate::{parse::parse_raw_data, PropDesc, GENERATED_FILE};

const TOTAL: usize = 0x110000;
const CHUNK: usize = 64;

pub(super) fn generate() {
    let input = parse_raw_data();

    println!("Starting UCD module generation...");

    let output = Emitter::generate(input);

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

struct Emitter {
    output: String,
    line_length: usize,
}

impl Emitter {
    fn generate(input: Vec<(&'static PropDesc, AHashSet<u32>)>) -> String {
        let mut emitter = Self {
            output: String::new(),
            line_length: 0,
        };

        emitter.emit_notice();
        emitter.emit_ucd_interface(&input);
        emitter.emit_trie_type();

        for (prop, code_points) in &input {
            emitter.emit_table(*prop, code_points);
        }

        emitter.output
    }

    fn emit_notice(&mut self) {
        self.write_ln(r#"// This file is generated by "crates/ucd". Do not edit manually."#);
        self.blank_ln();
    }

    fn emit_ucd_interface(&mut self, input: &[(&'static PropDesc, AHashSet<u32>)]) {
        self.write_ln("pub trait UCD {");

        let mut first = true;

        for (prop, _) in input {
            match first {
                true => first = false,
                false => self.blank_ln(),
            }

            self.write("    fn is_ucd_");
            self.write(prop.field_name);
            self.write_ln("(self) -> bool;");
        }

        self.write_ln("}");
        self.blank_ln();

        self.write_ln("impl UCD for char {");

        first = true;

        for (prop, _) in input {
            match first {
                true => first = false,
                false => self.blank_ln(),
            }

            self.write_ln("    #[inline(always)]");
            self.write("    fn is_ucd_");
            self.write(prop.field_name);
            self.write_ln("(self) -> bool {");
            self.write("        ");
            self.write(prop.table_name);
            self.write_ln(".lookup(self as usize)");
            self.write_ln("    }");
        }

        self.write_ln("}");
        self.blank_ln();
    }

    fn emit_trie_type(&mut self) {
        self.write_ln(
            r#"struct UCDTrie {
    r1: [u64; 32],
    r2: [u8; 992],
    r3: &'static [u64],
    r4: [u8; 256],
    r5: &'static [u8],
    r6: &'static [u64],
}

impl UCDTrie {
    #[inline(always)]
    fn lookup(&self, code_point: usize) -> bool {
        if code_point < 0x800 {
            return Self::check_chunk(self.r1[code_point >> 6], code_point);
        }

        if code_point < 0x10000 {
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
}"#,
        );

        self.blank_ln();
    }

    fn emit_table(&mut self, prop: &'static PropDesc, code_points: &AHashSet<u32>) {
        let mut chunks = Vec::new();

        for chunk_index in 0..TOTAL / CHUNK {
            let mut chunk = 0u64;

            for bit in 0..64 {
                let code_point =
                    <u32>::try_from(chunk_index * 64 + bit).expect("Invalid code point.");

                if !code_points.contains(&code_point) {
                    continue;
                }

                chunk |= 1 << bit;
            }

            chunks.push(chunk);
        }

        let r1 = &chunks[0..(0x800 / CHUNK)];
        assert_eq!(r1.len(), 32);

        let (r2, r3) = Self::compute_trie(&chunks[0x800 / CHUNK..0x10000 / CHUNK], 64 / CHUNK);
        assert_eq!(r2.len(), 992);

        let (mid, r6) = Self::compute_trie(&chunks[0x10000 / CHUNK..0x110000 / CHUNK], 64 / CHUNK);
        let (r4, r5) = Self::compute_trie(&mid, 64);

        self.write_ln("#[rustfmt::skip]");
        self.write_ln(&format!("static {}: UCDTrie = UCDTrie {{", prop.table_name));

        self.write("    r1: ");
        self.write_chunk_array(r1, true);
        self.write_ln(",");

        self.write("    r2: ");
        self.write_chunk_array(&r2, false);
        self.write_ln(",");

        self.write("    r3: &");
        self.write_chunk_array(&r3, true);
        self.write_ln(",");

        self.write("    r4: ");
        self.write_chunk_array(&r4, false);
        self.write_ln(",");

        self.write("    r5: &");
        self.write_chunk_array(&r5, false);
        self.write_ln(",");

        self.write("    r6: &");
        self.write_chunk_array(&r6, true);
        self.write_ln(",");

        self.write_ln("};");
        self.blank_ln();
    }

    fn write_chunk_array(&mut self, chunks: &[u64], is_leaf: bool) {
        let digits = match is_leaf {
            true => 16,
            false => 4,
        };

        self.write_ln("[");

        self.write("        ");

        for chunk in chunks {
            if !is_leaf {
                assert!(*chunk <= u8::MAX as u64);
            }

            match self.line_length >= 80 {
                true => {
                    self.blank_ln();
                    self.write("        ");
                }

                false => {
                    if self.line_length > 8 {
                        self.write(" ");
                    }
                }
            }

            self.write("0x");
            self.write(&format!("{chunk:0digits$x},").to_ascii_uppercase());
        }

        self.blank_ln();
        self.write("    ]");
    }

    fn write(&mut self, string: &str) {
        self.output.push_str(&string);
        self.line_length += string.len();
    }

    fn write_ln(&mut self, string: &str) {
        self.output.push_str(string);
        self.blank_ln();
    }

    fn blank_ln(&mut self) {
        self.output.push('\n');
        self.line_length = 0;
    }

    fn compute_trie(chunks: &[u64], chunk_size: usize) -> (Vec<u64>, Vec<u64>) {
        let mut root = Vec::new();
        let mut child = Vec::new();
        let mut map = AHashMap::new();

        for chunk_index in 0..chunks.len() / chunk_size {
            let slice = &chunks[chunk_index * chunk_size..(chunk_index + 1) * chunk_size];
            let map_len = map.len() as u64;

            let index = map.entry(slice).or_insert_with(|| {
                child.extend(slice);
                map_len
            });

            root.push(*index)
        }

        (root, child)
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
}
