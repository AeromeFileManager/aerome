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

use core::fmt::Display;
use std::path::{Path,PathBuf};
use std::fs;
use crate::constants;
use url::Url;
use http::uri::Uri;

// Relevent standards
// https://specifications.freedesktop.org/thumbnail-spec/thumbnail-spec-latest.html

#[derive(Clone, Debug)]
pub struct Thumbnails {
    cache_dir: PathBuf,
}

pub type ThumbnailFile = Vec<u8>;

impl Thumbnails {
    pub fn new() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .expect("Cache directory could not be found. It's required for thumbnails")
        }
    }

    pub fn find(&self, url: &Uri, preferred_size: ThumbnailSize) -> ThumbnailFile {
        if url.host().is_none() {
            return constants::APP_ICON_MIMETYPE_IMAGE_X_GENERIC.into();
        }

        let hash = url.host().unwrap();
        let thumbnails_dir = self.cache_dir.join("thumbnails");

        for size in preferred_size.get_fallback_order() {
            let path = thumbnails_dir.join(PathBuf::from(size)).join(hash).with_extension("png");

            if let Ok(file) = fs::read(path) {
                return file;
            }
        }

        return constants::APP_ICON_MIMETYPE_IMAGE_X_GENERIC.into();
    }

    pub fn url_from(path: &Path) -> Option<Url> {
        let path_str = path.to_str()?;
        // NOTE: We need to transform the path_str into a url here to get the proper escapes. This
        // looks like needless indirection but it's not.
        let hash_url = Url::parse(&format!("file://{path_str}")).unwrap();
        let hash = format!("{:x}", md5::compute(&hash_url.to_string()));

        Some(Url::parse(&format!("thumbnail://{hash}")).unwrap())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbnailSize {
    Normal,
    Large,
    XLarge,
    XXLarge
}

impl From<ThumbnailSize> for PathBuf {
    fn from(ts: ThumbnailSize) -> PathBuf {
        match ts {
            ThumbnailSize::Normal => PathBuf::from("normal"),
            ThumbnailSize::Large => PathBuf::from("large"),
            ThumbnailSize::XLarge => PathBuf::from("x-large"),
            ThumbnailSize::XXLarge => PathBuf::from("xx-large"),
        }
    }
}

impl ThumbnailSize {
    fn get_fallback_order(&self) -> Vec<ThumbnailSize> {
        match self {
            Self::Normal => vec![ Self::Normal, Self::Large, Self::XLarge, Self::XXLarge ],
            Self::Large => vec![ Self::Large, Self::XLarge, Self::XXLarge, Self::Normal ],
            Self::XLarge => vec![ Self::XLarge, Self::XXLarge, Self::Normal, Self::Large ],
            Self::XXLarge => vec![ Self::XXLarge, Self::Normal, Self::Large, Self::XLarge ],
        }
    }
}
