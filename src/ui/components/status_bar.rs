#![allow(dead_code)]
// Status bar component - 상태바 컴포넌트
//
// 파일/디렉토리 개수, 총 크기, 선택된 항목 정보 표시

use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

/// 상태바 컴포넌트
pub struct StatusBar<'a> {
    /// 파일 개수
    file_count: usize,
    /// 디렉토리 개수
    dir_count: usize,
    /// 총 크기 (포맷된 문자열)
    total_size: &'a str,
    /// 선택된 항목 수
    selected_count: usize,
    /// 선택된 항목 총 크기 (포맷된 문자열)
    selected_size: &'a str,
    /// 레이아웃 모드 표시 (싱글/듀얼)
    layout_mode: &'a str,
    /// 배경색
    bg_color: Color,
    /// 전경색
    fg_color: Color,
}

impl<'a> Default for StatusBar<'a> {
    fn default() -> Self {
        Self {
            file_count: 0,
            dir_count: 0,
            total_size: "0B",
            selected_count: 0,
            selected_size: "0B",
            layout_mode: "DUAL",
            bg_color: Color::Rgb(30, 30, 30),
            fg_color: Color::Rgb(212, 212, 212),
        }
    }
}

impl<'a> StatusBar<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// 파일 개수 설정
    pub fn file_count(mut self, count: usize) -> Self {
        self.file_count = count;
        self
    }

    /// 디렉토리 개수 설정
    pub fn dir_count(mut self, count: usize) -> Self {
        self.dir_count = count;
        self
    }

    /// 총 크기 설정
    pub fn total_size(mut self, size: &'a str) -> Self {
        self.total_size = size;
        self
    }

    /// 선택된 항목 수 설정
    pub fn selected_count(mut self, count: usize) -> Self {
        self.selected_count = count;
        self
    }

    /// 선택된 항목 총 크기 설정
    pub fn selected_size(mut self, size: &'a str) -> Self {
        self.selected_size = size;
        self
    }

    /// 레이아웃 모드 설정
    pub fn layout_mode(mut self, mode: &'a str) -> Self {
        self.layout_mode = mode;
        self
    }

    /// 배경색 설정
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// 전경색 설정
    pub fn fg_color(mut self, color: Color) -> Self {
        self.fg_color = color;
        self
    }

    /// 테마 적용
    pub fn theme(mut self, theme: &Theme) -> Self {
        self.bg_color = theme.status_bar_bg.to_color();
        self.fg_color = theme.status_bar_fg.to_color();
        self
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 배경 채우기
        buf.set_style(area, Style::default().bg(self.bg_color));

        // 왼쪽 정보: 파일/디렉토리 개수, 크기
        let left_info = format!(
            " {} files, {} dirs | {}",
            self.file_count, self.dir_count, self.total_size
        );

        // 선택 정보 (있을 경우)
        let selected_info = if self.selected_count > 0 {
            format!(
                " | {} selected ({})",
                self.selected_count, self.selected_size
            )
        } else {
            String::new()
        };

        // 오른쪽 정보: 레이아웃 모드
        let right_info = format!("[{}] ", self.layout_mode);

        // 가용 공간 계산
        let left_len = left_info.len() + selected_info.len();
        let right_len = right_info.len();
        let padding_len = area
            .width
            .saturating_sub(left_len as u16 + right_len as u16) as usize;
        let padding = " ".repeat(padding_len);

        let spans = vec![
            Span::styled(&left_info, Style::default().fg(self.fg_color)),
            Span::styled(&selected_info, Style::default().fg(Color::Yellow)),
            Span::raw(padding),
            Span::styled(right_info, Style::default().fg(Color::Rgb(100, 100, 100))),
        ];

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_creation() {
        let status_bar = StatusBar::new()
            .file_count(10)
            .dir_count(5)
            .total_size("1.2GB");

        assert_eq!(status_bar.file_count, 10);
        assert_eq!(status_bar.dir_count, 5);
        assert_eq!(status_bar.total_size, "1.2GB");
    }
}
