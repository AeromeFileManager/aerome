/*
 * Copyright (c) 2023 Jesse Tuchsen
 *
 * This file is part of Aerome.
 *
 * Aerome is free software: you can redistribute it and/or modify it under the terms of the GNU
 * General Public License as published by the Free Software Foundation, version 3 of the License.
 *
 * Aerome is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even
 * the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
 * Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with Aerome. If not, see
 * <https://www.gnu.org/licenses/>.
 */

use std::path::{Path,PathBuf};
use std::fs::File;
use std::io::{Read,Write};
use std::os::unix::fs::PermissionsExt;
use zip::write::{FileOptions,ZipWriter};
use zip::CompressionMethod;

pub struct Compress {}

impl Compress {
    pub fn compress(to: &Path, files: &[PathBuf]) {
        let mut output = File::create(to).unwrap();
        let mut zip_writer = ZipWriter::new(output);

        for file in files {
            let mut file_contents = vec![];
            let mut input_file = File::open(file).unwrap();
            let permissions = input_file.metadata().unwrap().permissions();

            input_file.read_to_end(&mut file_contents).unwrap();

            let options = FileOptions::default()
                .unix_permissions(permissions.mode());
            let name = file.file_name().unwrap().to_str().unwrap();

            zip_writer.start_file(name, options).unwrap();
            zip_writer.write_all(&file_contents).unwrap();
        }

        zip_writer.finish().unwrap();
    }
}
