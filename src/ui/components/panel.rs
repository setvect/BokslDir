#![allow(dead_code)]
// Panel component - 파일 패널 컴포넌트
//
// 파일 리스트 표시, 선택 상태, 테두리 렌더링

use crate::models::file_entry::{FileEntry, FileType};
use crate::ui::Theme;
use crate::utils::formatter::{format_date, format_file_size, format_permissions};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// 패널 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelStatus {
    #[default]
    Inactive,
    Active,
}

/// 패널 컴포넌트
pub struct Panel<'a> {
    /// 패널 제목 (경로)
    title: &'a str,
    /// 패널 상태
    status: PanelStatus,
    /// 파일 목록
    entries: &'a [FileEntry],
    /// 선택된 항목 인덱스
    selected_index: usize,
    /// 스크롤 오프셋
    scroll_offset: usize,
    /// ".." (상위 디렉토리) 표시 여부
    show_parent: bool,
    /// 활성 테두리 색상
    active_border_color: Color,
    /// 비활성 테두리 색상
    inactive_border_color: Color,
    /// 패널 배경색
    bg_color: Color,
    /// 파일 일반 색상
    file_normal_color: Color,
    /// 파일 선택 색상
    file_selected_color: Color,
    /// 파일 선택 배경색
    file_selected_bg_color: Color,
    /// 디렉토리 색상
    directory_color: Color,
    /// 실행 파일 색상
    executable_color: Color,
    /// 심볼릭 링크 색상
    symlink_color: Color,
}

impl<'a> Default for Panel<'a> {
    fn default() -> Self {
        Self {
            title: "",
            status: PanelStatus::default(),
            entries: &[],
            selected_index: 0,
            scroll_offset: 0,
            show_parent: false,
            active_border_color: Color::Rgb(0, 120, 212),
            inactive_border_color: Color::Rgb(60, 60, 60),
            bg_color: Color::Rgb(30, 30, 30),
            file_normal_color: Color::Rgb(212, 212, 212),
            file_selected_color: Color::Rgb(255, 255, 255),
            file_selected_bg_color: Color::Rgb(0, 120, 212),
            directory_color: Color::Rgb(86, 156, 214),
            executable_color: Color::Rgb(78, 201, 176),
            symlink_color: Color::Rgb(206, 145, 120),
        }
    }
}

