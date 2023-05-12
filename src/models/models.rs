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

use serde::{Serialize,Deserialize};
use std::path::PathBuf;
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub path: PathBuf,
    pub files: Vec<FolderListing>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FolderListing {
    pub name: String,
    pub kind: FolderListingType,
    pub graphic: Option<Url>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FolderListingType {
    File,
    Folder,
    Link
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub name: String,
    pub graphic: Option<Url>,
    pub openers: Vec<FileOpener>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileOpener {
    pub name: String,
    pub graphic: Option<Url>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suggestions {
    pub purpose: String,
    pub actions: Vec<Action>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub title: String,
    pub description: String
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub sort: Sort,
    pub sort_folders_first: bool,
    pub sort_show_hidden: bool,
    pub grid_scale: f32
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub account: Option<Account>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Account {
    Direct(AccountDirect),
    Aerome(AccountAerome)
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountDirect(pub String);

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AccountAerome {
    pub active: bool,
    pub email: String,
    pub key: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum Sort {
    #[default]
    #[serde(rename = "a-z")]
    AToZ,
    #[serde(rename = "z-a")]
    ZToA,
    #[serde(rename = "date")]
    Date,
}
