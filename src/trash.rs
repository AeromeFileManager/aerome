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

use std::fs;
use std::process::Command;
use std::path::{PathBuf,Path};
use chrono::{DateTime,Utc,TimeZone};
use dirs;
use crate::constants::APP_NAME;

pub struct Trash {}

#[cfg(target_os = "linux")]
impl Trash {
    // https://specifications.freedesktop.org/trash-spec/trashspec-latest.html
    // This is a piss poor implementation of the above spec, but will do for now

    pub fn new() -> Trash {
        Trash {}
    }

    pub fn put(&self, paths: &[PathBuf]) {
        let (files, info) = get_trash_dirs();
        let now: DateTime<Utc> = Utc::now();
        let timestamp = now.format("%Y-%m-%dT%H:%M:%S").to_string();

        for path in paths {
            if !path.is_absolute() {
                panic!(r#"Only absolute paths are supported, was passed "{:?}""#, path);
            }
        }

        for path in paths {
            let (trashinfo, trashpath) = find_unique_path(&path);
            let meta = TRASH_META_TEMPLATE
                .replace("$PATH", &path.to_str().unwrap())
                .replace("$DATE", &timestamp);

            fs::write(trashinfo, &meta).unwrap();
            fs::rename(path, trashpath).unwrap();
        }
    }

    pub fn restore(&self, paths: &[String]) {
        let (files, info) = get_trash_dirs();

        for path in paths {
            let meta = fs::read_to_string(info.join(format!("{path}.trashinfo"))).unwrap();
            let restore_path = meta.lines()
                .find(|line| line.starts_with("Path="))
                .map(|line| PathBuf::from(line.split('=').last().unwrap()));

            fs::rename(files.join(path), restore_path.unwrap()).unwrap();
            fs::remove_file(info.join(format!("{path}.trashinfo"))).unwrap();
        }
    }

    pub fn clear(&self, paths: Option<&[String]>) {
        let (files, info) = get_trash_dirs();

        match paths {
            Some(paths) => {
                for path in paths {
                    fs::remove_file(info.join(format!("{path}.trashinfo"))).unwrap();
                    fs::remove_file(files.join(path)).unwrap();
                }
            },
            None => {
                fs::remove_dir_all(&files).unwrap();
                fs::remove_dir_all(&info).unwrap();

                fs::create_dir_all(&files).unwrap();
                fs::create_dir_all(&info).unwrap();
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl Trash {
    // TODO: Finish this implementation. I found a script that'll use the Mac OS Finder API's to put
    // items in the trash, but cannot find ways to pull items out. For now this is ok, because the
    // trash icon is a permanent fixture on the Mac OS dock, and the trash page in Aerome is a bit
    // of a hidden feature given we don't have a sidebar or a menu bar yet that would point to it.
    // It's a good stopping point, but I'll have to revisit this later when Aerome is a little more
    // polished.

    pub fn new() -> Trash {
        Trash {}
    }

    pub fn put(&self, paths: &[PathBuf]) {
        let scripts_dir = dirs::data_local_dir().unwrap().join(APP_NAME).join("scripts");

        Command::new(scripts_dir.join("trash.sh"))
            .args(paths)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
    }

    pub fn restore(&self, paths: &[String]) {}
    pub fn clear(&self, paths: Option<&[String]>) {}
}

fn find_unique_path(path: &Path) -> (PathBuf, PathBuf) {
    let (files, info) = get_trash_dirs();

    let mut i = 0;
    loop {
        let name = path.file_name().unwrap().to_str().unwrap();
        let escaped_path = if i > 0 {
            match name.split_once('.') {
                Some((before, after)) => format!("{before}.{i}.{after}"),
                None => format!("{name}.{i}"),
            }
        } else {
            format!("{name}")
        };

        let trashinfo = info.join(format!("{escaped_path}.trashinfo"));
        let trashpath = files.join(&escaped_path);

        if !trashinfo.exists() && !trashpath.exists() {
            return (trashinfo, trashpath);
        }

        i += 1;
    }
}

fn get_trash_dirs() -> (PathBuf, PathBuf) {
    let trash_base_dir = dirs::data_dir().unwrap().join("Trash");
    (trash_base_dir.join("files"), trash_base_dir.join("info"))
}

const TRASH_META_TEMPLATE: &'static str = "\
[Trash Info]
Path=$PATH
DeletionDate=$DATE\
";
