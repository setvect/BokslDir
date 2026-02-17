use super::{DialogKind, InputPurpose};
use crate::models::operation::OperationProgress;
use std::path::PathBuf;

impl DialogKind {
    /// 새 입력 다이얼로그 생성
    pub fn input(
        title: impl Into<String>,
        prompt: impl Into<String>,
        initial: impl Into<String>,
    ) -> Self {
        Self::operation_path_input(title, prompt, initial, PathBuf::from("."))
    }

    /// 파일 작업 대상 경로 입력 다이얼로그 생성
    pub fn operation_path_input(
        title: impl Into<String>,
        prompt: impl Into<String>,
        initial: impl Into<String>,
        base_path: PathBuf,
    ) -> Self {
        let value: String = initial.into();
        let cursor_pos = value.len();
        DialogKind::Input {
            title: title.into(),
            prompt: prompt.into(),
            value,
            cursor_pos,
            selected_button: 0, // OK 기본 선택
            purpose: InputPurpose::OperationDestination,
            base_path,
            completion_candidates: Vec::new(),
            completion_index: None,
            mask_input: false,
        }
    }

    /// 경로 직접 이동 입력 다이얼로그 생성
    pub fn go_to_path_input(initial: impl Into<String>, base_path: PathBuf) -> Self {
        let value: String = initial.into();
        let cursor_pos = value.len();
        DialogKind::Input {
            title: "Go to Path".to_string(),
            prompt: "Path:".to_string(),
            value,
            cursor_pos,
            selected_button: 0,
            purpose: InputPurpose::GoToPath,
            base_path,
            completion_candidates: Vec::new(),
            completion_index: None,
            mask_input: false,
        }
    }

    /// 압축 생성 대상 경로 입력 다이얼로그 생성
    pub fn archive_create_path_input(initial: impl Into<String>, base_path: PathBuf) -> Self {
        let value: String = initial.into();
        let cursor_pos = value.len();
        DialogKind::Input {
            title: "Create Archive".to_string(),
            prompt: "Archive path:".to_string(),
            value,
            cursor_pos,
            selected_button: 0,
            purpose: InputPurpose::ArchiveCreatePath,
            base_path,
            completion_candidates: Vec::new(),
            completion_index: None,
            mask_input: false,
        }
    }

    /// 압축 생성 다이얼로그 생성
    pub fn archive_create_options_input(initial: impl Into<String>, base_path: PathBuf) -> Self {
        let path_value: String = initial.into();
        let path_cursor_pos = path_value.len();
        DialogKind::ArchiveCreateOptions {
            path_value,
            path_cursor_pos,
            use_password: false,
            password_value: String::new(),
            password_cursor_pos: 0,
            password_confirm_value: String::new(),
            password_confirm_cursor_pos: 0,
            focused_field: 0,
            selected_button: 0,
            base_path,
        }
    }

    /// 압축 해제 대상 경로 입력 다이얼로그 생성
    pub fn archive_extract_path_input(initial: impl Into<String>, base_path: PathBuf) -> Self {
        let value: String = initial.into();
        let cursor_pos = value.len();
        DialogKind::Input {
            title: "Extract Archive".to_string(),
            prompt: "Extract to:".to_string(),
            value,
            cursor_pos,
            selected_button: 0,
            purpose: InputPurpose::ArchiveExtractDestination,
            base_path,
            completion_candidates: Vec::new(),
            completion_index: None,
            mask_input: false,
        }
    }

    /// 압축 비밀번호 입력 다이얼로그 생성
    pub fn archive_password_input(title: impl Into<String>) -> Self {
        DialogKind::Input {
            title: title.into(),
            prompt: "Password (empty = none):".to_string(),
            value: String::new(),
            cursor_pos: 0,
            selected_button: 0,
            purpose: InputPurpose::ArchivePassword,
            base_path: PathBuf::from("."),
            completion_candidates: Vec::new(),
            completion_index: None,
            mask_input: true,
        }
    }

