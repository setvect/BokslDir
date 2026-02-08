//! 다이얼로그 시스템 (Phase 3.2)
//!
//! 파일 복사/이동 작업에 필요한 다이얼로그 위젯 정의

#![allow(dead_code)]

use crate::core::actions::generate_help_entries;
use crate::models::operation::{ConflictResolution, OperationProgress};
use crate::ui::Theme;
use crate::utils::formatter::format_file_size;
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
    /// 단축키 도움말 다이얼로그 (Phase 4)
    Help { scroll_offset: usize },
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
        let value: String = initial.into();
        let cursor_pos = value.len();
        DialogKind::Input {
            title: title.into(),
            prompt: prompt.into(),
            value,
            cursor_pos,
            selected_button: 0, // OK 기본 선택
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

    /// 단축키 도움말 다이얼로그
    pub fn help() -> Self {
        DialogKind::Help { scroll_offset: 0 }
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
            DialogKind::Input { .. }
            | DialogKind::MkdirInput { .. }
            | DialogKind::RenameInput { .. } => (50u16.min(sw.saturating_sub(4)).max(30), 7u16),
            DialogKind::Confirm { .. } => (40u16.min(sw.saturating_sub(4)).max(25), 8u16),
            DialogKind::Conflict { .. } => (55u16.min(sw.saturating_sub(4)).max(35), 15u16),
            DialogKind::Progress { .. } => (50u16.min(sw.saturating_sub(4)).max(30), 11u16),
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
        cursor_pos: usize,
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
        let value_display_width = UnicodeWidthStr::width(value);

        // 스크롤 처리: 커서가 보이도록 표시 시작점 결정
        let (display_value, cursor_display_col) = if value_display_width <= max_display {
            // 전체 표시 가능
            let cursor_col: usize = value[..cursor_pos]
                .chars()
                .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                .sum();
            (value, cursor_col)
        } else {
            // 스크롤 필요: 커서 위치를 기준으로 표시 범위 계산
            let cursor_col_from_start: usize = value[..cursor_pos]
                .chars()
                .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                .sum();

            if cursor_col_from_start < max_display {
                // 커서가 앞쪽이면 앞에서부터 표시
                (value, cursor_col_from_start)
            } else {
                // 커서가 화면 밖이면 커서가 오른쪽 끝에 오도록 스크롤
                let mut start_byte = 0;
                let mut width_sum = 0;
                // 뒤에서부터 max_display만큼의 너비를 찾음
                let target_start_width = cursor_col_from_start.saturating_sub(max_display - 1);
                for (i, c) in value.char_indices() {
                    let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                    if width_sum >= target_start_width {
                        start_byte = i;
                        break;
                    }
                    width_sum += cw;
                }
                let display = &value[start_byte..];
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

        // 버튼
        let button_y = inner.y + 3;
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
        let truncated_path = truncate_path_display(dest, inner.width as usize);
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
        let truncated = truncate_middle(&progress.current_file, inner.width as usize);
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

        // 속도 / ETA
        let speed_eta = format!(
            "{}  ETA: {}",
            progress.format_speed(),
            progress.format_eta()
        );
        let speed_style = Style::default().fg(Color::Rgb(100, 180, 100));
        buf.set_string(inner.x, inner.y + 6, &speed_eta, speed_style);

        // Esc 안내
        let hint_style = Style::default().fg(Color::Rgb(128, 128, 128));
        buf.set_string(inner.x, inner.y + 8, "Press Esc to cancel", hint_style);
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
            let display = truncate_middle(item, inner.width.saturating_sub(4) as usize);
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
            let truncated =
                truncate_middle(value, inner.width.saturating_sub(label_width) as usize);
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
    fn render_help(&self, buf: &mut Buffer, area: Rect, scroll_offset: usize) {
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
            height: area.height.saturating_sub(3), // 하단 힌트 공간 확보
        };

        let header_style = Style::default()
            .fg(self.title_color)
            .add_modifier(Modifier::BOLD);
        let key_style = Style::default().fg(Color::Rgb(86, 156, 214));
        let desc_style = Style::default().fg(self.fg_color);

        // 도움말 내용 (액션 레지스트리에서 생성)
        let lines = generate_help_entries();

        // 전체 행 리스트 생성
        let mut all_rows: Vec<(bool, &str, &str)> = Vec::new(); // (is_header, col1, col2)
        for (category, items) in &lines {
            all_rows.push((true, category, ""));
            for (key, desc) in items {
                all_rows.push((false, key, desc));
            }
            all_rows.push((false, "", "")); // 빈 줄
        }

        // 스크롤 적용
        let visible_height = inner.height as usize;
        let max_scroll = all_rows.len().saturating_sub(visible_height);
        let effective_scroll = scroll_offset.min(max_scroll);

        let key_col_width = 16u16;

        for (i, row) in all_rows
            .iter()
            .skip(effective_scroll)
            .take(visible_height)
            .enumerate()
        {
            let y = inner.y + i as u16;
            if row.0 {
                // 카테고리 헤더
                buf.set_string(inner.x, y, row.1, header_style);
            } else if !row.1.is_empty() {
                // 키바인딩
                buf.set_string(inner.x + 2, y, row.1, key_style);
                buf.set_string(inner.x + key_col_width, y, row.2, desc_style);
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
                let sy = inner.y + i as u16;
                let (symbol, style) = if i >= thumb_pos && i < thumb_pos + thumb_height {
                    ("┃", thumb_style)
                } else {
                    ("│", track_style)
                };
                buf.set_string(scrollbar_x, sy, symbol, style);
            }
        }

        // 하단 힌트
        let hint = "Esc/?:Close  j/k:Scroll";
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
            } => {
                self.render_input(
                    buf,
                    dialog_area,
                    title,
                    prompt,
                    value,
                    *cursor_pos,
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
                    *cursor_pos,
                    *selected_button,
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
                    *cursor_pos,
                    *selected_button,
                );
            }
            DialogKind::Help { scroll_offset } => {
                self.render_help(buf, dialog_area, *scroll_offset);
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

/// 경로를 화면 너비에 맞게 축약
fn truncate_path_display(path: &Path, max_width: usize) -> String {
    let path_str = path.to_string_lossy();
    if path_str.len() <= max_width {
        path_str.to_string()
    } else {
        truncate_middle(&path_str, max_width)
    }
}

/// 문자열 중간 생략
fn truncate_middle(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        return s.to_string();
    }
    if max_width < 5 {
        return s.chars().take(max_width).collect();
    }

    let half = (max_width - 3) / 2;
    let start: String = s.chars().take(half).collect();
    let end: String = s.chars().skip(s.len() - half).collect();
    format!("{}...{}", start, end)
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
            } => {
                assert_eq!(title, "Copy");
                assert_eq!(prompt, "Copy to:");
                assert_eq!(value, "/home/user");
                assert_eq!(cursor_pos, 10);
                assert_eq!(selected_button, 0);
            }
            _ => panic!("Expected Input dialog"),
        }
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

    #[test]
    fn test_truncate_middle() {
        assert_eq!(truncate_middle("short", 10), "short");
        assert_eq!(truncate_middle("verylongstring", 10), "ver...ing");
    }
}
