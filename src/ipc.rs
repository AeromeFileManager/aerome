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
use crate::{Folder,Suggestions,Options};

#[derive(Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Cmd {
    Back {
        options: Options
    },
    Forward {
        to: String,
        options: Options
    },
    Options {
        options: Options
    },
    Communicate {
        message: String
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
    ExecEval(),
    UpdateFolder {
        folder: Folder
    },
    UpdateSuggestions {
        description: Suggestions
    },
    Ai(AiResponse),
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "response", rename_all = "camelCase")]
pub enum AiResponse {
    Failure(String),
    Success(String)
}
