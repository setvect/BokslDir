//! 다이얼로그 시스템 (Phase 3.2)
//!
//! 파일 복사/이동 작업에 필요한 다이얼로그 위젯 정의

#![allow(dead_code)]

use crate::core::actions::generate_help_entries;
use crate::models::operation::{ConflictResolution, OperationProgress};
use crate::ui::Theme;
use crate::utils::formatter::format_file_size;
use crate::utils::path_display;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Widget, Wrap},
};
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthStr;

/// 다이얼로그 내부 좌우 패딩 (border 안쪽 여백)
const DIALOG_H_PADDING: u16 = 2;
/// 다이얼로그 내부 상단 패딩 (border 아래 여백)
const DIALOG_V_PADDING: u16 = 1;

fn contains_case_insensitive(text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    text.to_lowercase().contains(&needle.to_lowercase())
}

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

/// 다이얼로그 위젯
pub struct Dialog<'a> {
    kind: &'a DialogKind,
    bg_color: Color,
    fg_color: Color,
    border_color: Color,
    title_color: Color,
    button_bg: Color,
    button_fg: Color,
    button_selected_bg: Color,
    button_selected_fg: Color,
    input_bg: Color,
    progress_filled: Color,
    progress_unfilled: Color,
}

impl<'a> Default for Dialog<'a> {
    fn default() -> Self {
        static DEFAULT_KIND: DialogKind = DialogKind::Message {
            title: String::new(),
            message: String::new(),
        };
        Self {
            kind: &DEFAULT_KIND,
            bg_color: Color::Rgb(45, 45, 48),
            fg_color: Color::Rgb(212, 212, 212),
            border_color: Color::Rgb(0, 120, 212),
            title_color: Color::Rgb(0, 120, 212),
            button_bg: Color::Rgb(60, 60, 60),
            button_fg: Color::Rgb(212, 212, 212),
            button_selected_bg: Color::Rgb(0, 120, 212),
            button_selected_fg: Color::White,
            input_bg: Color::Rgb(30, 30, 30),
            progress_filled: Color::Rgb(0, 120, 212),
            progress_unfilled: Color::Rgb(60, 60, 60),
        }
    }
}

impl<'a> Dialog<'a> {
    pub fn new(kind: &'a DialogKind) -> Self {
        Self {
            kind,
            ..Default::default()
        }
    }

    /// 테마 적용
    pub fn theme(mut self, theme: &Theme) -> Self {
        self.bg_color = theme.panel_bg.to_color();
        self.fg_color = theme.fg_primary.to_color();
        self.border_color = theme.panel_active_border.to_color();
        self.title_color = theme.accent.to_color();
        self.button_bg = theme.command_bar_bg.to_color();
        self.button_fg = theme.fg_primary.to_color();
        self.button_selected_bg = theme.file_selected_bg.to_color();
        self.button_selected_fg = theme.file_selected.to_color();
        self.input_bg = theme.bg_primary.to_color();
        self.progress_filled = theme.accent.to_color();
        self.progress_unfilled = theme.panel_inactive_border.to_color();
        self
    }

