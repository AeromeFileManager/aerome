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

use serde::{Deserialize,Serialize};
use crate::{ConversationItem,Folder,FileMetadata,Suggestions,Options,Settings};
use std::path::PathBuf;
use url::Url;

#[derive(Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    /// Show developer tools in when not in production
    Dev,
    Initialized,
    Back {
        options: Options
    },
    Forward {
        to: String,
        options: Options
    },
    Jump {
        to: String,
        options: Options
    },
    Options {
        options: Options
    },
    Settings {
        settings: Settings
    },
    Communicate {
        message: String
    },
    Evaluate {
        item: ConversationItem,
        options: Options
    },
    Window(WindowCmd),
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "window", rename_all = "camelCase")]
pub enum WindowCmd {
    Close,
    Drag,
    Maximize,
    Minimize
}

pub enum UserEvent {
    CloseWindow,
    DevTools,
    ExecEval(),
    UpdateFileDeepLook {
        file: FileMetadata
    },
    UpdateFolder {
        folder: Folder,
        script_result: Option<ConversationItem>
    },
    UpdateSuggestions {
        description: Suggestions
    },
    UpdateThumbnail {
        thumbnail: ThumbnailUpdate
    },
    UpdateSettings {
        settings: Settings
    },
    NonexistentFolder {
        path: String
    },
    Ai(AiResponse),
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailUpdate {
    pub name: String,
    pub url: Url
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "response", rename_all = "camelCase")]
pub enum AiResponse {
    Failure(String),
    Success(String)
}
