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

use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::iter;
use std::collections::HashMap;

// Relevant standards
// https://specifications.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html
// https://specifications.freedesktop.org/icon-naming-spec/icon-naming-spec-latest.html

#[derive(Default)]
pub struct Icons {
    cache: HashMap<Lookup, PathBuf>
}

impl Icons {
    pub fn find(
        &mut self,
        theme: &str,
        icon: &str,
        size: i32,
        scale: i32) -> Result<PathBuf, IconLookupFailure>
    {
        let lookup = Lookup::new(theme, icon, size, scale);

        if let Some(path) = self.cache.get(&lookup) {
            return Ok(path.to_owned());
        }

        if let Some(icon) = find_icon(icon, size, scale, theme) {
            self.cache.insert(lookup, icon.clone());
            Ok(icon)
        } else {
            Err(IconLookupFailure::IconResolutionFailed(lookup))
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Hash)]
struct Lookup {
    icon: String,
    size: i32,
    scale: i32,
    theme: String
}

impl Lookup {
    fn new(theme: impl Into<String>, icon: impl Into<String>, size: i32, scale: i32) -> Self {
        Lookup { theme: theme.into(), icon: icon.into(), size, scale }
    }
}

fn find_icon(icon: &str, size: i32, scale: i32, theme: &str) -> Option<PathBuf> {
    let filename = find_icon_helper(icon, size, scale, theme)?;
    if filename.exists() {
        return Some(filename);
    }

    let fallback_filename = find_icon_helper(icon, size, scale, "hicolor")?;
    if fallback_filename.exists() {
        return Some(fallback_filename);
    }

    lookup_fallback_icon(icon)
}

fn find_icon_helper(icon: &str, size: i32, scale: i32, theme: &str) -> Option<PathBuf> {
    let subdir_list = get_subdir_list(theme)?;
    let basename_list = get_basename_list();
    let mut closest_filename = None;
    let mut minimal_size = i32::MAX;

    for subdir in subdir_list {
        for directory in &basename_list {
            for extension in &["png", "svg", "xpm"] {
                let subdir_matches_size = directory_matches_size(&subdir, size, scale, theme);
                let filename = Path::new(directory)
                    .join(theme)
                    .join(&subdir)
                    .join(icon)
                    .with_extension(extension);

                if filename.exists() && subdir_matches_size {
                    return Some(filename);
                }

                if let Some(distance) = directory_size_distance(&subdir, size, scale) {
                    if distance < minimal_size {
                        closest_filename = Some(filename);
                        minimal_size = distance;
                    }
                }
            }
        }
    }

    if let Some(filename) = closest_filename {
        return Some(filename);
    }

    if let Some(parent) = get_parent_theme(theme) {
        return find_icon_helper(icon, size, scale, &parent);
    }

    None
}

fn directory_matches_size(subdir: &str, size: i32, scale: i32, theme: &str) -> bool {
    let (dir_type, dir_size) = get_directory_size_data(subdir, theme)
        .unwrap_or((String::new(), 0));

    if dir_type == "Fixed" {
        return size == dir_size && scale == 1;
    } else if dir_type == "Scalable" {
        let min_size = dir_size / 2;
        let max_size = dir_size * 2;
        return min_size <= size * scale && size * scale <= max_size;
    } else if dir_type == "Threshold" {
        let threshold = dir_size / 4;
        return dir_size - threshold <= size * scale && size * scale <= dir_size + threshold;
    }

    false
}

fn directory_size_distance(subdir: &str, size: i32, scale: i32) -> Option<i32> {
    let (dir_type, dir_size) = get_directory_size_data(subdir, "")?;
    if dir_type == "Fixed" {
        return Some(i32::abs(dir_size * scale - size));
    } else if dir_type == "Scaled" {
        let min_size = dir_size / 2;
        let max_size = dir_size * 2;
        if size * scale < min_size {
            return Some(min_size * scale - size * scale);
        } else if size * scale > max_size {
            return Some(size * scale - max_size * scale);
        }
    } else if dir_type == "Threshold" {
        let threshold = dir_size / 4;
        if size * scale < (dir_size - threshold) {
            return Some((dir_size - threshold) * scale - size * scale);
        } else if size * scale > (dir_size + threshold) {
            return Some(size * scale - (dir_size + threshold) * scale);
        }
    }
    Some(0)
}

fn lookup_fallback_icon(icon: &str) -> Option<PathBuf> {
    let basename_list = get_basename_list();
    for directory in &basename_list {
        for extension in &["png", "svg", "xpm"] {
            let filename = Path::new(directory).join(icon).with_extension(extension);
            if filename.exists() {
                return Some(filename);
            }
        }
    }
    None
}

fn get_theme_index(theme: &str) -> Option<String> {
    for mut base in get_basename_list() {
        base.push(theme);
        base.push("index.theme");

        if base.exists() {
            return fs::read_to_string(base).ok()
        }
    }

    None
}

fn get_subdir_list(theme: &str) -> Option<Vec<String>> {
    get_theme_index(theme).and_then(|theme| theme
        .lines()
        .find(|line| line.starts_with("Directories="))
        .map(|line| line
            .replacen("Directories=", "", 1)
            .split(",")
            .filter(|s| s.len() > 0)
            .map(|s| s.to_owned())
            .collect()
        )
    )
}

fn get_basename_list() -> Vec<PathBuf> {
    let home_icons = env::var("HOME")
        .map(|home| {
            let mut path = PathBuf::from(home);
            path.push(".icons");
            path
        })
        .ok();

    let xdg_data_dirs = env::var("XDG_DATA_DIRS")
        .map(|dirs| dirs.split(':')
            .map(|dir| Some({
                let mut path = PathBuf::from(dir);
                path.push("icons");
                path
            }))
            .collect()
        )
        .unwrap_or_else(|_| Vec::new());
    
    iter::once(home_icons)
        .chain(xdg_data_dirs.into_iter())
        .chain(iter::once(Some(PathBuf::from("/usr/share/pixmaps"))))
        .filter_map(|dir| dir)
        .collect()
}

fn get_directory_size_data(subdir: &str, theme: &str) -> Option<(String, i32)> {
    let theme_index = get_theme_index(theme)?;
    let test = format!("[{subdir}]");
    let mut is_inside_directory_data = false;
    let mut r#type = None;
    let mut size = None;

    for line in theme_index.lines() {
        if !is_inside_directory_data {
            // We haven't found our directory options index yet
            is_inside_directory_data = line == test;
            continue;
        }

        if line.starts_with("[") {
            // We've hit the end of our options window
            break;
        }

        if line.starts_with("Size=") {
            size = Some(line.replacen("Size=", "", 1));
        }

        if line.starts_with("Type=") {
            r#type = Some(line.replacen("Type=", "", 1));
        }
    }

    Some((r#type?, size?.parse::<i32>().ok()?))
}

fn get_parent_theme(theme: &str) -> Option<String> {
    // FIXME: The spec mentions parent themes, but doesn't explain them at all
    None
}

#[derive(Debug)]
pub enum IconLookupFailure {
    ThemeIndexMissing,
    IconResolutionFailed(Lookup)
}
