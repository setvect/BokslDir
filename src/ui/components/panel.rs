#![allow(dead_code)]
// Panel component - 파일 패널 컴포넌트
//
// 파일 리스트 표시, 선택 상태, 테두리 렌더링

use crate::app::SizeFormat;
use crate::models::file_entry::{FileEntry, FileType};
use crate::models::panel_state::{SortBy, SortOrder};
use crate::ui::Theme;
use crate::utils::formatter::{
    format_date, format_file_size, format_file_size_bytes, format_permissions,
};
use crate::utils::glob;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use std::collections::HashSet;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// 아이콘 표시 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconMode {
    /// 이모지 아이콘 (기본)
    #[default]
    Emoji,
    /// ASCII 텍스트 아이콘 (터미널 호환)
    Ascii,
}

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
    /// 커서 위치 (selected_index, ".." 포함)
    selected_index: usize,
    /// 스크롤 오프셋
    scroll_offset: usize,
    /// ".." (상위 디렉토리) 표시 여부
    show_parent: bool,
    /// 다중 선택된 항목 (entries 인덱스 기반)
    selected_items: &'a HashSet<usize>,
    /// 활성 테두리 색상
    active_border_color: Color,
    /// 비활성 테두리 색상
    inactive_border_color: Color,
    /// 패널 배경색
    bg_color: Color,
    /// 파일 일반 색상
    file_normal_color: Color,
    /// 파일 선택(커서) 색상
    file_selected_color: Color,
    /// 파일 선택(커서) 배경색
    file_selected_bg_color: Color,
    /// 다중 선택(마킹) 색상
    file_marked_color: Color,
    /// 다중 선택 마커 색상
    file_marked_symbol_color: Color,
    /// 디렉토리 색상
    directory_color: Color,
    /// 실행 파일 색상
    executable_color: Color,
    /// 심볼릭 링크 색상
    symlink_color: Color,
    /// 아이콘 모드
    icon_mode: IconMode,
    /// 현재 정렬 기준
    sort_by: SortBy,
    /// 현재 정렬 순서
    sort_order: SortOrder,
    /// 필터 패턴 (하이라이트용)
    filter_pattern: Option<&'a str>,
    /// 파일 크기 표시 형식
    size_format: SizeFormat,
}

/// 빈 HashSet을 위한 정적 참조
static EMPTY_SELECTION: std::sync::LazyLock<HashSet<usize>> =
    std::sync::LazyLock::new(HashSet::new);

