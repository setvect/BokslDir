use crate::models::operation::{ConflictResolution, OperationProgress};
use std::path::PathBuf;

/// 입력 다이얼로그 목적
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPurpose {
    /// 복사/이동 대상 경로 입력
    OperationDestination,
    /// 경로 직접 이동
    GoToPath,
    /// 압축 파일 생성 경로 입력
    ArchiveCreatePath,
    /// 압축 해제 대상 경로 입력
    ArchiveExtractDestination,
    /// 압축 비밀번호 입력
    ArchivePassword,
}

/// 다이얼로그 종류
#[derive(Debug, Clone)]
pub enum DialogKind {
    /// 입력 다이얼로그 (대상 경로 입력)
    Input {
        title: String,
        prompt: String,
        value: String,
        cursor_pos: usize,
        selected_button: usize, // 0: OK, 1: Cancel
        purpose: InputPurpose,
        base_path: PathBuf,
        completion_candidates: Vec<String>,
        completion_index: Option<usize>,
        mask_input: bool,
    },
    /// 압축 생성 입력 다이얼로그 (경로 + 비밀번호 옵션)
    ArchiveCreateOptions {
        path_value: String,
        path_cursor_pos: usize,
        use_password: bool,
        password_value: String,
        password_cursor_pos: usize,
        password_confirm_value: String,
        password_confirm_cursor_pos: usize,
        focused_field: usize, // 0:path, 1:checkbox, 2:password, 3:confirm, 4:buttons
        selected_button: usize, // 0: OK, 1: Cancel
        base_path: PathBuf,
    },
    /// 확인 다이얼로그 (Yes/No)
    Confirm {
        title: String,
        message: String,
        selected_button: usize, // 0: OK, 1: Cancel
    },
    /// 충돌 다이얼로그 (덮어쓰기/건너뛰기)
    Conflict {
        source_path: PathBuf,
        dest_path: PathBuf,
        selected_option: usize, // 0-4: Overwrite, Skip, OverwriteAll, SkipAll, Cancel
    },
    /// 진행률 다이얼로그
    Progress { progress: OperationProgress },
    /// 에러 다이얼로그
    Error { title: String, message: String },
    /// 메시지 다이얼로그 (정보 표시)
    Message { title: String, message: String },
    /// 삭제 확인 다이얼로그 (Phase 3.3)
    DeleteConfirm {
        items: Vec<String>,
        total_size: String,
        selected_button: usize, // 0: 휴지통, 1: 영구 삭제, 2: 취소
    },
    // Phase 3.4: 기타 파일 작업
    /// 새 디렉토리 생성 입력 다이얼로그
    MkdirInput {
        value: String,
        cursor_pos: usize,
        selected_button: usize, // 0: OK, 1: Cancel
        parent_path: PathBuf,
    },
    /// 이름 변경 입력 다이얼로그
    RenameInput {
        value: String,
        cursor_pos: usize,
        selected_button: usize, // 0: OK, 1: Cancel
        original_path: PathBuf,
    },
    /// 필터 입력 다이얼로그 (Phase 5.2)
    FilterInput {
        value: String,
        cursor_pos: usize,
        selected_button: usize, // 0: OK, 1: Cancel
    },
    /// 단축키 도움말 다이얼로그 (Phase 4)
    Help {
        scroll_offset: usize,
        search_query: String,
        search_cursor: usize,
        search_mode: bool,
    },
    /// 마운트 포인트 선택 다이얼로그 (Phase 5.3)
    MountPoints {
        items: Vec<(String, std::path::PathBuf)>,
        selected_index: usize,
    },
    /// 탭 목록 선택 다이얼로그 (Phase 6.1)
    TabList {
        items: Vec<String>,
        selected_index: usize,
    },
    /// 디렉토리 히스토리 목록 선택 다이얼로그 (Phase 6.2)
    HistoryList {
        items: Vec<(String, std::path::PathBuf, bool)>,
        selected_index: usize,
    },
    /// 북마크 목록 선택 다이얼로그 (Phase 6.3)
    BookmarkList {
        items: Vec<(String, std::path::PathBuf)>,
        selected_index: usize,
    },
    /// 북마크 이름 변경 입력 다이얼로그 (Phase 6.3)
    BookmarkRenameInput {
        value: String,
        cursor_pos: usize,
        selected_button: usize, // 0: OK, 1: Cancel
        bookmark_index: usize,
    },
    /// 압축 파일 내부 목록 미리보기
    ArchivePreviewList {
        archive_name: String,
        items: Vec<(String, String)>,
        selected_index: usize,
        scroll_offset: usize,
        truncated: bool,
    },
    /// 파일 속성 다이얼로그
    Properties {
        name: String,
        path: String,
        file_type: String,
        size: String,
        modified: String,
        permissions: String,
        children_info: Option<String>, // 디렉토리인 경우 하위 항목 수
    },
}

/// 다이얼로그 결과
#[derive(Debug, Clone)]
pub enum DialogResult {
    /// 확인 (입력값 포함)
    Confirm(String),
    /// 취소
    Cancel,
    /// 덮어쓰기
    Overwrite,
    /// 건너뛰기
    Skip,
    /// 모두 덮어쓰기
    OverwriteAll,
    /// 모두 건너뛰기
    SkipAll,
}

impl DialogResult {
    /// ConflictResolution으로 변환
    pub fn to_conflict_resolution(&self) -> Option<ConflictResolution> {
        match self {
            DialogResult::Overwrite => Some(ConflictResolution::Overwrite),
            DialogResult::Skip => Some(ConflictResolution::Skip),
            DialogResult::OverwriteAll => Some(ConflictResolution::OverwriteAll),
            DialogResult::SkipAll => Some(ConflictResolution::SkipAll),
            DialogResult::Cancel => Some(ConflictResolution::Cancel),
            _ => None,
        }
    }
}
