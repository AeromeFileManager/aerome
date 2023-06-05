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
pub use super::prompts::*;

pub const BACKEND_URL: &'static str = "http://localhost:9008";
pub const APP_NAME: &'static str = "aerome";
pub const FILE_MANAGER_THEME_NAME: &'static str = "AeromeFileManager";

pub const APP_DESKTOP_ENTRY: &'static str =
    include_str!("../assets/aerome.desktop");

pub const APP_ICON: &'static [u8] =
    include_bytes!("../assets/icon/icon.png");

pub const APP_ICON_PLACES_FOLDER: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder.png");

pub const APP_ICON_PLACES_FOLDER_PICTURES: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-pictures.png");

pub const APP_ICON_PLACES_FOLDER_DOCUMENTS: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-documents.png");

pub const APP_ICON_PLACES_FOLDER_DOWNLOAD: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-download.png");

pub const APP_ICON_PLACES_FOLDER_DROPBOX: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-dropbox.png");

pub const APP_ICON_PLACES_FOLDER_MUSIC: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-music.png");

pub const APP_ICON_PLACES_FOLDER_PUBLICSHARE: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-publicshare.png");

pub const APP_ICON_PLACES_FOLDER_REMOTE: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-remote.png");

pub const APP_ICON_PLACES_FOLDER_TEMPLATES: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-templates.png");

pub const APP_ICON_PLACES_FOLDER_VIDEOS: &'static [u8] =
    include_bytes!("../assets/Yaru/places/folder-videos.png");

pub const APP_ICON_PLACES_FOLDER_INSYNC: &'static [u8] =
    include_bytes!("../assets/Yaru/places/insync-folder.png");

pub const APP_ICON_PLACES_FOLDER_NETWORK_SERVER: &'static [u8] =
    include_bytes!("../assets/Yaru/places/network-server.png");

pub const APP_ICON_PLACES_FOLDER_NETWORK_WORKGROUP: &'static [u8] =
    include_bytes!("../assets/Yaru/places/network-workgroup.png");

pub const APP_ICON_PLACES_FOLDER_USER_DESKTOP: &'static [u8] =
    include_bytes!("../assets/Yaru/places/user-desktop.png");

pub const APP_ICON_PLACES_FOLDER_USER_HOME: &'static [u8] =
    include_bytes!("../assets/Yaru/places/user-home.png");

pub const APP_ICON_PLACES_FOLDER_USER_TRASH: &'static [u8] =
    include_bytes!("../assets/Yaru/places/user-trash.png");

pub const APP_ICON_SCALABLE_ERROR_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/error-symbolic.svg");

pub const APP_ICON_SCALABLE_GO_DOWN_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/go-down-symbolic.svg");

pub const APP_ICON_SCALABLE_IMAGE_MISSING_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/image-missing-symbolic.svg");

pub const APP_ICON_SCALABLE_OPEN_MENU_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/open-menu-symbolic.svg");

pub const APP_ICON_SCALABLE_STOPWATCH_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/stopwatch-symbolic.svg");

pub const APP_ICON_SCALABLE_VIEW_SORT_ASCENDING_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/view-sort-ascending-symbolic.svg");

pub const APP_ICON_SCALABLE_VIEW_SORT_DESCENDING_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/view-sort-descending-symbolic.svg");

pub const APP_ICON_SCALABLE_WINDOW_CLOSE_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/window-close-symbolic.svg");

pub const APP_ICON_SCALABLE_WINDOW_MAXIMIZE_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/window-maximize-symbolic.svg");

pub const APP_ICON_SCALABLE_WINDOW_MINIMIZE_SYMBOLIC: &'static [u8] =
    include_bytes!("../assets/Yaru/scalable/window-minimize-symbolic.svg");

pub const APP_ICON_MIMETYPE_IMAGE_X_GENERIC: &'static [u8] =
    include_bytes!("../assets/Yaru/mimetypes/image-x-generic.png");