impl<'a> Panel<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// 제목 설정
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// 패널 상태 설정
    pub fn status(mut self, status: PanelStatus) -> Self {
        self.status = status;
        self
    }

    /// 활성 상태로 설정
    pub fn active(mut self) -> Self {
        self.status = PanelStatus::Active;
        self
    }

    /// 비활성 상태로 설정
    pub fn inactive(mut self) -> Self {
        self.status = PanelStatus::Inactive;
        self
    }

    /// 파일 목록 설정
    pub fn entries(mut self, entries: &'a [FileEntry]) -> Self {
        self.entries = entries;
        self
    }

    /// 선택 인덱스 설정
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// 스크롤 오프셋 설정
    pub fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// 상위 디렉토리 표시 여부 설정
    pub fn show_parent(mut self, show: bool) -> Self {
        self.show_parent = show;
        self
    }

    /// 활성 테두리 색상 설정
    pub fn active_border_color(mut self, color: Color) -> Self {
        self.active_border_color = color;
        self
    }

    /// 비활성 테두리 색상 설정
    pub fn inactive_border_color(mut self, color: Color) -> Self {
        self.inactive_border_color = color;
        self
    }

    /// 배경색 설정
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// 테마 적용
    pub fn theme(mut self, theme: &Theme) -> Self {
        self.active_border_color = theme.panel_active_border.to_color();
        self.inactive_border_color = theme.panel_inactive_border.to_color();
        self.bg_color = theme.panel_bg.to_color();
        self.file_normal_color = theme.file_normal.to_color();
        self.file_selected_color = theme.file_selected.to_color();
        self.file_selected_bg_color = theme.file_selected_bg.to_color();
        self.directory_color = theme.directory.to_color();
        self.executable_color = theme.executable.to_color();
        self.symlink_color = theme.symlink.to_color();
        self
    }

    /// 테두리 색상 반환
    fn border_color(&self) -> Color {
        match self.status {
            PanelStatus::Active => self.active_border_color,
            PanelStatus::Inactive => self.inactive_border_color,
        }
    }

    /// 제목 스타일 반환
    fn title_style(&self) -> Style {
        let base = Style::default().fg(self.file_normal_color);
        match self.status {
            PanelStatus::Active => base.add_modifier(Modifier::BOLD),
            PanelStatus::Inactive => base,
        }
    }

    /// 파일 타입에 따른 아이콘 반환
    fn file_icon(&self, file_type: &FileType) -> &str {
        match file_type {
            FileType::Directory => "📁",
            FileType::File => "📄",
            FileType::Executable => "🔧",
            FileType::Symlink => "🔗",
        }
    }

    /// 파일 타입에 따른 색상 반환 (선택되지 않은 상태)
    fn file_color(&self, file_type: &FileType) -> Color {
        match file_type {
            FileType::Directory => self.directory_color,
            FileType::Executable => self.executable_color,
            FileType::Symlink => self.symlink_color,
            FileType::File => self.file_normal_color,
        }
    }

    /// 경로를 최대 너비에 맞게 축약 (홈 디렉토리 ~로 축약 + 중간 생략)
    fn truncate_path(&self, path: &str, max_width: usize) -> String {
        // 1. 홈 디렉토리를 ~로 축약
        let home_dir = std::env::var("HOME").unwrap_or_default();
        let path = if !home_dir.is_empty() && path.starts_with(&home_dir) {
            format!("~{}", &path[home_dir.len()..])
        } else {
            path.to_string()
        };

        let display_width = path.width();
        if display_width <= max_width {
            return path;
        }

        // 2. 중간 생략: 첫 번째 디렉토리 + ... + 마지막 디렉토리들
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() <= 2 {
            // 경로가 짧으면 뒤에서부터 자르기
            return self.truncate_from_start(&path, max_width);
        }

        let ellipsis = "/...";

        // 첫 번째 부분 (~ 또는 루트)
        let first = if path.starts_with('~') {
            "~".to_string()
        } else {
            format!("/{}", parts[0])
        };

        // 뒤에서부터 가능한 만큼 추가
        let first_width = first.width() + ellipsis.width();
        let available_width = max_width.saturating_sub(first_width);

        let mut end_parts: Vec<&str> = Vec::new();
        let mut current_width = 0;

        for part in parts.iter().rev() {
            let part_width = part.width() + 1; // +1 for "/"
            if current_width + part_width > available_width {
                break;
            }
            end_parts.insert(0, part);
            current_width += part_width;
        }

        if end_parts.is_empty() {
            // 마지막 디렉토리도 안 들어가면 그냥 뒤에서 자르기
            return self.truncate_from_start(&path, max_width);
        }

        format!("{}{}/{}", first, ellipsis, end_parts.join("/"))
    }

    /// 경로를 앞에서부터 자르기 (fallback)
    fn truncate_from_start(&self, path: &str, max_width: usize) -> String {
        let ellipsis = "...";
        let ellipsis_width = ellipsis.width();
        let available_width = max_width.saturating_sub(ellipsis_width);

        let mut result = String::new();
        let mut current_width = 0;

        for ch in path.chars().rev() {
            let ch_width = ch.width().unwrap_or(1);
            if current_width + ch_width > available_width {
                break;
            }
            result.insert(0, ch);
            current_width += ch_width;
        }

        format!("{}{}", ellipsis, result)
    }
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 빈 영역은 렌더링하지 않음
        if area.width == 0 || area.height == 0 {
            return;
        }

        // 제목(경로) 최대 너비 계산 (테두리 2 + 양쪽 공백 2 = 4)
        let title_max_width = (area.width as usize).saturating_sub(4);
        let display_title = self.truncate_path(self.title, title_max_width);

        // 블록 생성 및 렌더링
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color()))
            .title(Span::styled(
                format!(" {} ", display_title),
                self.title_style(),
            ))
            .style(Style::default().bg(self.bg_color));

        let inner = block.inner(area);
        block.render(area, buf);

        // 내부 영역이 너무 작으면 렌더링하지 않음
        if inner.height < 3 {
            return;
        }

        // 패널 크기에 따른 컬럼 설정
        // 듀얼 패널 모드에서는 각 패널이 전체 터미널의 절반 정도이므로
        // 패널 기준으로 더 낮은 threshold 사용
        let width = inner.width as usize;
        let (show_permissions, show_size, date_format) = match width {
            w if w >= 60 => (true, true, "long"),   // "2026-01-30"
            w if w >= 40 => (false, true, "short"), // "01-30"
            _ => (false, false, "short"),           // "01-30"
        };

        // 컬럼 너비 계산
        let perm_width = if show_permissions { 12 } else { 0 };
        let date_width = if date_format == "long" { 12 } else { 6 };
        let size_width = if show_size { 10 } else { 0 };
        let margins = 6; // 좌우 여백 + 구분 공백
        let name_width = width
            .saturating_sub(perm_width)
            .saturating_sub(size_width)
            .saturating_sub(date_width)
            .saturating_sub(margins);

        let mut y = 0;

        // 헤더 렌더링
        let mut header_spans = vec![Span::raw(" ")];
        header_spans.push(Span::styled(
            format!("{:<width$}", "Name", width = name_width),
            Style::default()
                .fg(Color::Rgb(150, 150, 150))
                .add_modifier(Modifier::BOLD),
        ));

        if show_size {
            header_spans.push(Span::raw(" "));
            header_spans.push(Span::styled(
                format!("{:<10}", "Size"),
                Style::default()
                    .fg(Color::Rgb(150, 150, 150))
                    .add_modifier(Modifier::BOLD),
            ));
        }

        header_spans.push(Span::raw(" "));
        header_spans.push(Span::styled(
            format!(
                "{:<width$}",
                "Modified",
                width = if date_format == "long" { 12 } else { 8 }
            ),
            Style::default()
                .fg(Color::Rgb(150, 150, 150))
                .add_modifier(Modifier::BOLD),
        ));

        if show_permissions {
            header_spans.push(Span::raw(" "));
            header_spans.push(Span::styled(
                format!("{:<11}", "Permissions"),
                Style::default()
                    .fg(Color::Rgb(150, 150, 150))
                    .add_modifier(Modifier::BOLD),
            ));
        }

        let header_line = Line::from(header_spans);
        buf.set_line(inner.x, inner.y + y, &header_line, inner.width);
        y += 1;

        // 구분선
        let separator = "─".repeat(inner.width as usize);
        buf.set_string(
            inner.x,
            inner.y + y,
            separator,
            Style::default().fg(Color::Rgb(60, 60, 60)),
        );
        y += 1;

        // ".." (상위 디렉토리) 항목
        if self.show_parent {
            let is_selected = self.selected_index == 0;
            let style = if is_selected {
                Style::default()
                    .bg(self.file_selected_bg_color)
                    .fg(self.file_selected_color)
            } else {
                Style::default().fg(Color::Rgb(150, 150, 150))
            };

            let mut parent_spans = vec![Span::raw(" ")];
            parent_spans.push(Span::styled("[..]", style));
            parent_spans.push(Span::styled(
                " <UP>",
                Style::default().fg(Color::Rgb(100, 100, 100)),
            ));

            let parent_line = Line::from(parent_spans);
            buf.set_line(inner.x, inner.y + y, &parent_line, inner.width);
            y += 1;
        }

        // 가용 높이 계산
        let available_height = (inner.height as usize).saturating_sub(y as usize);

        // 파일 리스트 렌더링
        let start = self.scroll_offset;
        let end = (start + available_height).min(self.entries.len());

        for (i, entry) in self.entries[start..end].iter().enumerate() {
            let entry_index = start + i;
            // show_parent가 true면 ".."이 index 0을 차지하므로
            // entries는 index 1부터 시작
            let is_selected = if self.show_parent {
                entry_index + 1 == self.selected_index
            } else {
                entry_index == self.selected_index
            };

            // 색상 및 배경 결정
            let (fg, bg) = if is_selected {
                (self.file_selected_color, Some(self.file_selected_bg_color))
            } else {
                (self.file_color(&entry.file_type), None)
            };

            let style = if let Some(bg_color) = bg {
                Style::default().fg(fg).bg(bg_color)
            } else {
                Style::default().fg(fg)
            };

            // 파일 라인 구성
            let mut line_spans = vec![Span::styled(" ", style)];

            // 아이콘 + 파일명
            let icon = self.file_icon(&entry.file_type);
            let display_name = self.truncate_name(&entry.name, name_width.saturating_sub(3)); // 아이콘 너비 고려
            let name_str = format!("{} {}", icon, display_name);
            let name_display_width = name_str.width();
            let name_padding = name_width.saturating_sub(name_display_width);

            line_spans.push(Span::styled(name_str, style));
            line_spans.push(Span::styled(" ".repeat(name_padding), style));

            // 크기
            if show_size {
                line_spans.push(Span::styled(" ", style));
                let size_str = if entry.is_directory() {
                    "-".to_string()
                } else {
                    format_file_size(entry.size)
                };
                line_spans.push(Span::styled(format!("{:>9}", size_str), style));
            }

            // 날짜
            line_spans.push(Span::styled(" ", style));
            let date_str = if date_format == "long" {
                format_date(entry.modified)
            } else {
                // 짧은 형식: "MM-DD"
                let full_date = format_date(entry.modified);
                if full_date.contains(':') {
                    full_date // 오늘이면 시간 표시
                } else {
                    // "2026-01-30" -> "01-30"
                    full_date.split('-').skip(1).collect::<Vec<_>>().join("-")
                }
            };
            line_spans.push(Span::styled(
                format!("{:<width$}", date_str, width = date_width),
                style,
            ));

            // 권한
            if show_permissions {
                line_spans.push(Span::styled(" ", style));
                let perm_str = format_permissions(entry.permissions.as_ref());
                line_spans.push(Span::styled(format!("{:<11}", perm_str), style));
            }

            let file_line = Line::from(line_spans);
            buf.set_line(inner.x, inner.y + y, &file_line, inner.width);
            y += 1;

            // 가용 높이 초과 시 중단
            if y >= inner.height {
                break;
            }
        }

        // 빈 패널 상태 표시 (파일이 없고 ".."도 없을 때)
        if self.entries.is_empty() && !self.show_parent && y < inner.height {
            let empty_text = Line::from(vec![Span::styled(
                " <empty>",
                Style::default().fg(Color::Rgb(100, 100, 100)),
            )]);
            buf.set_line(inner.x, inner.y + y, &empty_text, inner.width);
        }
    }
}