impl<'a> Default for Panel<'a> {
    fn default() -> Self {
        Self {
            title: "",
            status: PanelStatus::default(),
            entries: &[],
            selected_index: 0,
            scroll_offset: 0,
            show_parent: false,
            selected_items: &EMPTY_SELECTION,
            active_border_color: Color::Rgb(0, 120, 212),
            inactive_border_color: Color::Rgb(60, 60, 60),
            bg_color: Color::Rgb(30, 30, 30),
            file_normal_color: Color::Rgb(212, 212, 212),
            file_selected_color: Color::Rgb(255, 255, 255),
            file_selected_bg_color: Color::Rgb(0, 120, 212),
            file_marked_color: Color::Rgb(255, 215, 0), // 골드색
            file_marked_symbol_color: Color::Rgb(255, 215, 0), // 골드색
            directory_color: Color::Rgb(86, 156, 214),
            executable_color: Color::Rgb(78, 201, 176),
            symlink_color: Color::Rgb(206, 145, 120),
            icon_mode: IconMode::default(),
            sort_by: SortBy::Name,
            sort_order: SortOrder::Ascending,
            filter_pattern: None,
            size_format: SizeFormat::default(),
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

    /// 다중 선택 항목 설정
    pub fn selected_items(mut self, items: &'a HashSet<usize>) -> Self {
        self.selected_items = items;
        self
    }

    /// 아이콘 모드 설정
    pub fn icon_mode(mut self, mode: IconMode) -> Self {
        self.icon_mode = mode;
        self
    }

    /// 정렬 상태 설정
    pub fn sort_state(mut self, sort_by: SortBy, sort_order: SortOrder) -> Self {
        self.sort_by = sort_by;
        self.sort_order = sort_order;
        self
    }

    /// 필터 패턴 설정 (하이라이트용)
    pub fn filter_pattern(mut self, pattern: Option<&'a str>) -> Self {
        self.filter_pattern = pattern;
        self
    }

    /// 크기 표시 형식 설정
    pub fn size_format(mut self, format: SizeFormat) -> Self {
        self.size_format = format;
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
        self.file_marked_color = theme.file_marked.to_color();
        self.file_marked_symbol_color = theme.file_marked_symbol.to_color();
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
        match self.icon_mode {
            IconMode::Emoji => match file_type {
                FileType::Directory => "📁",
                FileType::File => "📄",
                FileType::Executable => "🔧",
                FileType::Symlink => "🔗",
            },
            IconMode::Ascii => match file_type {
                FileType::Directory => "/",
                FileType::File => " ",
                FileType::Executable => "*",
                FileType::Symlink => "@",
            },
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

/// 컬럼 레이아웃 정보
struct ColumnLayout {
    show_permissions: bool,
    show_size: bool,
    date_format: &'static str,
    name_width: usize,
    size_width: usize,
    date_width: usize,
    perm_width: usize,
}

impl Panel<'_> {
    /// 패널 너비 기반 컬럼 표시 여부/크기 결정
    fn calculate_column_layout(width: usize, has_scrollbar: bool) -> ColumnLayout {
        let scrollbar_width = if has_scrollbar { 1 } else { 0 };

        let (show_permissions, show_size, date_format) = match width {
            w if w >= 70 => (true, true, "long"),
            w if w >= 45 => (false, true, "short"),
            _ => (false, false, "short"),
        };

        let perm_width = if show_permissions { 12 } else { 0 };
        let date_width = if date_format == "long" { 17 } else { 12 };
        let size_width = if show_size { 10 } else { 0 };
        let margins = 6;
        let name_width = width
            .saturating_sub(perm_width)
            .saturating_sub(size_width)
            .saturating_sub(date_width)
            .saturating_sub(margins)
            .saturating_sub(scrollbar_width);

        ColumnLayout {
            show_permissions,
            show_size,
            date_format,
            name_width,
            size_width,
            date_width,
            perm_width,
        }
    }

    /// 헤더 행 + 구분선 렌더링. y를 2 증가시킨다.
    fn render_header(
        layout: &ColumnLayout,
        inner: Rect,
        buf: &mut Buffer,
        y: &mut u16,
        sort_by: SortBy,
        sort_order: SortOrder,
    ) {
        let header_style = Style::default()
            .fg(Color::Rgb(150, 150, 150))
            .add_modifier(Modifier::BOLD);

        let arrow = match sort_order {
            SortOrder::Ascending => "▲",
            SortOrder::Descending => "▼",
        };

        // Name 헤더 (Extension 정렬 시 "Name(Ext)" 표시)
        let name_label = match sort_by {
            SortBy::Name => format!("Name {}", arrow),
            SortBy::Extension => format!("Name(Ext) {}", arrow),
            _ => "Name".to_string(),
        };

        let mut header_spans = vec![Span::raw(" ")];
        header_spans.push(Span::styled(
            format!("{:<width$}", name_label, width = layout.name_width),
            header_style,
        ));

        if layout.show_size {
            let size_label = if sort_by == SortBy::Size {
                format!("Size {}", arrow)
            } else {
                "Size".to_string()
            };
            header_spans.push(Span::raw(" "));
            header_spans.push(Span::styled(format!("{:<10}", size_label), header_style));
        }

        let modified_label = if sort_by == SortBy::Modified {
            format!("Modified {}", arrow)
        } else {
            "Modified".to_string()
        };
        header_spans.push(Span::raw(" "));
        header_spans.push(Span::styled(
            format!("{:<width$}", modified_label, width = layout.date_width),
            header_style,
        ));

        if layout.show_permissions {
            header_spans.push(Span::raw(" "));
            header_spans.push(Span::styled(format!("{:<11}", "Permissions"), header_style));
        }

        let header_line = Line::from(header_spans);
        buf.set_line(inner.x, inner.y + *y, &header_line, inner.width);
        *y += 1;

        let separator = "─".repeat(inner.width as usize);
        buf.set_string(
            inner.x,
            inner.y + *y,
            separator,
            Style::default().fg(Color::Rgb(60, 60, 60)),
        );
        *y += 1;
    }

    /// ".." 항목 렌더링
    fn render_parent_entry(&self, inner: Rect, buf: &mut Buffer, y: &mut u16) {
        let is_selected = self.selected_index == 0;
        let style = if is_selected {
            Style::default()
                .bg(self.file_selected_bg_color)
                .fg(self.file_selected_color)
        } else {
            Style::default().fg(Color::Rgb(150, 150, 150))
        };

        let parent_text = "[..]";
        let padding_width = (inner.width as usize).saturating_sub(parent_text.len() + 1);
        let padding = " ".repeat(padding_width);

        let parent_spans = vec![
            Span::styled(" ", style),
            Span::styled(parent_text, style),
            Span::styled(padding, style),
        ];

        let parent_line = Line::from(parent_spans);
        buf.set_line(inner.x, inner.y + *y, &parent_line, inner.width);
        *y += 1;
    }

    /// 단일 파일 행 렌더링
    fn render_file_entry(
        &self,
        entry: &FileEntry,
        entry_index: usize,
        layout: &ColumnLayout,
        inner: Rect,
        buf: &mut Buffer,
        y: &mut u16,
    ) {
        let is_cursor = if self.show_parent {
            entry_index + 1 == self.selected_index
        } else {
            entry_index == self.selected_index
        };
        let is_marked = self.selected_items.contains(&entry_index);

        let (fg, bg, marker) = match (is_cursor, is_marked) {
            (true, true) => (
                self.file_marked_color,
                Some(self.file_selected_bg_color),
                "*",
            ),
            (true, false) => (
                self.file_selected_color,
                Some(self.file_selected_bg_color),
                " ",
            ),
            (false, true) => (self.file_marked_color, None, "*"),
            (false, false) => (self.file_color(&entry.file_type), None, " "),
        };

        let style = if let Some(bg_color) = bg {
            Style::default().fg(fg).bg(bg_color)
        } else {
            Style::default().fg(fg)
        };

        let marker_style = if is_marked {
            Style::default()
                .fg(self.file_marked_symbol_color)
                .bg(bg.unwrap_or(self.bg_color))
        } else if let Some(bg_color) = bg {
            Style::default().bg(bg_color)
        } else {
            Style::default()
        };

        let mut line_spans = vec![Span::styled(marker, marker_style)];

        // 아이콘 + 파일명 (필터 하이라이트 지원)
        let icon = self.file_icon(&entry.file_type);
        let display_name = self.truncate_name(&entry.name, layout.name_width.saturating_sub(4));
        let icon_str = format!("{} ", icon);
        line_spans.push(Span::styled(&icon_str, style));

        let highlight_style = if let Some(bg_color) = bg {
            Style::default()
                .fg(Color::Rgb(255, 255, 100))
                .bg(bg_color)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default()
                .fg(Color::Rgb(255, 255, 100))
                .add_modifier(Modifier::UNDERLINED)
        };

        if let Some(pattern) = self.filter_pattern {
            if !pattern.is_empty() && !glob::is_glob_pattern(pattern) {
                // contains 매칭: 매칭 부분만 하이라이트
                let name_lower = display_name.to_lowercase();
                let pattern_lower = pattern.to_lowercase();
                if let Some(pos) = name_lower.find(&pattern_lower) {
                    let before: String = display_name.chars().take(pos).collect();
                    let matched: String = display_name
                        .chars()
                        .skip(pos)
                        .take(pattern_lower.len())
                        .collect();
                    let after: String = display_name
                        .chars()
                        .skip(pos + pattern_lower.len())
                        .collect();
                    line_spans.push(Span::styled(before, style));
                    line_spans.push(Span::styled(matched, highlight_style));
                    line_spans.push(Span::styled(after, style));
                } else {
                    line_spans.push(Span::styled(&display_name, style));
                }
            } else if glob::is_glob_pattern(pattern) {
                // glob 매칭: 전체 이름에 하이라이트 스타일
                line_spans.push(Span::styled(&display_name, highlight_style));
            } else {
                line_spans.push(Span::styled(&display_name, style));
            }
        } else {
            line_spans.push(Span::styled(&display_name, style));
        }

        let name_with_icon_width = icon_str.width() + display_name.width();
        let name_padding = layout.name_width.saturating_sub(name_with_icon_width + 1);
        line_spans.push(Span::styled(" ".repeat(name_padding), style));

        // 크기
        if layout.show_size {
            line_spans.push(Span::styled(" ", style));
            let size_str = if entry.is_directory() {
                "-".to_string()
            } else {
                match self.size_format {
                    SizeFormat::Auto => format_file_size(entry.size),
                    SizeFormat::Bytes => format_file_size_bytes(entry.size),
                }
            };
            line_spans.push(Span::styled(format!("{:>9}", size_str), style));
        }

        // 날짜 (format_date()는 항상 "YYYY-MM-DD HH:MM" 16자 반환)
        line_spans.push(Span::styled(" ", style));
        let full_date = format_date(entry.modified);
        let date_str = if layout.date_format == "long" {
            full_date
        } else {
            // short: "MM-DD HH:MM" (11자)
            full_date.get(5..).unwrap_or(&full_date).to_string()
        };
        line_spans.push(Span::styled(
            format!("{:<width$}", date_str, width = layout.date_width),
            style,
        ));

        // 권한
        if layout.show_permissions {
            line_spans.push(Span::styled(" ", style));
            let perm_str = format_permissions(entry.permissions.as_ref());
            line_spans.push(Span::styled(format!("{:<11}", perm_str), style));
        }

        let file_line = Line::from(line_spans);
        buf.set_line(inner.x, inner.y + *y, &file_line, inner.width);
        *y += 1;
    }

    /// 빈 패널 메시지 렌더링
    fn render_empty_state(inner: Rect, buf: &mut Buffer, y: u16) {
        let empty_text = Line::from(vec![Span::styled(
            " (No files)",
            Style::default().fg(Color::Rgb(100, 100, 100)),
        )]);
        buf.set_line(inner.x, inner.y + y, &empty_text, inner.width);
    }
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let title_max_width = (area.width as usize).saturating_sub(4);
        let display_title = self.truncate_path(self.title, title_max_width);

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

        if inner.height < 3 {
            return;
        }

        // 스크롤바 필요 여부 계산 (헤더 2줄 + ".." 1줄 차감)
        let header_lines: usize = 2;
        let parent_line: usize = if self.show_parent { 1 } else { 0 };
        let file_area_height = (inner.height as usize).saturating_sub(header_lines + parent_line);
        let has_scrollbar = self.entries.len() > file_area_height;

        let layout = Self::calculate_column_layout(inner.width as usize, has_scrollbar);
        let mut y: u16 = 0;

        Self::render_header(&layout, inner, buf, &mut y, self.sort_by, self.sort_order);

        if self.show_parent {
            self.render_parent_entry(inner, buf, &mut y);
        }

        let available_height = (inner.height as usize).saturating_sub(y as usize);
        let start = self.scroll_offset;
        let end = (start + available_height).min(self.entries.len());

        for (i, entry) in self.entries[start..end].iter().enumerate() {
            let entry_index = start + i;
            self.render_file_entry(entry, entry_index, &layout, inner, buf, &mut y);
            if y >= inner.height {
                break;
            }
        }

        if self.entries.is_empty() && !self.show_parent && y < inner.height {
            Self::render_empty_state(inner, buf, y);
        }

        // 스크롤바 렌더링
        if has_scrollbar {
            let total_items = self.entries.len();
            let track_height = file_area_height;
            if track_height > 0 && total_items > 0 {
                let thumb_height = (track_height * track_height / total_items).max(1);
                let max_scroll = total_items.saturating_sub(file_area_height);
                let thumb_pos = if max_scroll == 0 {
                    0
                } else {
                    self.scroll_offset * (track_height.saturating_sub(thumb_height)) / max_scroll
                };

                let scrollbar_x = inner.x + inner.width - 1;
                let track_start_y = inner.y + (header_lines + parent_line) as u16;

                let track_style = Style::default().fg(Color::Rgb(60, 60, 60));
                let thumb_style = Style::default().fg(Color::Rgb(150, 150, 150));

                for i in 0..track_height {
                    let sy = track_start_y + i as u16;
                    if sy < inner.y + inner.height {
                        let (symbol, style) = if i >= thumb_pos && i < thumb_pos + thumb_height {
                            ("┃", thumb_style)
                        } else {
                            ("│", track_style)
                        };
                        buf.set_string(scrollbar_x, sy, symbol, style);
                    }
                }
            }
        }
    }
}

impl Panel<'_> {
    /// 파일명을 최대 너비로 잘라냄 (확장자 보존)
    ///
    /// 중간 생략 방식: "very_long_fi...ated.txt" (확장자 유지)
    /// 확장자 없거나 숨김파일(.bashrc)은 끝에서 자름
    fn truncate_name(&self, name: &str, max_width: usize) -> String {
        let display_width = name.width();
        if display_width <= max_width {
            return name.to_string();
        }

        let ellipsis = "...";
        let ellipsis_width = 3;

        // 확장자 분리: 마지막 '.' 기준 (숨김파일 제외)
        let (stem, ext) = match name.rfind('.') {
            Some(dot_pos) if dot_pos > 0 => (&name[..dot_pos], &name[dot_pos..]),
            _ => (name, ""),
        };

        let ext_width = ext.width();

        // 확장자 + "..." 만으로 max_width 초과 시 끝에서 자르기 방식
        if ellipsis_width + ext_width >= max_width || ext.is_empty() {
            let mut truncated = String::new();
            let mut current_width = 0;
            for ch in name.chars() {
                let ch_width = ch.width().unwrap_or(1);
                if current_width + ch_width + ellipsis_width > max_width {
                    truncated.push_str(ellipsis);
                    break;
                }
                truncated.push(ch);
                current_width += ch_width;
            }
            return truncated;
        }

        // 중간 생략: stem 앞부분 + "..." + 확장자
        let available_stem_width = max_width - ellipsis_width - ext_width;
        let mut truncated = String::new();
        let mut current_width = 0;
        for ch in stem.chars() {
            let ch_width = ch.width().unwrap_or(1);
            if current_width + ch_width > available_stem_width {
                break;
            }
            truncated.push(ch);
            current_width += ch_width;
        }
        truncated.push_str(ellipsis);
        truncated.push_str(ext);
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

        // 긴 이름은 중간 생략 + 확장자 보존
        let long_name = "very_long_filename_that_should_be_truncated.txt";
        let truncated = panel.truncate_name(long_name, 20);
        assert!(truncated.contains("..."));
        assert!(truncated.ends_with(".txt")); // 확장자 보존

        // 확장자 없는 파일은 끝에서 자름
        let no_ext = "very_long_filename_without_extension";
        let truncated = panel.truncate_name(no_ext, 15);
        assert!(truncated.ends_with("..."));

        // 숨김 파일(.bashrc)은 끝에서 자름
        let hidden = ".very_long_hidden_config_file";
        let truncated = panel.truncate_name(hidden, 15);
        assert!(truncated.ends_with("..."));
    }
}
