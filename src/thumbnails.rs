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
use std::fs::File;
use std::fs;
use std::io::{self,BufWriter};
use url::Url;
use http::uri::Uri;
use image::{GenericImageView,ImageBuffer};
use png::{Encoder};
use chrono::offset::Utc;
use chrono::DateTime;
use derive_more::{From,Error,Display};
use std::thread::{self,JoinHandle};
use std::sync::mpsc::{self,Sender,Receiver};
use wry::application::event_loop::EventLoopProxy;
use crate::{constants,UserEvent,ThumbnailUpdate};

// Relevent standards
// https://specifications.freedesktop.org/thumbnail-spec/thumbnail-spec-latest.html

#[derive(Clone, Debug)]
pub struct Thumbnails {
    cache_dir: PathBuf,
    queue: mpsc::Sender<PathBuf>
}

pub type ThumbnailFile = Vec<u8>;

impl Thumbnails {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> (Self, JoinHandle<()>) {
        let cache_dir = dirs::cache_dir()
            .expect("Cache directory could not be found. It's required for thumbnails");

        let (tx, rx): (Sender<PathBuf>, Receiver<PathBuf>) = mpsc::channel();
        let handle_cache_dir = cache_dir.clone();
        let handle = thread::spawn(move || {
            for path in rx {
                match generate(&handle_cache_dir, &path, ThumbnailSize::Large) {
                    Ok(url) => {
                        proxy.send_event(UserEvent::UpdateThumbnail {
                            thumbnail: ThumbnailUpdate {
                                name: path.file_name().unwrap().to_str().unwrap().into(),
                                url
                            }
                        });
                    },
                    Err(e) => eprintln!("Thumbnail generation failed with error {e:?}"),
                }
            }
        });

        (Self {
            cache_dir,
            queue: tx
        }, handle)
    }

    pub fn find(&self, url: &Uri, preferred_size: ThumbnailSize) -> ThumbnailFile {
        if url.host().is_none() {
            return constants::APP_ICON_MIMETYPE_IMAGE_X_GENERIC.into();
        }

        let hash = url.host().unwrap();

        for size in preferred_size.get_fallback_order() {
            if let Ok(file) = fs::read(get_thumbnail_path(&self.cache_dir, &hash, size)) {
                return file;
            }
        }

        return constants::APP_ICON_MIMETYPE_IMAGE_X_GENERIC.into();
    }

    pub fn generate(&self, from: &Path) {
        self.queue.send(from.to_owned());
    }

    pub fn url_from(&self, path: &Path) -> Option<Url> {
        let hash = get_hash(path);

        if get_thumbnail_path(&self.cache_dir, &hash, ThumbnailSize::Large).exists() {
            Some(Url::parse(&format!("thumbnail://{hash}")).unwrap())
        } else {
            None
        }
    }
}

fn generate(
    cache_dir: &Path,
    from: &Path,
    size: ThumbnailSize) -> Result<Url, ThumbnailGenerationError>
{
    if !from.is_absolute() {
        return Err(ThumbnailGenerationError::Path(PathError(
            "Unable to generate thumbnail. Path is not absolute. '{from:?}'".into())))
    }

    generate_temporary_thumbnail(cache_dir, from, size)
        .and_then(|tmp_image_path| {
            let tmp_stripped = tmp_image_path.to_str().unwrap().replace("__future_tmp__", "");
            fs::rename(tmp_image_path, &tmp_stripped)?;
            Ok(PathBuf::from(tmp_stripped))
        })
        .map(|path| {
            let url = format!("thumbnail://{}", path.file_stem().unwrap().to_str().unwrap());
            Url::parse(&url).unwrap()
        })
}

fn generate_temporary_thumbnail(
    cache_dir: &Path,
    from: &Path,
    size: ThumbnailSize) -> Result<PathBuf, ThumbnailGenerationError>
{
    let img = image::open(from)?;
    let pixel_size = match size {
        ThumbnailSize::Normal => 128,
        ThumbnailSize::Large => 256,
        ThumbnailSize::XLarge => 512,
        ThumbnailSize::XXLarge => 1024,
    };
    let thumbnail = img.thumbnail(pixel_size, pixel_size);
    let (width, height) = thumbnail.dimensions();
    let thumbnail_rgba = thumbnail.to_rgba8();
    let buffer = thumbnail_rgba.into_raw();
    let path_str = from.to_str().ok_or(ThumbnailGenerationError::InvalidPath)?;
    let canonical_url = Url::parse(&format!("file://{path_str}"))?.to_string();

    let tmp_hash = format!("__future_tmp__{}", get_hash(from));
    let path = get_thumbnail_path(&cache_dir, &tmp_hash, size);

    let mut file = File::create(&path)?;
    let writer = BufWriter::new(file);
    let mut encoder = Encoder::new(writer, width, height);

    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.add_text_chunk("Thumb::URI".into(), canonical_url.into())?;

    let modified = fs::metadata(from)?.modified()?;
    let timestamp = DateTime::<Utc>::from(modified).timestamp();

    encoder.add_text_chunk("Thumb::MTime".into(), timestamp.to_string())?;

    let mut writer = encoder.write_header()?;
    writer.write_image_data(&buffer)?;

    Ok(path)
}

fn get_thumbnail_path(cache_dir: &Path, hash: &str, size: ThumbnailSize) -> PathBuf {
    cache_dir.join("thumbnails").join(PathBuf::from(size)).join(hash).with_extension("png")
}

fn get_hash(path: &Path) -> String {
    // NOTE: We need to transform the path_str into a url here to get the proper escapes. This
    // looks like needless indirection but it's not.
    let path_str = path.to_str().unwrap();
    let hash_url = Url::parse(&format!("file://{path_str}")).unwrap();
    format!("{:x}", md5::compute(&hash_url.to_string()))
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

#[derive(Display, Debug, From, Error)]
enum ThumbnailGenerationError {
    IO(io::Error),
    Image(image::ImageError),
    Encoding(png::EncodingError),
    UrlParse(url::ParseError),
    Path(PathError),
    InvalidPath
}

#[derive(Display, Debug)]
struct PathError(String);

impl std::error::Error for PathError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn thumbnails_generation() {
        let (thumbnails, _task) = Thumbnails::new();
        let path = env::current_dir().unwrap()
            .join("assets").join("icon").join("icon")
            .with_extension("png");
        let hash = get_hash(&path);
        let cache_dir = dirs::cache_dir().unwrap();
        let expected = get_thumbnail_path(&cache_dir, &hash, ThumbnailSize::Large);

        fs::remove_file(&expected).unwrap_or_default();

        thumbnails.generate(&path);

        std::thread::sleep(std::time::Duration::from_secs(2));

        assert!(expected.exists());
    }
}