    /// 다이얼로그 영역 계산 (화면 중앙, 반응형)
    fn calculate_area(&self, screen: Rect) -> Rect {
        let sw = screen.width;
        let sh = screen.height;

        let (width, height) = match self.kind {
            DialogKind::Input { .. } => {
                let w = ((sw as f32 * 0.72) as u16).clamp(56, 110);
                let h = 12u16;
                (w, h)
            }
            DialogKind::ArchiveCreateOptions { .. } => {
                let w = ((sw as f32 * 0.72) as u16).clamp(56, 110);
                let h = 15u16;
                (w, h)
            }
            DialogKind::MkdirInput { .. }
            | DialogKind::RenameInput { .. }
            | DialogKind::BookmarkRenameInput { .. }
            | DialogKind::FilterInput { .. } => (50u16.min(sw.saturating_sub(4)).max(30), 7u16),
            DialogKind::Confirm { .. } => (40u16.min(sw.saturating_sub(4)).max(25), 8u16),
            DialogKind::Conflict { .. } => (55u16.min(sw.saturating_sub(4)).max(35), 15u16),
            DialogKind::Progress { .. } => (56u16.min(sw.saturating_sub(4)).max(36), 12u16),
            DialogKind::Error { message, .. } | DialogKind::Message { message, .. } => {
                let lines = message.lines().count().max(1);
                let w = 50u16.min(sw.saturating_sub(4)).max(30);
                let h = (6 + lines as u16).min(sh.saturating_sub(4)).max(6);
                (w, h)
            }
            DialogKind::DeleteConfirm { items, .. } => {
                let list_lines = items.len().min(10) as u16;
                let w = 45u16.min(sw.saturating_sub(4)).max(30);
                let h = (7 + list_lines).min(sh.saturating_sub(4)).max(8);
                (w, h)
            }
            DialogKind::Help { .. } => {
                let w = 60u16.min(sw.saturating_sub(4)).max(40);
                let h = sh.saturating_sub(6).max(15);
                (w, h)
            }
            DialogKind::MountPoints { items, .. } => {
                let list_lines = items.len().min(15) as u16;
                let w = 50u16.min(sw.saturating_sub(4)).max(30);
                let h = (4 + list_lines).min(sh.saturating_sub(4)).max(6);
                (w, h)
            }
            DialogKind::TabList { items, .. } => {
                let list_lines = items.len().min(10) as u16;
                let w = 45u16.min(sw.saturating_sub(4)).max(30);
                let h = (4 + list_lines).min(sh.saturating_sub(4)).max(6);
                (w, h)
            }
            DialogKind::HistoryList { items, .. } => {
                let list_lines = items.len().min(12) as u16;
                let w = 70u16.min(sw.saturating_sub(4)).max(40);
                let h = (4 + list_lines).min(sh.saturating_sub(4)).max(8);
                (w, h)
            }
            DialogKind::BookmarkList { items, .. } => {
                let list_lines = items.len().min(12) as u16;
                let w = 70u16.min(sw.saturating_sub(4)).max(40);
                let h = (4 + list_lines).min(sh.saturating_sub(4)).max(8);
                (w, h)
            }
            DialogKind::ArchivePreviewList { items, .. } => {
                let list_lines = items.len().min(16) as u16;
                let w = 90u16.min(sw.saturating_sub(4)).max(48);
                let h = (5 + list_lines).min(sh.saturating_sub(4)).max(10);
                (w, h)
            }
            DialogKind::Properties { children_info, .. } => {
                let base = if children_info.is_some() { 12u16 } else { 11 };
                let w = 80u16.min(sw.saturating_sub(8)).max(40);
                (w, base)
            }
        };

        let width = width.min(sw.saturating_sub(4));
        let height = height.min(sh.saturating_sub(4));

        let x = screen.x + (sw.saturating_sub(width)) / 2;
        let y = screen.y + (sh.saturating_sub(height)) / 2;

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// 버튼 렌더링 헬퍼
    fn render_button(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        label: &str,
        is_selected: bool,
    ) -> u16 {
        let (bg, fg) = if is_selected {
            (self.button_selected_bg, self.button_selected_fg)
        } else {
            (self.button_bg, self.button_fg)
        };

        let padded_label = format!(" {} ", label);
        let width = padded_label.width() as u16;
        let style = Style::default().fg(fg).bg(bg);

        buf.set_string(x, y, &padded_label, style);

        // Wide character(한글 등) continuation cell의 배경색 보정
        for i in 0..width {
            if let Some(cell) = buf.cell_mut((x + i, y)) {
                cell.set_bg(bg);
            }
        }

        width
    }

    /// 입력 다이얼로그 렌더링
    #[allow(clippy::too_many_arguments)]
    fn render_input(
        &self,
        buf: &mut Buffer,
        area: Rect,
        title: &str,
        prompt: &str,
        value: &str,
        purpose: InputPurpose,
        completion_candidates: &[String],
        completion_index: Option<usize>,
        cursor_pos: usize,
        selected_button: usize,
        show_suggestions_panel: bool,
        mask_input: bool,
    ) {
        // 테두리
        let block = Block::default()
            .title(format!(" {} ", title))
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        // 프롬프트
        let prompt_style = Style::default().fg(self.fg_color);
        buf.set_string(inner.x, inner.y, prompt, prompt_style);

        // 입력 필드 배경
        let input_y = inner.y + 1;
        let input_width = inner.width;
        for x in inner.x..inner.x + input_width {
            if let Some(cell) = buf.cell_mut((x, input_y)) {
                cell.set_bg(self.input_bg);
            }
        }

        // 입력값 표시 (unicode-width 기반)
        // cursor_pos는 바이트 인덱스, 화면 표시는 display width 기반
        let max_display = input_width as usize - 2;
        let visible_value = if mask_input {
            "*".repeat(value.chars().count())
        } else {
            value.to_string()
        };
        let visible_cursor_pos = if mask_input {
            value[..cursor_pos].chars().count()
        } else {
            cursor_pos
        };
        let value_display_width = UnicodeWidthStr::width(visible_value.as_str());

        // 스크롤 처리: 커서가 보이도록 표시 시작점 결정
        let (display_value, cursor_display_col) = if value_display_width <= max_display {
            // 전체 표시 가능
            let cursor_col: usize = visible_value[..visible_cursor_pos]
                .chars()
                .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                .sum();
            (visible_value.as_str(), cursor_col)
        } else {
            // 스크롤 필요: 커서 위치를 기준으로 표시 범위 계산
            let cursor_col_from_start: usize = visible_value[..visible_cursor_pos]
                .chars()
                .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                .sum();

            if cursor_col_from_start < max_display {
                // 커서가 앞쪽이면 앞에서부터 표시
                (visible_value.as_str(), cursor_col_from_start)
            } else {
                // 커서가 화면 밖이면 커서가 오른쪽 끝에 오도록 스크롤
                let mut start_byte = 0;
                let mut width_sum = 0;
                // 뒤에서부터 max_display만큼의 너비를 찾음
                let target_start_width = cursor_col_from_start.saturating_sub(max_display - 1);
                for (i, c) in visible_value.char_indices() {
                    let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                    if width_sum >= target_start_width {
                        start_byte = i;
                        break;
                    }
                    width_sum += cw;
                }
                let display = &visible_value[start_byte..];
                let cursor_col = cursor_col_from_start - width_sum;
                (display, cursor_col)
            }
        };
        let value_style = Style::default().fg(self.fg_color).bg(self.input_bg);
        buf.set_string(inner.x + 1, input_y, display_value, value_style);

        // 커서 표시
        let cursor_x = inner.x + 1 + cursor_display_col as u16;
        if cursor_x < inner.x + input_width - 1 {
            if let Some(cell) = buf.cell_mut((cursor_x, input_y)) {
                if cursor_pos < value.len() {
                    // 문자가 있는 위치: 반전 스타일
                    cell.set_style(Style::default().fg(self.input_bg).bg(self.fg_color));
                } else {
                    // 문자열 끝: 블록 커서 문자 표시
                    cell.set_char('▏');
                    cell.set_style(Style::default().fg(self.fg_color).bg(self.input_bg));
                }
            }
        }

        // 자동완성 목록 (표시 가능한 높이만 렌더, 선택 항목 기준 스크롤)
        if show_suggestions_panel && !mask_input && inner.height >= 5 {
            let title_y = inner.y + 2;
            let list_y = inner.y + 3;
            let button_y = area.y + area.height.saturating_sub(2);
            let show_hint = purpose == InputPurpose::GoToPath;
            let list_bottom_y = if show_hint {
                button_y.saturating_sub(1)
            } else {
                button_y
            };
            let visible_rows = list_bottom_y.saturating_sub(list_y) as usize;
            let total_candidates = completion_candidates.len();
            if total_candidates > 0 {
                let selected = completion_index.unwrap_or(0).min(total_candidates - 1);
                let selected_display = selected + 1;

                let title_style = Style::default()
                    .fg(self.border_color)
                    .bg(self.bg_color)
                    .add_modifier(Modifier::DIM);
                let title = format!("Suggestions ({}/{})", selected_display, total_candidates);
                buf.set_string(inner.x, title_y, title, title_style);

                if visible_rows > 0 {
                    let mut scroll = 0usize;
                    if selected >= visible_rows {
                        scroll = selected + 1 - visible_rows;
                    }

                    for (row, candidate) in completion_candidates
                        .iter()
                        .enumerate()
                        .skip(scroll)
                        .take(visible_rows)
                    {
                        let y = list_y + (row - scroll) as u16;
                        let marker = if row == selected { "> " } else { "  " };
                        let content_width =
                            inner.width.saturating_sub(marker.width() as u16) as usize;
                        let text = path_display::truncate_middle(candidate, content_width);
                        let line = format!("{}{}", marker, text);
                        let style = if row == selected {
                            Style::default()
                                .fg(self.button_selected_fg)
                                .bg(self.button_selected_bg)
                        } else {
                            Style::default().fg(self.fg_color).bg(self.bg_color)
                        };
                        buf.set_string(inner.x, y, line, style);
                    }
                }
            }

            if show_hint {
                let hint = "Tab:Apply suggestion  Shift+Tab/Up/Down:Select";
                let hint_y = button_y.saturating_sub(1);
                let hint_x = inner.x + (inner.width.saturating_sub(hint.width() as u16)) / 2;
                buf.set_string(
                    hint_x,
                    hint_y,
                    hint,
                    Style::default()
                        .fg(self.border_color)
                        .bg(self.bg_color)
                        .add_modifier(Modifier::DIM),
                );
            }
        }

        // 버튼
        let button_y = area.y + area.height.saturating_sub(2);
        let ok_width = self.render_button(buf, inner.x, button_y, "OK", selected_button == 0);
        self.render_button(
            buf,
            inner.x + ok_width + 2,
            button_y,
            "Cancel",
            selected_button == 1,
        );
    }

    /// 확인 다이얼로그 렌더링
    fn render_confirm(
        &self,
        buf: &mut Buffer,
        area: Rect,
        title: &str,
        message: &str,
        selected_button: usize,
    ) {
        // 테두리
        let block = Block::default()
            .title(format!(" {} ", title))
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        // 메시지
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(self.fg_color))
            .wrap(Wrap { trim: true });
        let msg_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.saturating_sub(2),
        };
        paragraph.render(msg_area, buf);