impl Panel<'_> {
    /// 파일명을 최대 너비로 잘라냄
    fn truncate_name(&self, name: &str, max_width: usize) -> String {
        let display_width = name.width();
        if display_width <= max_width {
            return name.to_string();
        }

        // "..." 포함하여 잘라내기
        let mut truncated = String::new();
        let mut current_width = 0;
        for ch in name.chars() {
            let ch_width = ch.width().unwrap_or(1);
            if current_width + ch_width + 3 > max_width {
                truncated.push_str("...");
                break;
            }
            truncated.push(ch);
            current_width += ch_width;
        }
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let entries = vec![];
        let panel = Panel::new()
            .title("/home/user")
            .active()
            .entries(&entries)
            .selected_index(0)
            .scroll_offset(0)
            .show_parent(false);

        assert_eq!(panel.title, "/home/user");
        assert_eq!(panel.status, PanelStatus::Active);
        assert_eq!(panel.entries.len(), 0);
        assert_eq!(panel.selected_index, 0);
    }

    #[test]
    fn test_panel_status_toggle() {
        let active_panel = Panel::new().active();
        assert_eq!(active_panel.status, PanelStatus::Active);

        let inactive_panel = Panel::new().inactive();
        assert_eq!(inactive_panel.status, PanelStatus::Inactive);
    }

    #[test]
    fn test_truncate_name() {
        let panel = Panel::new();

        // 짧은 이름은 그대로 유지
        assert_eq!(panel.truncate_name("test.txt", 20), "test.txt");

        // 긴 이름은 잘림
        let long_name = "very_long_filename_that_should_be_truncated.txt";
        let truncated = panel.truncate_name(long_name, 20);
        assert!(truncated.ends_with("..."));
        assert!(truncated.len() <= 23); // 20 + "..."
    }
}