pub const APP_ICON_MIMETYPE_TEXT_X_GENERIC: &'static [u8] =
    include_bytes!("../assets/Yaru/mimetypes/text-x-generic.png");

pub const APP_ICON_MIMETYPE_APPLICATION_X_ZIP: &'static [u8] =
    include_bytes!("../assets/Yaru/mimetypes/application-x-zip.png");

pub const COMMON_MIME_TYPES: [&str; 29] = [
    "folder",
    "folder-music",
    "folder-pictures",
    "folder-documents",
    "folder-download",
    "user-desktop",
    "folder-publicshare",
    "folder-templates",
    "folder-videos",
    "text-plain",
    "text-html",
    "application-json",
    "application-xml",
    "application-pdf",
    "image-jpeg",
    "image-png",
    "audio-mpeg",
    "audio-ogg",
    "video-mp4",
    "video-quicktime",
    "application-octet-stream",
    "application-vnd.ms-excel",
    "application-vnd.ms-powerpoint",
    "application-vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application-x-www-form-urlencoded",
    "multipart-form-data",
    "application-zip",
    "application-javascript",
    "text-css",
];

pub fn install() {
    install_icons();
    install_prompts();
    install_desktop_files();

    if cfg!(target_os = "linux") {
        if let Ok(exe_path) = std::env::current_exe() {
            let aerome = dirs::executable_dir().map(|e| e.join("aerome")).unwrap();
            if !aerome.exists() {
                fs::copy(&exe_path, &aerome).unwrap();
            }
        }
        let _ = Command::new("update-desktop-database").output();
    }
}

#[cfg(target_os = "linux")]
fn install_desktop_files() {
    let app_icon_path = dirs::data_local_dir()
        .map(|data_dir| data_dir.join(APP_NAME).join("icon.png"))
        .unwrap();

    if !app_icon_path.exists() {
        fs::write(&app_icon_path, APP_ICON).unwrap();
    }

    if let Some(applications_dir) = dirs::data_local_dir().map(|d| d.join("applications")) {
        let desktop_entry = APP_DESKTOP_ENTRY.replace("$ICON", app_icon_path.to_str().unwrap());

        fs::create_dir_all(&applications_dir).expect("Could not write to the apps data directory");
        fs::write(applications_dir.join("aerome.desktop"), desktop_entry).unwrap();
    }
}

#[cfg(not(target_os = "linux"))]
fn install_desktop_files() {}

fn install_prompts() {
    let prompts_dir = dirs::data_local_dir()
        .map(|data_dir| data_dir.join(APP_NAME).join("prompts"))
        .expect("Could not find the apps data directory");

    fs::create_dir_all(&prompts_dir).expect("Could not write to the apps data directory");
    fs::write(prompts_dir.join("./communicate.pr"), PROMPT_COMMUNICATE).unwrap();
    fs::write(prompts_dir.join("./summary.pr"), PROMPT_SUMMARY).unwrap();
}

