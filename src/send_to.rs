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

use std::process::Command;
use std::thread;
use std::ffi::OsString;
use std::path::Path;

pub struct SendTo {}

impl SendTo {
    #[cfg(not(target_os = "linux"))]
    pub fn email(_from: &Path, _files: &[String]) {
        // TODO
        log::error!("SendTo::email not implemented for {}", std::env::consts::OS);
    }

    #[cfg(target_os = "linux")]
    pub fn email(from: &Path, files: &[String]) {
        let files = files.iter()
            .map(|f| vec! [
                 "--attach".to_string(),
                 from.join(f).display().to_string()
            ])
            .flatten()
            .map(|s| s.into())
            .collect::<Vec<OsString>>();

        thread::spawn(move || {
            Command::new("xdg-email")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .args(files)
                .spawn()
                .unwrap();
        });
    }
}
