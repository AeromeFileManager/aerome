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
use fs_extra::TransitProcess;
use fs_extra::dir::{TransitState,TransitProcessResult};

use std::path::{PathBuf};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Cmd {
    Compress {
        to: String,
        files: Vec<String>
    },
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
    FileTransfer(FileTransferCmd),
    Options {
        options: Options
    },
    Rename {
        from: String,
        to: String,
        options: Options
    },
    Settings {
        settings: Settings
    },
    SendTo {
        files: Vec<String>
    },
    Trash(TrashCmd),
    Communicate {
        message: String
    },
    Evaluate {
        item: ConversationItem,
        options: Options
    },
    Window(WindowCmd),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashCmd {
    Put { paths: Vec<PathBuf> },
    Restore { paths: Vec<String> },
    Clear { paths: Option<Vec<String>> }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "window", rename_all = "lowercase")]
pub enum WindowCmd {
    Close,
    Drag,
    Maximize,
    Minimize
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileTransferCmd {
    Start(FileTransferCmdStart),
    Resume(FileTransferCmdResponse)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct FileTransferCmdStart {
    pub parent: PathBuf,
    pub names: Vec<String>,
    pub to: PathBuf,
    pub kind: FileTransferKind
}

#[derive(Copy, Clone, Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileTransferKind {
    Cut,
    #[default]
    Copy
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(tag="action", rename_all = "snake_case")]
pub enum FileTransferCmdResponse {
    Overwrite,
    OverwriteAll,
    Skip,
    SkipAll,
    Retry,
    #[default]
    Abort,
    ContinueOrAbort,
}

impl From<FileTransferCmdResponse> for TransitProcessResult {
    fn from(ftcr: FileTransferCmdResponse) -> Self {
        match ftcr {
            FileTransferCmdResponse::Overwrite => TransitProcessResult::Overwrite,
            FileTransferCmdResponse::OverwriteAll => TransitProcessResult::OverwriteAll,
            FileTransferCmdResponse::Skip => TransitProcessResult::Skip,
            FileTransferCmdResponse::SkipAll => TransitProcessResult::SkipAll,
            FileTransferCmdResponse::Retry => TransitProcessResult::Retry,
            FileTransferCmdResponse::Abort => TransitProcessResult::Abort,
            FileTransferCmdResponse::ContinueOrAbort => TransitProcessResult::ContinueOrAbort
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTransfer {
    pub progress: FileTransferProgress,
    pub from: PathBuf,
    pub to: PathBuf,
    pub state: FileTransferProgressState,
    pub kind: FileTransferKind
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTransferProgress {
    pub copied_bytes: u64,
    pub total_bytes: u64,
    pub file_bytes_copied: u64,
    pub file_total_bytes: u64,
    pub file_name: String,
    pub dir_name: String,
}

impl From<TransitProcess> for FileTransferProgress {
    fn from(p: TransitProcess) -> Self {
        Self {
            copied_bytes: p.copied_bytes,
            total_bytes: p.total_bytes,
            file_bytes_copied: p.file_bytes_copied,
            file_total_bytes: p.file_total_bytes,
            file_name: p.file_name,
            dir_name: p.dir_name,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileTransferProgressState {
    #[default]
    Normal,
    Exists,
    NoAccess,
    Finished
}

impl From<TransitState> for FileTransferProgressState {
    fn from(s: TransitState) -> Self {
        match s {
            TransitState::Normal => Self::Normal,
            TransitState::Exists => Self::Exists,
            TransitState::NoAccess => Self::NoAccess,
        }
    }
}

#[derive(Debug)]
pub enum UserEvent {
    CloseWindow,
    DevTools,
    ExecEval(),
    FileTransferProgress(FileTransfer),
    SetSubscriptionsServer(SubscriptionServer),
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailUpdate {
    pub name: String,
    pub url: Url
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionServer {
    pub url: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "response", rename_all = "camelCase")]
pub enum AiResponse {
    Failure(String),
    Success(String)
}