        // 버튼 (하단 중앙)
        let button_y = area.y + area.height - 2;
        let buttons_width = 14; // "[ OK ]  [Cancel]"
        let button_x = area.x + (area.width.saturating_sub(buttons_width)) / 2;

        let ok_width = self.render_button(buf, button_x, button_y, "OK", selected_button == 0);
        self.render_button(
            buf,
            button_x + ok_width + 2,
            button_y,
            "Cancel",
            selected_button == 1,
        );
    }

    /// 충돌 다이얼로그 렌더링
    fn render_conflict(
        &self,
        buf: &mut Buffer,
        area: Rect,
        source: &Path,
        dest: &Path,
        selected_option: usize,
    ) {
        // 테두리
        let block = Block::default()
            .title(" File Exists ")
            .title_style(
                Style::default()
                    .fg(Color::Rgb(255, 165, 0))
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(255, 165, 0)))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        let msg_style = Style::default().fg(self.fg_color);
        let path_style = Style::default().fg(Color::Rgb(86, 156, 214));
        let label_style = Style::default().fg(Color::Rgb(128, 128, 128));

        // 소스 파일 표시
        buf.set_string(inner.x, inner.y, "Source:", label_style);
        let source_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        buf.set_string(inner.x + 8, inner.y, source_name, path_style);

        // 대상 경로 표시
        buf.set_string(inner.x, inner.y + 2, "Target already exists:", msg_style);
        let truncated_path = path_display::truncate_path_buf(dest, inner.width as usize);
        buf.set_string(inner.x, inner.y + 3, &truncated_path, path_style);

        // 옵션 버튼들 (2줄로 배치)
        // 첫 번째 줄: Overwrite, Skip
        let row1_options = ["Overwrite", "Skip"];
        let button_y1 = inner.y + 6;
        let mut x = inner.x;

        for (i, option) in row1_options.iter().enumerate() {
            let width = self.render_button(buf, x, button_y1, option, selected_option == i);
            x += width + 1;
        }

        // 두 번째 줄: Overwrite All, Skip All, Cancel
        let row2_options = ["Overwrite All", "Skip All", "Cancel"];
        let button_y2 = inner.y + 8;
        x = inner.x;

        for (i, option) in row2_options.iter().enumerate() {
            let width = self.render_button(buf, x, button_y2, option, selected_option == i + 2);
            x += width + 1;
        }
    }

    /// 진행률 다이얼로그 렌더링
    fn render_progress(&self, buf: &mut Buffer, area: Rect, progress: &OperationProgress) {
        let title = format!(" {} ", progress.operation_type.name());

        // 테두리
        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        // 현재 파일
        let file_style = Style::default().fg(self.fg_color);
        let truncated = path_display::truncate_middle(&progress.current_file, inner.width as usize);
        buf.set_string(inner.x, inner.y, &truncated, file_style);

        // 진행률 바
        let gauge_area = Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: 1,
        };
        let percent = progress.percentage();
        let gauge = Gauge::default()
            .ratio(percent as f64 / 100.0)
            .gauge_style(
                Style::default()
                    .fg(self.progress_filled)
                    .bg(self.progress_unfilled),
            )
            .label(format!("{}%", percent));
        gauge.render(gauge_area, buf);

        // 파일 카운트
        let count_text = format!(
            "{} / {} files",
            progress.files_completed, progress.total_files
        );
        buf.set_string(inner.x, inner.y + 4, &count_text, file_style);

        // 바이트 카운트
        let size_text = format!(
            "{} / {}",
            format_file_size(progress.bytes_copied),
            format_file_size(progress.total_bytes)
        );
        buf.set_string(inner.x, inner.y + 5, &size_text, file_style);

        let remaining = progress
            .total_files
            .saturating_sub(progress.items_processed);
        let processed_text = format!(
            "Processed: {}  Remaining: {}  Failed: {}",
            progress.items_processed, remaining, progress.items_failed
        );
        buf.set_string(inner.x, inner.y + 6, &processed_text, file_style);

        // 속도 / ETA
        let speed_eta = format!(
            "{}  ETA: {}",
            progress.format_speed(),
            progress.format_eta()
        );
        let speed_style = Style::default().fg(Color::Rgb(100, 180, 100));
        buf.set_string(inner.x, inner.y + 7, &speed_eta, speed_style);

        // Esc 안내
        let hint_style = Style::default().fg(Color::Rgb(128, 128, 128));
        buf.set_string(inner.x, inner.y + 9, "Press Esc to cancel", hint_style);
    }

    /// 삭제 확인 다이얼로그 렌더링
    fn render_delete_confirm(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[String],
        total_size: &str,
        selected_button: usize,
    ) {
        // 테두리
        let block = Block::default()
            .title(" Delete ")
            .title_style(
                Style::default()
                    .fg(Color::Rgb(244, 71, 71))
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(244, 71, 71)))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        // 헤더 메시지
        let header = format!(
            "Delete {}? ({})",
            crate::utils::formatter::pluralize(items.len(), "item", "items"),
            total_size
        );
        let header_style = Style::default()
            .fg(self.fg_color)
            .add_modifier(Modifier::BOLD);
        buf.set_string(inner.x, inner.y, &header, header_style);

        // 파일 목록
        let item_style = Style::default().fg(Color::Rgb(86, 156, 214));
        let max_items = (inner.height.saturating_sub(4)) as usize; // 헤더 + 빈줄 + 버튼줄 + 빈줄
        for (i, item) in items.iter().enumerate() {
            if i >= max_items {
                let more = format!("  ... and {} more", items.len() - i);
                buf.set_string(
                    inner.x,
                    inner.y + 2 + i as u16,
                    &more,
                    Style::default().fg(Color::Rgb(128, 128, 128)),
                );
                break;
            }
            let display =
                path_display::truncate_middle(item, inner.width.saturating_sub(4) as usize);
            let line = format!("  · {}", display);
            buf.set_string(inner.x, inner.y + 2 + i as u16, &line, item_style);
        }

        // 버튼 (하단)
        let button_y = area.y + area.height - 2;
        let mut x = inner.x;

        let w1 = self.render_button(buf, x, button_y, "Trash", selected_button == 0);
        x += w1 + 1;
        let w2 = self.render_button(buf, x, button_y, "Delete", selected_button == 1);
        x += w2 + 1;
        self.render_button(buf, x, button_y, "Cancel", selected_button == 2);
    }

    /// 텍스트 필드 렌더링 헬퍼 (cursor_pos는 바이트 인덱스)
    fn render_text_field(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        value: &str,
        cursor_pos: Option<usize>,
    ) {
        // 입력 필드 배경
        for fx in x..x + width {
            if let Some(cell) = buf.cell_mut((fx, y)) {
                cell.set_bg(self.input_bg);
            }
        }

        // 입력값 표시
        let value_style = Style::default().fg(self.fg_color).bg(self.input_bg);
        buf.set_string(x + 1, y, value, value_style);

        // 커서 표시
        if let Some(cpos) = cursor_pos {
            let cursor_col: usize = value[..cpos]
                .chars()
                .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                .sum();
            let cursor_x = x + 1 + cursor_col as u16;
            if cursor_x < x + width - 1 {
                if let Some(cell) = buf.cell_mut((cursor_x, y)) {
                    if cpos < value.len() {
                        cell.set_style(Style::default().fg(self.input_bg).bg(self.fg_color));
                    } else {
                        cell.set_char('▏');
                        cell.set_style(Style::default().fg(self.fg_color).bg(self.input_bg));
                    }
                }
            }
        }
    }

    /// 파일 속성 다이얼로그 렌더링
    #[allow(clippy::too_many_arguments)]
    fn render_properties(
        &self,
        buf: &mut Buffer,
        area: Rect,
        name: &str,
        path: &str,
        file_type: &str,
        size: &str,
        modified: &str,
        permissions: &str,
        children_info: &Option<String>,
    ) {
        // 테두리
        let block = Block::default()
            .title(" Properties ")
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        let label_style = Style::default().fg(Color::Rgb(128, 128, 128));
        let value_style = Style::default().fg(self.fg_color);

        let mut y = inner.y;
        let label_width = 12u16;

        let rows: Vec<(&str, &str)> = vec![
            ("Name:", name),
            ("Path:", path),
            ("Type:", file_type),
            ("Size:", size),
            ("Modified:", modified),
            ("Permissions:", permissions),
        ];

        for (label, value) in &rows {
            buf.set_string(inner.x, y, label, label_style);
            let truncated = path_display::truncate_middle(
                value,
                inner.width.saturating_sub(label_width) as usize,
            );
            buf.set_string(inner.x + label_width, y, &truncated, value_style);
            y += 1;
        }

        if let Some(ref info) = children_info {
            buf.set_string(inner.x, y, "Contents:", label_style);
            buf.set_string(inner.x + label_width, y, info, value_style);
        }

        // OK 버튼
        let button_y = area.y + area.height - 2;
        let button_x = area.x + (area.width - 6) / 2;
        self.render_button(buf, button_x, button_y, "OK", true);
    }

    /// 도움말 다이얼로그 렌더링
    fn render_mount_points(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[(String, std::path::PathBuf)],
        selected_index: usize,
    ) {
        let block = Block::default()
            .title(" Mount Points ")
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(3),
        };

        let normal_style = Style::default().fg(self.fg_color);
        let selected_style = Style::default()
            .fg(self.button_selected_fg)
            .bg(self.button_selected_bg);

        let visible_height = inner.height as usize;
        let scroll = if selected_index >= visible_height {
            selected_index - visible_height + 1
        } else {
            0
        };

        for (i, (name, _path)) in items.iter().skip(scroll).enumerate() {
            if i >= visible_height {
                break;
            }
            let actual_index = scroll + i;
            let style = if actual_index == selected_index {
                selected_style
            } else {
                normal_style
            };

            let y = inner.y + i as u16;
            let display = format!(" {:<width$}", name, width = inner.width as usize - 1);
            let display = if display.len() > inner.width as usize {
                display[..inner.width as usize].to_string()
            } else {
                display
            };
            buf.set_string(inner.x, y, &display, style);
        }

        // 하단 힌트
        let hint = " j/k:Move  Enter:Go  Esc:Close ";
        let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(
            hint_x,
            hint_y,
            hint,
            Style::default().fg(Color::Rgb(100, 100, 100)),
        );
    }

    fn render_tab_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[String],
        selected_index: usize,
    ) {
        let block = Block::default()
            .title(" Tabs ")
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(3),
        };

        let normal_style = Style::default().fg(self.fg_color);
        let selected_style = Style::default()
            .fg(self.button_selected_fg)
            .bg(self.button_selected_bg);

        let visible_height = inner.height as usize;
        let scroll = if selected_index >= visible_height {
            selected_index - visible_height + 1
        } else {
            0
        };

        for (i, name) in items.iter().skip(scroll).enumerate() {
            if i >= visible_height {
                break;
            }
            let actual_index = scroll + i;
            let style = if actual_index == selected_index {
                selected_style
            } else {
                normal_style
            };

            let y = inner.y + i as u16;
            let label = format!(" {}: {}", actual_index + 1, name);
            let display = format!("{:<width$}", label, width = inner.width as usize);
            buf.set_string(inner.x, y, &display, style);
        }

        // 하단 힌트
        let hint = " j/k:Move  Enter:Go  Esc:Close ";
        let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(
            hint_x,
            hint_y,
            hint,
            Style::default().fg(Color::Rgb(100, 100, 100)),
        );
    }

    fn render_history_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[(String, std::path::PathBuf, bool)],
        selected_index: usize,
    ) {
        let block = Block::default()
            .title(" Directory History ")
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(3),
        };

        let normal_style = Style::default().fg(self.fg_color);
        let selected_style = Style::default()
            .fg(self.button_selected_fg)
            .bg(self.button_selected_bg);

        let visible_height = inner.height as usize;
        let scroll = if selected_index >= visible_height {
            selected_index - visible_height + 1
        } else {
            0
        };

        for (i, (display_path, _path, is_current)) in items.iter().skip(scroll).enumerate() {
            if i >= visible_height {
                break;
            }
            let actual_index = scroll + i;
            let style = if actual_index == selected_index {
                selected_style
            } else {
                normal_style
            };

            let y = inner.y + i as u16;
            let prefix = format!(" {}: ", actual_index + 1);
            let marker = if *is_current { " (current)" } else { "" };
            let total_width = inner.width as usize;
            let reserved = UnicodeWidthStr::width(prefix.as_str()) + UnicodeWidthStr::width(marker);
            let path_max_width = total_width.saturating_sub(reserved);
            let path_display = path_display::truncate_path(display_path, path_max_width);
            let label = format!("{}{}{}", prefix, path_display, marker);
            let display = if UnicodeWidthStr::width(label.as_str()) > total_width {
                path_display::truncate_middle(&label, total_width)
            } else {
                format!("{:<width$}", label, width = total_width)
            };
            buf.set_string(inner.x, y, &display, style);
        }

        // 스크롤바 (내용이 화면보다 많을 때만)
        let total_items = items.len();
        if total_items > visible_height && visible_height > 0 {
            let track_height = visible_height;
            let max_scroll = total_items.saturating_sub(visible_height);
            let thumb_height = (track_height * track_height / total_items).max(1);
            let thumb_pos = if max_scroll == 0 {
                0
            } else {
                scroll * (track_height.saturating_sub(thumb_height)) / max_scroll
            };

            let scrollbar_x = area.x + area.width - 2;
            let track_style = Style::default().fg(Color::Rgb(60, 60, 60));
            let thumb_style = Style::default().fg(Color::Rgb(150, 150, 150));

            for i in 0..track_height {
                let sy = inner.y + i as u16;
                let (symbol, style) = if i >= thumb_pos && i < thumb_pos + thumb_height {
                    ("┃", thumb_style)
                } else {
                    ("│", track_style)
                };
                buf.set_string(scrollbar_x, sy, symbol, style);
            }
        }

        let hint = " j/k:Move  Enter:Go  D:Clear  Esc:Close ";
        let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(
            hint_x,
            hint_y,
            hint,
            Style::default().fg(Color::Rgb(100, 100, 100)),
        );
    }

    fn render_bookmark_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[(String, std::path::PathBuf)],
        selected_index: usize,
    ) {
        let block = Block::default()
            .title(" Bookmarks ")
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(3),
        };

        let normal_style = Style::default().fg(self.fg_color);
        let selected_style = Style::default()
            .fg(self.button_selected_fg)
            .bg(self.button_selected_bg);

        let visible_height = inner.height as usize;
        let scroll = if selected_index >= visible_height {
            selected_index - visible_height + 1
        } else {
            0
        };

        for (i, (name, path)) in items.iter().skip(scroll).enumerate() {
            if i >= visible_height {
                break;
            }
            let actual_index = scroll + i;
            let style = if actual_index == selected_index {
                selected_style
            } else {
                normal_style
            };

            let y = inner.y + i as u16;
            let prefix = format!(" {}: ", actual_index + 1);
            let path_text = path.to_string_lossy();
            let content = format!("{}{}", name, if path_text.is_empty() { "" } else { " - " });
            let content_width = UnicodeWidthStr::width(content.as_str());
            let total_width = inner.width as usize;
            let path_width =
                total_width.saturating_sub(UnicodeWidthStr::width(prefix.as_str()) + content_width);
            let truncated_path = path_display::truncate_middle(&path_text, path_width);
            let label = format!("{}{}{}", prefix, content, truncated_path);
            let display = if UnicodeWidthStr::width(label.as_str()) > total_width {
                path_display::truncate_middle(&label, total_width)
            } else {
                path_display::pad_right_to_width(&label, total_width)
            };
            buf.set_string(inner.x, y, &display, style);
        }

        let hint = " j/k:Move  Enter:Go  r:Rename  d:Delete  Esc:Close ";
        let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(
            hint_x,
            hint_y,
            hint,
            Style::default().fg(Color::Rgb(100, 100, 100)),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_archive_preview_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        archive_name: &str,
        items: &[(String, String)],
        selected_index: usize,
        scroll_offset: usize,
        truncated: bool,
    ) {
        let title = format!(" Archive Preview: {} ", archive_name);
        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(3),
        };

        let normal_style = Style::default().fg(self.fg_color);
        let selected_style = Style::default()
            .fg(self.button_selected_fg)
            .bg(self.button_selected_bg);

        let visible_height = inner.height as usize;
        let scroll = scroll_offset.min(items.len().saturating_sub(visible_height));
        let size_col = 12usize.min(inner.width as usize / 3);

        for (row, (path, size_text)) in items.iter().skip(scroll).take(visible_height).enumerate() {
            let actual_index = scroll + row;
            let style = if actual_index == selected_index {
                selected_style
            } else {
                normal_style
            };
            let y = inner.y + row as u16;
            let path_width = inner.width as usize - size_col - 2;
            let path_display = path_display::truncate_middle(path, path_width);
            let left = path_display::pad_right_to_width(&path_display, path_width);
            let line = format!("{}  {:>width$}", left, size_text, width = size_col);
            buf.set_string(inner.x, y, line, style);
        }

        let mut hint = format!(
            " j/k:Move  PgUp/PgDn:Scroll  Home/End  Esc:Close  [{} items] ",
            items.len()
        );
        if truncated {
            hint.push_str("[showing first 5000]");
        }
        let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(
            hint_x,
            hint_y,
            hint,
            Style::default().fg(Color::Rgb(100, 100, 100)),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_archive_create_options(
        &self,
        buf: &mut Buffer,
        area: Rect,
        path_value: &str,
        path_cursor_pos: usize,
        use_password: bool,
        password_value: &str,
        password_cursor_pos: usize,
        password_confirm_value: &str,
        password_confirm_cursor_pos: usize,
        focused_field: usize,
        selected_button: usize,
    ) {
        let block = Block::default()
            .title(" Create Archive ")
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        let label_style = Style::default().fg(self.fg_color);
        let dim_style = Style::default().fg(Color::Rgb(120, 120, 120));
        let focused_label_style = label_style.add_modifier(Modifier::BOLD);

        let path_label_style = if focused_field == 0 {
            focused_label_style
        } else {
            label_style
        };
        buf.set_string(inner.x, inner.y, "Archive path:", path_label_style);
        self.render_text_field(
            buf,
            inner.x,
            inner.y + 1,
            inner.width,
            path_value,
            if focused_field == 0 {
                Some(path_cursor_pos)
            } else {
                None
            },
        );

        let checkbox_style = if focused_field == 1 {
            focused_label_style
        } else {
            label_style
        };
        let checkbox = if use_password { "[x]" } else { "[ ]" };
        let checkbox_line = format!("{} Use password", checkbox);
        buf.set_string(inner.x, inner.y + 3, checkbox_line, checkbox_style);

        let password_label_style = if use_password {
            if focused_field == 2 {
                focused_label_style
            } else {
                label_style
            }
        } else {
            dim_style
        };
        buf.set_string(inner.x, inner.y + 5, "Password:", password_label_style);
        let masked_password = "*".repeat(password_value.chars().count());
        self.render_text_field(
            buf,
            inner.x,
            inner.y + 6,
            inner.width,
            &masked_password,
            if use_password && focused_field == 2 {
                Some(password_cursor_pos)
            } else {
                None
            },
        );

        let confirm_label_style = if use_password {
            if focused_field == 3 {
                focused_label_style
            } else {
                label_style
            }
        } else {
            dim_style
        };
        buf.set_string(
            inner.x,
            inner.y + 8,
            "Confirm password:",
            confirm_label_style,
        );
        let masked_confirm = "*".repeat(password_confirm_value.chars().count());
        self.render_text_field(
            buf,
            inner.x,
            inner.y + 9,
            inner.width,
            &masked_confirm,
            if use_password && focused_field == 3 {
                Some(password_confirm_cursor_pos)
            } else {
                None
            },
        );

        let hint = "Tab/Shift+Tab:Move  Space:Toggle password  Enter:OK  Esc:Cancel  (zip/7z only)";
        let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
        let hint_y = area.y + area.height.saturating_sub(3);
        buf.set_string(
            hint_x,
            hint_y,
            hint,
            Style::default().fg(Color::Rgb(100, 100, 100)),
        );

        let button_y = area.y + area.height.saturating_sub(2);
        let buttons_selected = focused_field == 4;
        let ok_selected = buttons_selected && selected_button == 0;
        let cancel_selected = buttons_selected && selected_button == 1;
        let ok_width = self.render_button(buf, inner.x, button_y, "OK", ok_selected);
        self.render_button(
            buf,
            inner.x + ok_width + 2,
            button_y,
            "Cancel",
            cancel_selected,
        );
    }

    fn render_help(
        &self,
        buf: &mut Buffer,
        area: Rect,
        scroll_offset: usize,
        search_query: &str,
        search_mode: bool,
    ) {
        // 테두리
        let block = Block::default()
            .title(" Keyboard Shortcuts ")
            .title_style(
                Style::default()
                    .fg(self.title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(5), // 검색 줄 + 하단 힌트 공간 확보
        };

        let header_style = Style::default()
            .fg(self.title_color)
            .add_modifier(Modifier::BOLD);
        let key_style = Style::default().fg(Color::Rgb(86, 156, 214));
        let desc_style = Style::default().fg(self.fg_color);
        let match_style = Style::default()
            .fg(Color::Rgb(255, 255, 100))
            .add_modifier(Modifier::UNDERLINED);

        // 도움말 내용 (액션 레지스트리에서 생성)
        let lines = generate_help_entries();
        let query = search_query.trim();

        // 전체 행 리스트 생성
        let mut all_rows: Vec<(bool, String, String)> = Vec::new(); // (is_header, col1, col2)
        for (category, items) in &lines {
            let mut filtered_items = Vec::new();
            for (key, desc) in items {
                if query.is_empty()
                    || contains_case_insensitive(key, query)
                    || contains_case_insensitive(desc, query)
                {
                    filtered_items.push(((*key).to_string(), (*desc).to_string()));
                }
            }
            if !query.is_empty() && filtered_items.is_empty() {
                continue;
            }
            all_rows.push((true, (*category).to_string(), String::new()));
            for (key, desc) in filtered_items {
                all_rows.push((false, key, desc));
            }
            all_rows.push((false, String::new(), String::new())); // 빈 줄
        }
        while all_rows
            .last()
            .is_some_and(|r| !r.0 && r.1.is_empty() && r.2.is_empty())
        {
            all_rows.pop();
        }

        let result_count = all_rows.iter().filter(|r| !r.0 && !r.1.is_empty()).count();
        let result_text = if query.is_empty() {
            format!("Total: {}", result_count)
        } else {
            format!("Results: {}", result_count)
        };
        let result_x = area.x + area.width.saturating_sub(result_text.len() as u16 + 3);

        // 검색 표시줄
        let search_y = area.y + 1;
        let search_label = if search_mode { "Search*:" } else { "Search:" };
        let search_label_style = Style::default().fg(self.border_color);
        let search_style = Style::default().fg(self.fg_color).bg(self.input_bg);
        buf.set_string(inner.x, search_y, search_label, search_label_style);
        let search_field_x = inner.x + 9;
        // 우측 결과 텍스트와 겹치지 않도록 검색 필드 폭 예약
        let search_field_end = result_x.saturating_sub(2).max(search_field_x);
        let search_field_width = search_field_end.saturating_sub(search_field_x);
        for x in search_field_x..search_field_x + search_field_width {
            if let Some(cell) = buf.cell_mut((x, search_y)) {
                cell.set_bg(self.input_bg);
            }
        }
        let search_display = path_display::truncate_middle(query, search_field_width as usize);
        buf.set_string(search_field_x, search_y, search_display, search_style);

        buf.set_string(
            result_x,
            search_y,
            &result_text,
            Style::default().fg(Color::Rgb(128, 128, 128)),
        );

        // 검색 줄 아래부터 본문 시작
        let content_y = inner.y + 1;
        let content_height = inner.height.saturating_sub(1);

        if all_rows.is_empty() {
            let no_result = "No shortcuts match your search";
            let y = content_y;
            buf.set_string(
                inner.x,
                y,
                no_result,
                Style::default().fg(Color::Rgb(128, 128, 128)),
            );
            let hint = "Esc:Clear/Close  /:Search  j/k:Scroll";
            let hint_style = Style::default().fg(Color::Rgb(128, 128, 128));
            let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
            let hint_y = area.y + area.height - 2;
            buf.set_string(hint_x, hint_y, hint, hint_style);
            return;
        }

        // 스크롤 적용
        let visible_height = content_height as usize;
        let max_scroll = all_rows.len().saturating_sub(visible_height);
        let effective_scroll = scroll_offset.min(max_scroll);

        let key_col_width = 16u16;

        for (i, row) in all_rows
            .iter()
            .skip(effective_scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = content_y + i as u16;
            if row.0 {
                // 카테고리 헤더
                buf.set_string(inner.x, y, &row.1, header_style);
            } else if !row.1.is_empty() {
                // 키바인딩
                let key_matches = !query.is_empty() && contains_case_insensitive(&row.1, query);
                let desc_matches = !query.is_empty() && contains_case_insensitive(&row.2, query);
                buf.set_string(
                    inner.x + 2,
                    y,
                    &row.1,
                    if key_matches { match_style } else { key_style },
                );
                buf.set_string(
                    inner.x + key_col_width,
                    y,
                    &row.2,
                    if desc_matches {
                        match_style
                    } else {
                        desc_style
                    },
                );
            }
        }

        // 스크롤바 (내용이 화면보다 많을 때만)
        let total_items = all_rows.len();
        if total_items > visible_height && visible_height > 0 {
            let track_height = visible_height;
            let thumb_height = (track_height * track_height / total_items).max(1);
            let thumb_pos = if max_scroll == 0 {
                0
            } else {
                effective_scroll * (track_height.saturating_sub(thumb_height)) / max_scroll
            };

            let scrollbar_x = area.x + area.width - 2;
            let track_style = Style::default().fg(Color::Rgb(60, 60, 60));
            let thumb_style = Style::default().fg(Color::Rgb(150, 150, 150));

            for i in 0..track_height {
                let sy = content_y + i as u16;
                let (symbol, style) = if i >= thumb_pos && i < thumb_pos + thumb_height {
                    ("┃", thumb_style)
                } else {
                    ("│", track_style)
                };
                buf.set_string(scrollbar_x, sy, symbol, style);
            }
        }

        // 하단 힌트
        let hint = "Esc:Clear/Close  /:Search  j/k:Scroll";
        let hint_style = Style::default().fg(Color::Rgb(128, 128, 128));
        let hint_x = area.x + (area.width.saturating_sub(hint.len() as u16)) / 2;
        let hint_y = area.y + area.height - 2;
        buf.set_string(hint_x, hint_y, hint, hint_style);
    }

    /// 에러/메시지 다이얼로그 렌더링
    fn render_message(
        &self,
        buf: &mut Buffer,
        area: Rect,
        title: &str,
        message: &str,
        is_error: bool,
    ) {
        let title_color = if is_error {
            Color::Rgb(244, 71, 71)
        } else {
            self.title_color
        };
        let border_color = if is_error {
            Color::Rgb(244, 71, 71)
        } else {
            self.border_color
        };

        // 테두리
        let block = Block::default()
            .title(format!(" {} ", title))
            .title_style(
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING + 3),
        };

        // 메시지
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(self.fg_color))
            .wrap(Wrap { trim: true });
        paragraph.render(inner, buf);

        // OK 버튼
        let button_y = area.y + area.height - 2;
        let button_x = area.x + (area.width - 6) / 2;
        self.render_button(buf, button_x, button_y, "OK", true);
    }
}