fn install_icons() {
    let icons_dir = dirs::data_local_dir()
        .map(|data_dir| data_dir.join(APP_NAME).join(FILE_MANAGER_THEME_NAME))
        .expect("Could not find the apps data directory");

    let mimetypes_dir = icons_dir.join("mimetypes");
    let places_dir = icons_dir.join("places");
    let scalable_dir = icons_dir.join("scalable");

    fs::create_dir_all(&icons_dir).expect("Could not write to the apps data directory");
    fs::create_dir_all(&mimetypes_dir).unwrap();
    fs::create_dir_all(&places_dir).unwrap();
    fs::create_dir_all(&scalable_dir).unwrap();

    fs::write(
        places_dir.join("./folder.png"),
        APP_ICON_PLACES_FOLDER).unwrap();

    fs::write(
        places_dir.join("./folder-pictures.png"),
        APP_ICON_PLACES_FOLDER_PICTURES).unwrap();

    fs::write(
        places_dir.join("./folder-documents.png"),
        APP_ICON_PLACES_FOLDER_DOCUMENTS).unwrap();

    fs::write(
        places_dir.join("./folder-download.png"),
        APP_ICON_PLACES_FOLDER_DOWNLOAD).unwrap();

    fs::write(
        places_dir.join("./folder-dropbox.png"),
        APP_ICON_PLACES_FOLDER_DROPBOX).unwrap();

    fs::write(
        places_dir.join("./folder-music.png"),
        APP_ICON_PLACES_FOLDER_MUSIC).unwrap();

    fs::write(
        places_dir.join("./folder-pictures.png"),
        APP_ICON_PLACES_FOLDER_PICTURES).unwrap();

    fs::write(
        places_dir.join("./folder-publicshare.png"),
        APP_ICON_PLACES_FOLDER_PUBLICSHARE).unwrap();

    fs::write(
        places_dir.join("./folder-remote.png"),
        APP_ICON_PLACES_FOLDER_REMOTE).unwrap();

    fs::write(
        places_dir.join("./folder-templates.png"),
        APP_ICON_PLACES_FOLDER_TEMPLATES).unwrap();

    fs::write(
        places_dir.join("./folder-videos.png"),
        APP_ICON_PLACES_FOLDER_VIDEOS).unwrap();

    fs::write(
        places_dir.join("./insync-folder.png"),
        APP_ICON_PLACES_FOLDER_INSYNC).unwrap();

    fs::write(
        places_dir.join("./network-server.png"),
        APP_ICON_PLACES_FOLDER_NETWORK_SERVER).unwrap();

    fs::write(
        places_dir.join("./network-workgroup.png"),
        APP_ICON_PLACES_FOLDER_NETWORK_WORKGROUP).unwrap();

    fs::write(
        places_dir.join("./user-desktop.png"),
        APP_ICON_PLACES_FOLDER_USER_DESKTOP).unwrap();

    fs::write(
        places_dir.join("./user-home.png"),
        APP_ICON_PLACES_FOLDER_USER_HOME).unwrap();

    fs::write(
        places_dir.join("./user-trash.png"),
        APP_ICON_PLACES_FOLDER_USER_TRASH).unwrap();

    fs::write(
        scalable_dir.join("./error-symbolic.svg"),
        APP_ICON_SCALABLE_ERROR_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./go-down-symbolic.svg"),
        APP_ICON_SCALABLE_GO_DOWN_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./image-missing-symbolic.svg"),
        APP_ICON_SCALABLE_IMAGE_MISSING_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./open-menu-symbolic.svg"),
        APP_ICON_SCALABLE_OPEN_MENU_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./stopwatch-symbolic.svg"),
        APP_ICON_SCALABLE_STOPWATCH_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./view-sort-ascending-symbolic.svg"),
        APP_ICON_SCALABLE_VIEW_SORT_ASCENDING_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./view-sort-descending-symbolic.svg"),
        APP_ICON_SCALABLE_VIEW_SORT_DESCENDING_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./window-close-symbolic.svg"),
        APP_ICON_SCALABLE_WINDOW_CLOSE_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./window-maximize-symbolic.svg"),
        APP_ICON_SCALABLE_WINDOW_MAXIMIZE_SYMBOLIC).unwrap();

    fs::write(
        scalable_dir.join("./window-minimize-symbolic.svg"),
        APP_ICON_SCALABLE_WINDOW_MINIMIZE_SYMBOLIC).unwrap();

    fs::write(
        mimetypes_dir.join("./image-x-generic.png"),
        APP_ICON_MIMETYPE_IMAGE_X_GENERIC).unwrap();

    fs::write(
        mimetypes_dir.join("./text-x-generic.png"),
        APP_ICON_MIMETYPE_TEXT_X_GENERIC).unwrap();

    fs::write(
        mimetypes_dir.join("./application-x-zip.png"),
        APP_ICON_MIMETYPE_APPLICATION_X_ZIP).unwrap();
}
