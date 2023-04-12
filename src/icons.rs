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

type ThemeName = String;
type ThemePath = String;

#[derive(Default)]
pub struct Icons {
    cache: HashMap<Lookup, PathBuf>,
    themes: HashMap<ThemeName, Theme>
}

#[derive(Clone, Debug, Default)]
struct Theme {
    name: String,
    directories: Vec<String>,
    // TODO: Respect this
    inherits: Vec<String>,
    context: HashMap<ThemePath, ThemeContext>
}

#[derive(Clone, Debug, Default)]
struct ThemeContext {
    context: String,
    size: i32,
    r#type: ThemeType
}

#[derive(Copy, Clone, Debug, Default)]
enum ThemeType {
    #[default]
    Fixed,
    Scalable,
    Threshold
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

        let theme = if let Some(theme) = self.themes.get(theme) {
            theme.clone()
        } else {
            let theme = Theme::load(theme)
                .ok_or(IconLookupFailure::ThemeMissing(theme.to_owned()))?;

            self.themes.insert(theme.name.clone(), theme.clone());
            theme
        };

        let fallback = if let Some(theme) = self.themes.get("hicolor") {
            theme.clone()
        } else {
            let theme = Theme::load("hicolor")
                .ok_or(IconLookupFailure::ThemeMissing("hicolor".to_owned()))?;

            self.themes.insert("hicolor".to_owned(), theme.clone());
            theme
        };

        if let Some(icon) = find_icon(icon, size, scale, &theme, &fallback) {
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

fn find_icon(icon: &str, size: i32, scale: i32, theme: &Theme, fallback: &Theme) -> Option<PathBuf> {
    let filename = find_icon_helper(icon, size, scale, theme)?;
    if filename.exists() {
        return Some(filename);
    }

    let fallback_filename = find_icon_helper(icon, size, scale, fallback)?;
    if fallback_filename.exists() {
        return Some(fallback_filename);
    }

    lookup_fallback_icon(icon)
}

fn find_icon_helper(icon: &str, size: i32, scale: i32, theme: &Theme) -> Option<PathBuf> {
    let subdir_list = &theme.directories;
    let basename_list = get_basename_list();
    let mut closest_filename = None;
    let mut minimal_size = i32::MAX;

    for subdir in subdir_list {
        for directory in &basename_list {
            for extension in &["png", "svg", "xpm"] {
                let subdir_matches_size = directory_matches_size(&subdir, size, scale, theme);
                let filename = Path::new(directory)
                    .join(&theme.name)
                    .join(&subdir)
                    .join(icon)
                    .with_extension(extension);

                if filename.exists() && subdir_matches_size {
                    return Some(filename);
                }

                if let Some(distance) = directory_size_distance(&subdir, size, scale, theme) {
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

    /*
    if let Some(parent) = get_parent_theme(&theme.name) {
        return find_icon_helper(icon, size, scale, &parent);
    }
    */

    None
}

fn directory_matches_size(subdir: &str, size: i32, scale: i32, theme: &Theme) -> bool {
    if let Some(context) = theme.context.get(subdir) {
        match context.r#type {
            ThemeType::Fixed => size == context.size && scale == 1,
            ThemeType::Scalable => {
                let min_size = context.size / 2;
                let max_size = context.size * 2;
                min_size <= size * scale && size * scale <= max_size
            },
            ThemeType::Threshold => {
                let threshold = context.size / 4;
                context.size - threshold <= size * scale && size * scale <= context.size + threshold
            },
            _ => false
        }
    } else {
        false
    }
}

fn directory_size_distance(subdir: &str, size: i32, scale: i32, theme: &Theme) -> Option<i32> {
    let context = theme.context.get(subdir)?;

    match context.r#type {
        ThemeType::Fixed => Some(i32::abs(context.size * scale - size)),
        ThemeType::Scalable => {
            let min_size = context.size / 2;
            let max_size = context.size * 2;
            if size * scale < min_size {
                Some(min_size * scale - size * scale)
            } else if size * scale > max_size {
                Some(size * scale - max_size * scale)
            } else {
                None
            }
        },
        ThemeType::Threshold => {
            let threshold = context.size / 4;
            if size * scale < (context.size - threshold) {
                Some((context.size - threshold) * scale - size * scale)
            } else if size * scale > (context.size + threshold) {
                Some(size * scale - (context.size + threshold) * scale)
            } else {
                None
            }
        },
        _ => None
    }
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

fn get_parent_theme(theme: &str) -> Option<String> {
    // FIXME: The spec mentions parent themes, but doesn't explain them at all
    None
}

#[derive(Debug)]
pub enum IconLookupFailure {
    ThemeMissing(ThemeName),
    IconResolutionFailed(Lookup)
}

impl Theme {
    fn load(name: &str) -> Option<Self> {
        let theme_ini = get_basename_list().into_iter().find_map(|mut base| {
            base.push(name);
            base.push("index.theme");

            if base.exists() {
                return Some(fs::read_to_string(base).ok()?)
            }

            None
        })?;

        let items: Vec<(String, Vec<String>)> = theme_ini.lines().fold(vec![], |mut items, line| {
            if line.starts_with("[") {
                items.push((line[1..line.len() - 1].to_string(), vec![]));
            } else {
                if let Some(mut item) = items.last_mut() {
                    item.1.push(line.to_owned());
                }
            }
            items
        });

        let mut theme = Theme::default();
        theme.name = name.to_owned();

        for (entry, lines) in items {
            match &*entry {
                "Icon Theme" => {
                    for line in lines {
                        if line.starts_with("Directories=") {
                            theme.directories = line
                                .replacen("Directories=", "", 1)
                                .split(",")
                                .filter(|s| s.len() > 0)
                                .map(|s| s.to_owned())
                                .collect();
                        }

                        if line.starts_with("Inherits=") {
                            theme.inherits = line
                                .replacen("Inherits==", "", 1)
                                .split(",")
                                .filter(|s| s.len() > 0)
                                .map(|s| s.to_owned())
                                .collect();
                        }
                    }
                },
                _ => {
                    let mut context = ThemeContext::default();

                    for line in lines {
                        if line.starts_with("Size=") {
                            context.size = line.replacen("Size=", "", 1).parse::<i32>().unwrap();
                        }

                        if line.starts_with("Type=") {
                            match line.trim() {
                                "Type=Fixed" => context.r#type = ThemeType::Fixed,
                                "Type=Scalable" => context.r#type = ThemeType::Scalable,
                                "Type=Threshold" => context.r#type = ThemeType::Threshold,
                                _ => {}
                            }
                        }

                        if line.starts_with("Context") {
                            context.context = line.replacen("Context=", "", 1);
                        }
                    }

                    theme.context.insert(entry, context);
                }
            }
        }
        Some(theme)
    }
}
