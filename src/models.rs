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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub path: PathBuf,
    pub files: Vec<File>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub thumbnail: Option<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Description {
    pub purpose: String,
    pub actions: Vec<Action>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub title: String,
    pub description: String
}
