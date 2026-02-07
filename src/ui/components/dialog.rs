//! 다이얼로그 시스템 (Phase 3.2)
//!
//! 파일 복사/이동 작업에 필요한 다이얼로그 위젯 정의

#![allow(dead_code)]

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

    /// 다이얼로그 영역 계산 (화면 중앙)
    fn calculate_area(&self, screen: Rect) -> Rect {
        let (width, height) = match self.kind {
            DialogKind::Input { .. } => (50, 7),
            DialogKind::Confirm { .. } => (40, 8),
            DialogKind::Conflict { .. } => (55, 15),
            DialogKind::Progress { .. } => (50, 9),
            DialogKind::Error { message, .. } | DialogKind::Message { message, .. } => {
                let lines = message.lines().count().max(1);
                (50, (6 + lines as u16).min(15))
            }
            DialogKind::DeleteConfirm { items, .. } => {
                let list_lines = items.len().min(10) as u16;
                (45, (7 + list_lines).min(20))
            }
        };

        let width = width.min(screen.width.saturating_sub(4));
        let height = height.min(screen.height.saturating_sub(4));

        let x = screen.x + (screen.width.saturating_sub(width)) / 2;
        let y = screen.y + (screen.height.saturating_sub(height)) / 2;

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
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
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

        // 입력값 표시
        let display_value = if value.len() > input_width as usize - 2 {
            let start = value.len() - (input_width as usize - 2);
            &value[start..]
        } else {
            value
        };
        let value_style = Style::default().fg(self.fg_color).bg(self.input_bg);
        buf.set_string(inner.x + 1, input_y, display_value, value_style);

        // 커서 표시 (반전 스타일 또는 블록 커서)
        let display_cursor_pos = if value.len() > input_width as usize - 2 {
            // 스크롤된 경우 커서 위치 조정
            let start = value.len() - (input_width as usize - 2);
            cursor_pos.saturating_sub(start)
        } else {
            cursor_pos
        };
        let cursor_x = inner.x + 1 + display_cursor_pos as u16;
        if cursor_x < inner.x + input_width - 1 {
            if let Some(cell) = buf.cell_mut((cursor_x, input_y)) {
                // 커서 위치에 문자가 있으면 반전, 없으면 블록 커서 표시
                if display_cursor_pos < display_value.len() {
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
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
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
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
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
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
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

        // Esc 안내
        let hint_style = Style::default().fg(Color::Rgb(128, 128, 128));
        buf.set_string(inner.x, inner.y + 6, "Press Esc to cancel", hint_style);
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
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        };

        // 헤더 메시지
        let header = format!("Delete {} item(s)? ({})", items.len(), total_size);
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

        let w1 = self.render_button(buf, x, button_y, "휴지통", selected_button == 0);
        x += w1 + 1;
        let w2 = self.render_button(buf, x, button_y, "영구 삭제", selected_button == 1);
        x += w2 + 1;
        self.render_button(buf, x, button_y, "취소", selected_button == 2);
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
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(4),
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
                self.render_delete_confirm(
                    buf,
                    dialog_area,
                    items,
                    total_size,
                    *selected_button,
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