    /// 새 확인 다이얼로그 생성
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        DialogKind::Confirm {
            title: title.into(),
            message: message.into(),
            selected_button: 0,
        }
    }

    /// 새 충돌 다이얼로그 생성
    pub fn conflict(source: PathBuf, dest: PathBuf) -> Self {
        DialogKind::Conflict {
            source_path: source,
            dest_path: dest,
            selected_option: 0,
        }
    }

    /// 새 진행률 다이얼로그 생성
    pub fn progress(progress: OperationProgress) -> Self {
        DialogKind::Progress { progress }
    }

    /// 새 에러 다이얼로그 생성
    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        DialogKind::Error {
            title: title.into(),
            message: message.into(),
        }
    }

    /// 새 메시지 다이얼로그 생성
    pub fn message(title: impl Into<String>, message: impl Into<String>) -> Self {
        DialogKind::Message {
            title: title.into(),
            message: message.into(),
        }
    }

    /// 새 삭제 확인 다이얼로그 생성
    pub fn delete_confirm(items: Vec<String>, total_size: impl Into<String>) -> Self {
        DialogKind::DeleteConfirm {
            items,
            total_size: total_size.into(),
            selected_button: 0,
        }
    }

    /// 새 디렉토리 생성 입력 다이얼로그
    pub fn mkdir_input(parent_path: PathBuf) -> Self {
        DialogKind::MkdirInput {
            value: String::new(),
            cursor_pos: 0,
            selected_button: 0,
            parent_path,
        }
    }

    /// 이름 변경 입력 다이얼로그
    pub fn rename_input(original_path: PathBuf, current_name: impl Into<String>) -> Self {
        let name: String = current_name.into();
        let cursor_pos = name.len();
        DialogKind::RenameInput {
            value: name,
            cursor_pos,
            selected_button: 0,
            original_path,
        }
    }

    /// 필터 입력 다이얼로그
    pub fn filter_input(initial: Option<&str>) -> Self {
        let value = initial.unwrap_or("").to_string();
        let cursor_pos = value.len();
        DialogKind::FilterInput {
            value,
            cursor_pos,
            selected_button: 0,
        }
    }

    /// 마운트 포인트 선택 다이얼로그
    pub fn mount_points(items: Vec<(String, std::path::PathBuf)>) -> Self {
        DialogKind::MountPoints {
            items,
            selected_index: 0,
        }
    }

    /// 탭 목록 선택 다이얼로그
    pub fn tab_list(items: Vec<String>, selected_index: usize) -> Self {
        DialogKind::TabList {
            items,
            selected_index,
        }
    }

    /// 히스토리 목록 선택 다이얼로그
    pub fn history_list(
        items: Vec<(String, std::path::PathBuf, bool)>,
        selected_index: usize,
    ) -> Self {
        DialogKind::HistoryList {
            items,
            selected_index,
        }
    }

    /// 북마크 목록 선택 다이얼로그
    pub fn bookmark_list(items: Vec<(String, std::path::PathBuf)>, selected_index: usize) -> Self {
        DialogKind::BookmarkList {
            items,
            selected_index,
        }
    }

    /// 북마크 이름 변경 입력 다이얼로그
    pub fn bookmark_rename_input(value: impl Into<String>, bookmark_index: usize) -> Self {
        let value: String = value.into();
        let cursor_pos = value.len();
        DialogKind::BookmarkRenameInput {
            value,
            cursor_pos,
            selected_button: 0,
            bookmark_index,
        }
    }

    /// 압축 파일 내부 목록 다이얼로그 생성
    pub fn archive_preview_list(
        archive_name: impl Into<String>,
        items: Vec<(String, String)>,
        truncated: bool,
    ) -> Self {
        DialogKind::ArchivePreviewList {
            archive_name: archive_name.into(),
            items,
            selected_index: 0,
            scroll_offset: 0,
            truncated,
        }
    }

    /// 단축키 도움말 다이얼로그
    pub fn help() -> Self {
        DialogKind::Help {
            scroll_offset: 0,
            search_query: String::new(),
            search_cursor: 0,
            search_mode: false,
        }
    }

    /// 파일 속성 다이얼로그
    pub fn properties(
        name: impl Into<String>,
        path: impl Into<String>,
        file_type: impl Into<String>,
        size: impl Into<String>,
        modified: impl Into<String>,
        permissions: impl Into<String>,
        children_info: Option<String>,
    ) -> Self {
        DialogKind::Properties {
            name: name.into(),
            path: path.into(),
            file_type: file_type.into(),
            size: size.into(),
            modified: modified.into(),
            permissions: permissions.into(),
            children_info,
        }
    }
}