impl Widget for Dialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let dialog_area = self.calculate_area(area);

        // 배경 클리어
        Clear.render(dialog_area, buf);

        match self.kind {
            DialogKind::Input {
                title,
                prompt,
                value,
                cursor_pos,
                selected_button,
                purpose,
                completion_candidates,
                completion_index,
                mask_input,
                ..
            } => {
                self.render_input(
                    buf,
                    dialog_area,
                    title,
                    prompt,
                    value,
                    *purpose,
                    completion_candidates,
                    *completion_index,
                    *cursor_pos,
                    *selected_button,
                    true,
                    *mask_input,
                );
            }
            DialogKind::ArchiveCreateOptions {
                path_value,
                path_cursor_pos,
                use_password,
                password_value,
                password_cursor_pos,
                password_confirm_value,
                password_confirm_cursor_pos,
                focused_field,
                selected_button,
                ..
            } => {
                self.render_archive_create_options(
                    buf,
                    dialog_area,
                    path_value,
                    *path_cursor_pos,
                    *use_password,
                    password_value,
                    *password_cursor_pos,
                    password_confirm_value,
                    *password_confirm_cursor_pos,
                    *focused_field,
                    *selected_button,
                );
            }
            DialogKind::Confirm {
                title,
                message,
                selected_button,
            } => {
                self.render_confirm(buf, dialog_area, title, message, *selected_button);
            }
            DialogKind::Conflict {
                source_path,
                dest_path,
                selected_option,
            } => {
                self.render_conflict(buf, dialog_area, source_path, dest_path, *selected_option);
            }
            DialogKind::Progress { progress } => {
                self.render_progress(buf, dialog_area, progress);
            }
            DialogKind::Error { title, message } => {
                self.render_message(buf, dialog_area, title, message, true);
            }
            DialogKind::Message { title, message } => {
                self.render_message(buf, dialog_area, title, message, false);
            }
            DialogKind::DeleteConfirm {
                items,
                total_size,
                selected_button,
            } => {
                self.render_delete_confirm(buf, dialog_area, items, total_size, *selected_button);
            }
            DialogKind::MkdirInput {
                value,
                cursor_pos,
                selected_button,
                ..
            } => {
                self.render_input(
                    buf,
                    dialog_area,
                    "New Directory",
                    "Directory name:",
                    value,
                    InputPurpose::OperationDestination,
                    &[],
                    None,
                    *cursor_pos,
                    *selected_button,
                    false,
                    false,
                );
            }
            DialogKind::RenameInput {
                value,
                cursor_pos,
                selected_button,
                ..
            } => {
                self.render_input(
                    buf,
                    dialog_area,
                    "Rename",
                    "New name:",
                    value,
                    InputPurpose::OperationDestination,
                    &[],
                    None,
                    *cursor_pos,
                    *selected_button,
                    false,
                    false,
                );
            }
            DialogKind::BookmarkRenameInput {
                value,
                cursor_pos,
                selected_button,
                ..
            } => {
                self.render_input(
                    buf,
                    dialog_area,
                    "Bookmark Rename",
                    "New bookmark name:",
                    value,
                    InputPurpose::OperationDestination,
                    &[],
                    None,
                    *cursor_pos,
                    *selected_button,
                    false,
                    false,
                );
            }
            DialogKind::FilterInput {
                value,
                cursor_pos,
                selected_button,
            } => {
                self.render_input(
                    buf,
                    dialog_area,
                    "Filter",
                    "Pattern (supports * ?):",
                    value,
                    InputPurpose::OperationDestination,
                    &[],
                    None,
                    *cursor_pos,
                    *selected_button,
                    false,
                    false,
                );
            }
            DialogKind::MountPoints {
                items,
                selected_index,
            } => {
                self.render_mount_points(buf, dialog_area, items, *selected_index);
            }
            DialogKind::TabList {
                items,
                selected_index,
            } => {
                self.render_tab_list(buf, dialog_area, items, *selected_index);
            }
            DialogKind::HistoryList {
                items,
                selected_index,
            } => {
                self.render_history_list(buf, dialog_area, items, *selected_index);
            }
            DialogKind::BookmarkList {
                items,
                selected_index,
            } => {
                self.render_bookmark_list(buf, dialog_area, items, *selected_index);
            }
            DialogKind::ArchivePreviewList {
                archive_name,
                items,
                selected_index,
                scroll_offset,
                truncated,
            } => {
                self.render_archive_preview_list(
                    buf,
                    dialog_area,
                    archive_name,
                    items,
                    *selected_index,
                    *scroll_offset,
                    *truncated,
                );
            }
            DialogKind::Help {
                scroll_offset,
                search_query,
                search_mode,
                ..
            } => {
                self.render_help(buf, dialog_area, *scroll_offset, search_query, *search_mode);
            }
            DialogKind::Properties {
                name,
                path,
                file_type,
                size,
                modified,
                permissions,
                children_info,
            } => {
                self.render_properties(
                    buf,
                    dialog_area,
                    name,
                    path,
                    file_type,
                    size,
                    modified,
                    permissions,
                    children_info,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_input_creation() {
        let dialog = DialogKind::input("Copy", "Copy to:", "/home/user");
        match dialog {
            DialogKind::Input {
                title,
                prompt,
                value,
                cursor_pos,
                selected_button,
                purpose,
                base_path,
                completion_candidates,
                completion_index,
                ..
            } => {
                assert_eq!(title, "Copy");
                assert_eq!(prompt, "Copy to:");
                assert_eq!(value, "/home/user");
                assert_eq!(cursor_pos, 10);
                assert_eq!(selected_button, 0);
                assert_eq!(purpose, InputPurpose::OperationDestination);
                assert_eq!(base_path, PathBuf::from("."));
                assert!(completion_candidates.is_empty());
                assert!(completion_index.is_none());
            }
            _ => panic!("Expected Input dialog"),
        }
    }

    #[test]
    fn test_go_to_path_input_creation() {
        let dialog = DialogKind::go_to_path_input("/tmp", PathBuf::from("/tmp"));
        match dialog {
            DialogKind::Input {
                title,
                prompt,
                value,
                purpose,
                base_path,
                completion_candidates,
                completion_index,
                ..
            } => {
                assert_eq!(title, "Go to Path");
                assert_eq!(prompt, "Path:");
                assert_eq!(value, "/tmp");
                assert_eq!(purpose, InputPurpose::GoToPath);
                assert_eq!(base_path, PathBuf::from("/tmp"));
                assert!(completion_candidates.is_empty());
                assert!(completion_index.is_none());
            }
            _ => panic!("Expected Input dialog"),
        }
    }

    #[test]
    fn test_archive_create_options_input_creation() {
        let dialog = DialogKind::archive_create_options_input("/tmp/a.zip", PathBuf::from("/tmp"));
        match dialog {
            DialogKind::ArchiveCreateOptions {
                path_value,
                path_cursor_pos,
                use_password,
                password_value,
                password_confirm_value,
                focused_field,
                selected_button,
                base_path,
                ..
            } => {
                assert_eq!(path_value, "/tmp/a.zip");
                assert_eq!(path_cursor_pos, "/tmp/a.zip".len());
                assert!(!use_password);
                assert!(password_value.is_empty());
                assert!(password_confirm_value.is_empty());
                assert_eq!(focused_field, 0);
                assert_eq!(selected_button, 0);
                assert_eq!(base_path, PathBuf::from("/tmp"));
            }
            _ => panic!("Expected ArchiveCreateOptions dialog"),
        }
    }

    #[test]
    fn test_input_dialog_responsive_size_clamp() {
        let dialog_kind = DialogKind::go_to_path_input("", PathBuf::from("."));
        let dialog = Dialog::new(&dialog_kind);

        let area_small = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };
        let area_large = Rect {
            x: 0,
            y: 0,
            width: 200,
            height: 60,
        };

        let sized_small = dialog.calculate_area(area_small);
        assert_eq!(sized_small.width, 57);
        assert_eq!(sized_small.height, 12);

        let sized_large = dialog.calculate_area(area_large);
        assert_eq!(sized_large.width, 110);
        assert_eq!(sized_large.height, 12);
    }

    #[test]
    fn test_go_to_path_does_not_render_inline_ghost_text() {
        let mut kind = DialogKind::go_to_path_input("/Users/boksl/", PathBuf::from("/Users/boksl"));
        if let DialogKind::Input {
            completion_candidates,
            completion_index,
            ..
        } = &mut kind
        {
            *completion_candidates = vec!["/Users/boksl/IdeaProjects".to_string()];
            *completion_index = Some(0);
        }

        let area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 24,
        };
        let mut buf = Buffer::empty(area);
        let dialog = Dialog::new(&kind);
        let dialog_area = dialog.calculate_area(area);
        dialog.render(area, &mut buf);

        let input_y = dialog_area.y + DIALOG_V_PADDING + 1;
        let mut input_line = String::new();
        for x in 0..area.width {
            if let Some(cell) = buf.cell((x, input_y)) {
                input_line.push_str(cell.symbol());
            }
        }
        assert!(
            !input_line.contains("IdeaProjects"),
            "inline ghost text should not be rendered in input line"
        );
    }

    #[test]
    fn test_suggestions_title_shows_selected_and_total_count() {
        let mut kind = DialogKind::go_to_path_input("/Users/boksl/", PathBuf::from("/Users/boksl"));
        if let DialogKind::Input {
            completion_candidates,
            completion_index,
            ..
        } = &mut kind
        {
            *completion_candidates = vec![
                "/Users/boksl/IdeaProjects".to_string(),
                "/Users/boksl/Downloads".to_string(),
            ];
            *completion_index = Some(1);
        }

        let area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 24,
        };
        let mut buf = Buffer::empty(area);
        Dialog::new(&kind).render(area, &mut buf);

        let mut found_title = false;
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            if line.contains("Suggestions (2/2)") {
                found_title = true;
                break;
            }
        }
        assert!(
            found_title,
            "suggestions title should show selected/total count"
        );
    }

    #[test]
    fn test_go_to_path_shows_tab_apply_hint() {
        let mut kind = DialogKind::go_to_path_input("/Users/boksl/", PathBuf::from("/Users/boksl"));
        if let DialogKind::Input {
            completion_candidates,
            completion_index,
            ..
        } = &mut kind
        {
            *completion_candidates = vec!["/Users/boksl/IdeaProjects".to_string()];
            *completion_index = Some(0);
        }

        let area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 24,
        };
        let mut buf = Buffer::empty(area);
        Dialog::new(&kind).render(area, &mut buf);

        let mut found_hint = false;
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            if line.contains("Tab:Apply suggestion") {
                found_hint = true;
                break;
            }
        }
        assert!(found_hint, "go to path dialog should show tab apply hint");
    }

    #[test]
    fn test_mkdir_input_hides_suggestions_panel() {
        let kind = DialogKind::mkdir_input(PathBuf::from("."));
        let area = Rect {
            x: 0,
            y: 0,
            width: 90,
            height: 20,
        };
        let mut buf = Buffer::empty(area);
        Dialog::new(&kind).render(area, &mut buf);

        let mut contains_suggestions = false;
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            if line.contains("Suggestions") || line.contains("No suggestions") {
                contains_suggestions = true;
                break;
            }
        }

        assert!(
            !contains_suggestions,
            "mkdir dialog should hide suggestions"
        );
    }

    #[test]
    fn test_help_search_renders_result_count() {
        let kind = DialogKind::Help {
            scroll_offset: 0,
            search_query: "copy".to_string(),
            search_cursor: 4,
            search_mode: true,
        };
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        Dialog::new(&kind).render(area, &mut buf);

        let mut found_results = false;
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            if line.contains("Results:") {
                found_results = true;
                break;
            }
        }

        assert!(
            found_results,
            "help dialog should render search result count"
        );
    }

    #[test]
    fn test_help_search_query_line_visible_with_results() {
        let kind = DialogKind::Help {
            scroll_offset: 0,
            search_query: "v".to_string(),
            search_cursor: 1,
            search_mode: true,
        };
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        Dialog::new(&kind).render(area, &mut buf);

        let mut found_search_line = false;
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            if line.contains("Search*:") && line.contains('v') {
                found_search_line = true;
                break;
            }
        }

        assert!(
            found_search_line,
            "help dialog should keep search query line visible when results exist"
        );
    }

    #[test]
    fn test_dialog_confirm_creation() {
        let dialog = DialogKind::confirm("Confirm", "Are you sure?");
        match dialog {
            DialogKind::Confirm {
                title,
                message,
                selected_button,
            } => {
                assert_eq!(title, "Confirm");
                assert_eq!(message, "Are you sure?");
                assert_eq!(selected_button, 0);
            }
            _ => panic!("Expected Confirm dialog"),
        }
    }

    #[test]
    fn test_dialog_result_conversion() {
        assert_eq!(
            DialogResult::Overwrite.to_conflict_resolution(),
            Some(ConflictResolution::Overwrite)
        );
        assert_eq!(
            DialogResult::Skip.to_conflict_resolution(),
            Some(ConflictResolution::Skip)
        );
        assert_eq!(
            DialogResult::OverwriteAll.to_conflict_resolution(),
            Some(ConflictResolution::OverwriteAll)
        );
    }
}
