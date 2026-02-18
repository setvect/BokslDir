use super::{DialogKind, InputPurpose};
use crate::core::actions::generate_help_entries;
use crate::models::operation::OperationProgress;
use crate::ui::{localize_runtime_text, I18n, Language, MessageKey, TextKey, Theme};
use crate::utils::formatter::format_file_size;
use crate::utils::path_display;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Widget, Wrap},
};
use std::path::Path;
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
    warning_color: Color,
    error_color: Color,
    success_color: Color,
    muted_color: Color,
    language: Language,
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
            warning_color: Color::Rgb(255, 165, 0),
            error_color: Color::Rgb(244, 71, 71),
            success_color: Color::Rgb(100, 180, 100),
            muted_color: Color::Rgb(128, 128, 128),
            language: Language::English,
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
        self.warning_color = theme.warning.to_color();
        self.error_color = theme.error.to_color();
        self.success_color = theme.success.to_color();
        self.muted_color = theme.panel_inactive_border.to_color();
        self
    }

    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    fn i18n(&self) -> I18n {
        I18n::new(self.language)
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
        let (title_text, prompt_text) = match purpose {
            InputPurpose::GoToPath => (
                self.i18n().tr(TextKey::DialogTitleGoToPath).to_string(),
                self.i18n().tr(TextKey::DialogPromptPath).to_string(),
            ),
            InputPurpose::ArchiveCreatePath => (
                self.i18n()
                    .tr(TextKey::DialogTitleCreateArchive)
                    .to_string(),
                self.i18n().tr(TextKey::DialogArchivePath).to_string(),
            ),
            InputPurpose::ArchiveExtractDestination => (
                self.i18n()
                    .tr(TextKey::DialogTitleExtractArchive)
                    .to_string(),
                self.i18n().tr(TextKey::DialogPromptExtractTo).to_string(),
            ),
            InputPurpose::ArchivePassword => (
                self.i18n()
                    .tr(TextKey::DialogTitleArchivePassword)
                    .to_string(),
                self.i18n()
                    .tr(TextKey::DialogPromptArchivePassword)
                    .to_string(),
            ),
            InputPurpose::OperationDestination => (
                localize_runtime_text(self.language, title),
                localize_runtime_text(self.language, prompt),
            ),
        };

        // 테두리
        let block = Block::default()
            .title(format!(" {} ", title_text))
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
        buf.set_string(inner.x, inner.y, prompt_text, prompt_style);

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
                let title = format!(
                    "{} ({}/{})",
                    self.i18n().tr(TextKey::DialogSuggestions),
                    selected_display,
                    total_candidates
                );
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
                let hint = self.i18n().tr(TextKey::DialogSuggestionHint);
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
        let ok_width = self.render_button(
            buf,
            inner.x,
            button_y,
            self.i18n().tr(TextKey::Ok),
            selected_button == 0,
        );
        self.render_button(
            buf,
            inner.x + ok_width + 2,
            button_y,
            self.i18n().tr(TextKey::Cancel),
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
        let localized_title = localize_runtime_text(self.language, title);
        let localized_message = localize_runtime_text(self.language, message);

        // 테두리
        let block = Block::default()
            .title(format!(" {} ", localized_title))
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
        let paragraph = Paragraph::new(localized_message)
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
        let ok_label = self.i18n().tr(TextKey::Ok);
        let cancel_label = self.i18n().tr(TextKey::Cancel);
        let buttons_width =
            (format!(" {} ", ok_label).width() + 2 + format!(" {} ", cancel_label).width()) as u16;
        let button_x = area.x + (area.width.saturating_sub(buttons_width)) / 2;

        let ok_width = self.render_button(buf, button_x, button_y, ok_label, selected_button == 0);
        self.render_button(
            buf,
            button_x + ok_width + 2,
            button_y,
            cancel_label,
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
            .title(self.i18n().tr(TextKey::DialogTitleFileExists))
            .title_style(
                Style::default()
                    .fg(self.warning_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.warning_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        let msg_style = Style::default().fg(self.fg_color);
        let path_style = Style::default().fg(self.title_color);
        let label_style = Style::default().fg(self.muted_color);

        // 소스 파일 표시
        buf.set_string(
            inner.x,
            inner.y,
            self.i18n().tr(TextKey::DialogSource),
            label_style,
        );
        let source_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(self.i18n().tr(TextKey::DialogUnknown));
        buf.set_string(inner.x + 8, inner.y, source_name, path_style);

        // 대상 경로 표시
        buf.set_string(
            inner.x,
            inner.y + 2,
            self.i18n().tr(TextKey::DialogTargetExists),
            msg_style,
        );
        let truncated_path = path_display::truncate_path_buf(dest, inner.width as usize);
        buf.set_string(inner.x, inner.y + 3, &truncated_path, path_style);

        // 옵션 버튼들 (2줄로 배치)
        // 첫 번째 줄: Overwrite, Skip
        let row1_options = [
            self.i18n().tr(TextKey::DialogOverwrite),
            self.i18n().tr(TextKey::DialogSkip),
        ];
        let button_y1 = inner.y + 6;
        let mut x = inner.x;

        for (i, option) in row1_options.iter().enumerate() {
            let width = self.render_button(buf, x, button_y1, option, selected_option == i);
            x += width + 1;
        }

        // 두 번째 줄: Overwrite All, Skip All, Cancel
        let row2_options = [
            self.i18n().tr(TextKey::DialogOverwriteAll),
            self.i18n().tr(TextKey::DialogSkipAll),
            self.i18n().tr(TextKey::Cancel),
        ];
        let button_y2 = inner.y + 8;
        x = inner.x;

        for (i, option) in row2_options.iter().enumerate() {
            let width = self.render_button(buf, x, button_y2, option, selected_option == i + 2);
            x += width + 1;
        }
    }

    /// 진행률 다이얼로그 렌더링
    fn render_progress(&self, buf: &mut Buffer, area: Rect, progress: &OperationProgress) {
        let operation_name = localize_runtime_text(self.language, progress.operation_type.name());
        let title = format!(" {} ", operation_name);

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
        let count_text = self.i18n().fmt(
            MessageKey::ProgressFilesCount,
            &[
                ("completed", progress.files_completed.to_string()),
                ("total", progress.total_files.to_string()),
            ],
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
        let processed_text = self.i18n().fmt(
            MessageKey::ProgressProcessed,
            &[
                ("processed", progress.items_processed.to_string()),
                ("remaining", remaining.to_string()),
                ("failed", progress.items_failed.to_string()),
            ],
        );
        buf.set_string(inner.x, inner.y + 6, &processed_text, file_style);

        // 속도 / ETA
        let speed_eta = format!(
            "{}  {}: {}",
            progress.format_speed(),
            self.i18n().tr(TextKey::DialogEta),
            progress.format_eta()
        );
        let speed_style = Style::default().fg(self.success_color);
        buf.set_string(inner.x, inner.y + 7, &speed_eta, speed_style);

        // Esc 안내
        let hint_style = Style::default().fg(self.muted_color);
        buf.set_string(
            inner.x,
            inner.y + 9,
            self.i18n().tr(TextKey::DialogPressEscToCancel),
            hint_style,
        );
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
            .title(self.i18n().tr(TextKey::DialogTitleDelete))
            .title_style(
                Style::default()
                    .fg(self.error_color)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.error_color))
            .style(Style::default().bg(self.bg_color));
        block.render(area, buf);

        let inner = Rect {
            x: area.x + DIALOG_H_PADDING,
            y: area.y + DIALOG_V_PADDING,
            width: area.width.saturating_sub(DIALOG_H_PADDING * 2),
            height: area.height.saturating_sub(DIALOG_V_PADDING * 2),
        };

        // 헤더 메시지
        let header = self.i18n().fmt(
            MessageKey::DeleteHeader,
            &[
                ("count", items.len().to_string()),
                ("total_size", total_size.to_string()),
            ],
        );
        let header_style = Style::default()
            .fg(self.fg_color)
            .add_modifier(Modifier::BOLD);
        buf.set_string(inner.x, inner.y, &header, header_style);

        // 파일 목록
        let item_style = Style::default().fg(self.title_color);
        let max_items = (inner.height.saturating_sub(4)) as usize; // 헤더 + 빈줄 + 버튼줄 + 빈줄
        for (i, item) in items.iter().enumerate() {
            if i >= max_items {
                let more = self.i18n().fmt(
                    MessageKey::DeleteMore,
                    &[("count", (items.len() - i).to_string())],
                );
                buf.set_string(
                    inner.x,
                    inner.y + 2 + i as u16,
                    &more,
                    Style::default().fg(self.muted_color),
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

        let w1 = self.render_button(
            buf,
            x,
            button_y,
            self.i18n().tr(TextKey::DialogTrash),
            selected_button == 0,
        );
        x += w1 + 1;
        let w2 = self.render_button(
            buf,
            x,
            button_y,
            self.i18n().tr(TextKey::DialogDelete),
            selected_button == 1,
        );
        x += w2 + 1;
        self.render_button(
            buf,
            x,
            button_y,
            self.i18n().tr(TextKey::Cancel),
            selected_button == 2,
        );
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
        let localized_file_type = localize_runtime_text(self.language, file_type);
        let localized_size = localize_runtime_text(self.language, size);
        let localized_children_info = children_info
            .as_ref()
            .map(|info| localize_runtime_text(self.language, info));

        // 테두리
        let block = Block::default()
            .title(self.i18n().tr(TextKey::DialogTitleProperties))
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

        let label_style = Style::default().fg(self.muted_color);
        let value_style = Style::default().fg(self.fg_color);

        let mut y = inner.y;
        let label_width = 12u16;

        let rows: Vec<(&str, &str)> = vec![
            (self.i18n().tr(TextKey::DialogName), name),
            (self.i18n().tr(TextKey::DialogPath), path),
            (self.i18n().tr(TextKey::DialogType), &localized_file_type),
            (self.i18n().tr(TextKey::DialogSize), &localized_size),
            (self.i18n().tr(TextKey::DialogModified), modified),
            (self.i18n().tr(TextKey::DialogPermissions), permissions),
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

        if let Some(ref info) = localized_children_info {
            buf.set_string(
                inner.x,
                y,
                self.i18n().tr(TextKey::DialogContents),
                label_style,
            );
            buf.set_string(inner.x + label_width, y, info, value_style);
        }

        // OK 버튼
        let button_y = area.y + area.height - 2;
        let ok_label = self.i18n().tr(TextKey::Ok);
        let button_width = format!(" {} ", ok_label).width() as u16;
        let button_x = area.x + (area.width.saturating_sub(button_width)) / 2;
        self.render_button(buf, button_x, button_y, ok_label, true);
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
            .title(self.i18n().tr(TextKey::DialogTitleMountPoints))
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
        let hint = self.i18n().tr(TextKey::DialogHintMoveGoClose);
        let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(hint_x, hint_y, hint, Style::default().fg(self.muted_color));
    }

    fn render_tab_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[String],
        selected_index: usize,
    ) {
        let block = Block::default()
            .title(self.i18n().tr(TextKey::DialogTitleTabs))
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
        let hint = self.i18n().tr(TextKey::DialogHintMoveGoClose);
        let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(hint_x, hint_y, hint, Style::default().fg(self.muted_color));
    }

    fn render_history_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[(String, std::path::PathBuf, bool)],
        selected_index: usize,
    ) {
        let block = Block::default()
            .title(self.i18n().tr(TextKey::DialogTitleHistory))
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
            let marker = if *is_current {
                self.i18n().tr(TextKey::DialogHistoryCurrentMarker)
            } else {
                ""
            };
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
            let track_style = Style::default().fg(self.progress_unfilled);
            let thumb_style = Style::default().fg(self.muted_color);

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

        let hint = self.i18n().tr(TextKey::DialogHintMoveGoClearClose);
        let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(hint_x, hint_y, hint, Style::default().fg(self.muted_color));
    }

    fn render_bookmark_list(
        &self,
        buf: &mut Buffer,
        area: Rect,
        items: &[(String, std::path::PathBuf)],
        selected_index: usize,
    ) {
        let block = Block::default()
            .title(self.i18n().tr(TextKey::DialogTitleBookmarks))
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

        let hint = self.i18n().tr(TextKey::DialogHintMoveGoRenameDeleteClose);
        let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(hint_x, hint_y, hint, Style::default().fg(self.muted_color));
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
        let title = self.i18n().fmt(
            MessageKey::DialogArchivePreviewTitle,
            &[("name", archive_name.to_string())],
        );
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

        let mut hint = self.i18n().fmt(
            MessageKey::DialogArchivePreviewHint,
            &[("count", items.len().to_string())],
        );
        if truncated {
            hint.push_str(self.i18n().tr(TextKey::DialogArchivePreviewTruncated));
        }
        let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
        let hint_y = area.y + area.height - 1;
        buf.set_string(hint_x, hint_y, hint, Style::default().fg(self.muted_color));
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
            .title(self.i18n().tr(TextKey::DialogTitleCreateArchive))
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
        let dim_style = Style::default().fg(self.muted_color);
        let focused_label_style = label_style.add_modifier(Modifier::BOLD);

        let path_label_style = if focused_field == 0 {
            focused_label_style
        } else {
            label_style
        };
        buf.set_string(
            inner.x,
            inner.y,
            self.i18n().tr(TextKey::DialogArchivePath),
            path_label_style,
        );
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
        let checkbox_line = format!(
            "{} {}",
            checkbox,
            self.i18n().tr(TextKey::DialogUsePassword)
        );
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
        buf.set_string(
            inner.x,
            inner.y + 5,
            self.i18n().tr(TextKey::DialogPassword),
            password_label_style,
        );
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
            self.i18n().tr(TextKey::DialogConfirmPassword),
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

        let hint = self.i18n().tr(TextKey::DialogHintArchiveCreate);
        let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
        let hint_y = area.y + area.height.saturating_sub(3);
        buf.set_string(hint_x, hint_y, hint, Style::default().fg(self.muted_color));

        let button_y = area.y + area.height.saturating_sub(2);
        let buttons_selected = focused_field == 4;
        let ok_selected = buttons_selected && selected_button == 0;
        let cancel_selected = buttons_selected && selected_button == 1;
        let ok_width = self.render_button(
            buf,
            inner.x,
            button_y,
            self.i18n().tr(TextKey::Ok),
            ok_selected,
        );
        self.render_button(
            buf,
            inner.x + ok_width + 2,
            button_y,
            self.i18n().tr(TextKey::Cancel),
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
            .title(self.i18n().tr(TextKey::DialogKeyboardShortcutsTitle))
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
        let key_style = Style::default().fg(self.title_color);
        let desc_style = Style::default().fg(self.fg_color);
        let match_style = Style::default()
            .fg(self.warning_color)
            .add_modifier(Modifier::UNDERLINED);

        // 도움말 내용 (액션 레지스트리에서 생성)
        let lines = generate_help_entries(self.language);
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
            self.i18n().fmt(
                MessageKey::HelpTotal,
                &[("count", result_count.to_string())],
            )
        } else {
            self.i18n().fmt(
                MessageKey::HelpResults,
                &[("count", result_count.to_string())],
            )
        };
        let result_x = area.x + area.width.saturating_sub(result_text.width() as u16 + 3);

        // 검색 표시줄
        let search_y = area.y + 1;
        let search_label = if search_mode {
            self.i18n().tr(TextKey::DialogSearchActive)
        } else {
            self.i18n().tr(TextKey::DialogSearch)
        };
        let search_label_style = Style::default().fg(self.border_color);
        let search_style = Style::default().fg(self.fg_color).bg(self.input_bg);
        buf.set_string(inner.x, search_y, search_label, search_label_style);
        let search_field_x = inner.x + search_label.width() as u16 + 1;
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
            Style::default().fg(self.muted_color),
        );

        // 검색 줄 아래부터 본문 시작
        let content_y = inner.y + 1;
        let content_height = inner.height.saturating_sub(1);

        if all_rows.is_empty() {
            let no_result = self.i18n().tr(TextKey::DialogNoShortcutMatches);
            let y = content_y;
            buf.set_string(inner.x, y, no_result, Style::default().fg(self.muted_color));
            let hint = self.i18n().tr(TextKey::DialogHelpHint);
            let hint_style = Style::default().fg(self.muted_color);
            let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
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
            let track_style = Style::default().fg(self.progress_unfilled);
            let thumb_style = Style::default().fg(self.muted_color);

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
        let hint = self.i18n().tr(TextKey::DialogHelpHint);
        let hint_style = Style::default().fg(self.muted_color);
        let hint_x = area.x + (area.width.saturating_sub(hint.width() as u16)) / 2;
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
        let localized_title = localize_runtime_text(self.language, title);
        let localized_message = localize_runtime_text(self.language, message);

        let title_color = if is_error {
            self.error_color
        } else {
            self.title_color
        };
        let border_color = if is_error {
            self.error_color
        } else {
            self.border_color
        };

        // 테두리
        let block = Block::default()
            .title(format!(" {} ", localized_title))
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
        let paragraph = Paragraph::new(localized_message)
            .style(Style::default().fg(self.fg_color))
            .wrap(Wrap { trim: true });
        paragraph.render(inner, buf);

        // OK 버튼
        let button_y = area.y + area.height - 2;
        let ok_label = self.i18n().tr(TextKey::Ok);
        let button_width = format!(" {} ", ok_label).width() as u16;
        let button_x = area.x + (area.width.saturating_sub(button_width)) / 2;
        self.render_button(buf, button_x, button_y, ok_label, true);
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
                    self.i18n().tr(TextKey::DialogNewDirectory),
                    self.i18n().tr(TextKey::DialogDirectoryName),
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
                    self.i18n().tr(TextKey::DialogRename),
                    self.i18n().tr(TextKey::DialogNewName),
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
                    self.i18n().tr(TextKey::DialogBookmarkRename),
                    self.i18n().tr(TextKey::DialogNewBookmarkName),
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
                    self.i18n().tr(TextKey::DialogFilter),
                    self.i18n().tr(TextKey::DialogFilterPattern),
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
    use super::super::kind::DialogResult;
    use super::*;
    use crate::models::operation::ConflictResolution;
    use std::path::PathBuf;

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
    fn test_archive_create_options_localized_in_korean() {
        let dialog = DialogKind::archive_create_options_input("/tmp/a.zip", PathBuf::from("/tmp"));
        let area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 30,
        };
        let mut buf = Buffer::empty(area);
        Dialog::new(&dialog)
            .language(Language::Korean)
            .render(area, &mut buf);

        let mut rendered = String::new();
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            rendered.push_str(&line);
            rendered.push('\n');
        }

        let normalized: String = rendered.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(normalized.contains("압축경로:"), "rendered=\n{}", rendered);
        assert!(normalized.contains("비밀번호"), "rendered=\n{}", rendered);
        assert!(
            !rendered.contains("Archive path:"),
            "rendered=\n{}",
            rendered
        );
        assert!(
            !rendered.contains("Use password"),
            "rendered=\n{}",
            rendered
        );
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
