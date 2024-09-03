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

use std::{
    fs::{create_dir_all, write},
    path::Path,
    process::{exit, Command},
};

use crate::{UCD_DOWNLOADS_DIR, UCD_RESOURCES, UCD_URL};

pub(super) fn download() {
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
