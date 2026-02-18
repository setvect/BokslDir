#![allow(dead_code)]
// Status bar component - 상태바 컴포넌트
//
// 파일/디렉토리 개수, 총 크기, 선택된 항목 정보 표시

use crate::ui::{I18n, Language, MessageKey, TextKey, Theme};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

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
    /// 대기 키 표시 (Phase 4)
    pending_key: Option<&'a str>,
    /// 토스트 메시지 (한글 IME 등)
    toast: Option<&'a str>,
    /// 정렬 정보 표시
    sort_info: Option<&'a str>,
    /// 필터 정보 표시
    filter_info: Option<&'a str>,
    /// 숨김 파일 표시 여부
    show_hidden: bool,
    /// IME 상태 표시
    ime_info: Option<&'a str>,
    /// 배경색
    bg_color: Color,
    /// 전경색
    fg_color: Color,
    /// 강조색
    accent_color: Color,
    /// 경고색
    warning_color: Color,
    /// 성공색
    success_color: Color,
    /// 보조(희미한) 색
    muted_color: Color,
    language: Language,
}

impl<'a> Default for StatusBar<'a> {
    fn default() -> Self {
        Self {
            file_count: 0,
            dir_count: 0,
            total_size: "0B",
            selected_count: 0,
            selected_size: "0B",
            pending_key: None,
            toast: None,
            sort_info: None,
            filter_info: None,
            show_hidden: false,
            ime_info: None,
            bg_color: Color::Rgb(30, 30, 30),
            fg_color: Color::Rgb(212, 212, 212),
            accent_color: Color::Rgb(0, 120, 212),
            warning_color: Color::Rgb(255, 180, 50),
            success_color: Color::Rgb(100, 200, 100),
            muted_color: Color::Rgb(100, 100, 100),
            language: Language::English,
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

    /// 대기 키 표시 설정
    pub fn pending_key(mut self, key: Option<&'a str>) -> Self {
        self.pending_key = key;
        self
    }

    /// 토스트 메시지 설정
    pub fn toast(mut self, toast: Option<&'a str>) -> Self {
        self.toast = toast;
        self
    }

    /// 정렬 정보 설정
    pub fn sort_info(mut self, info: Option<&'a str>) -> Self {
        self.sort_info = info;
        self
    }

    /// 필터 정보 설정
    pub fn filter_info(mut self, info: Option<&'a str>) -> Self {
        self.filter_info = info;
        self
    }

    /// 숨김 파일 표시 여부 설정
    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    /// IME 상태 설정
    pub fn ime_info(mut self, info: Option<&'a str>) -> Self {
        self.ime_info = info;
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
        self.accent_color = theme.accent.to_color();
        self.warning_color = theme.warning.to_color();
        self.success_color = theme.success.to_color();
        self.muted_color = theme.panel_inactive_border.to_color();
        self
    }

    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let i18n = I18n::new(self.language);
        // 배경 채우기
        buf.set_style(area, Style::default().bg(self.bg_color));

        let w = area.width as usize;

        // 토스트 메시지가 있으면 토스트만 표시
        if let Some(toast_msg) = self.toast {
            let toast_text = format!(" {} ", toast_msg);
            let toast_style = Style::default().fg(self.warning_color).bg(self.bg_color);
            let line = Line::from(Span::styled(&toast_text, toast_style));
            Paragraph::new(line).render(area, buf);
            return;
        }

        // 왼쪽 정보: 터미널 너비에 따라 3단계
        let left_info = if w >= 60 {
            i18n.fmt(
                MessageKey::StatusLeftLong,
                &[
                    ("files", self.file_count.to_string()),
                    ("dirs", self.dir_count.to_string()),
                    ("total", self.total_size.to_string()),
                ],
            )
        } else if w >= 40 {
            format!(
                " {}f {}d | {}",
                self.file_count, self.dir_count, self.total_size
            )
        } else {
            format!(
                " {} items | {}",
                self.file_count + self.dir_count,
                self.total_size
            )
        };

        // 선택 정보 (있을 경우, 너비 적응)
        let selected_info = if self.selected_count > 0 {
            if w >= 60 {
                i18n.fmt(
                    MessageKey::StatusSelectedLong,
                    &[
                        ("count", self.selected_count.to_string()),
                        ("size", self.selected_size.to_string()),
                    ],
                )
            } else if w >= 40 {
                format!(" | {}sel", self.selected_count)
            } else {
                format!(" {}sel", self.selected_count)
            }
        } else {
            String::new()
        };

        // 대기 키 표시
        let pending_info = match self.pending_key {
            Some(key) => format!(" [{}]", key),
            None => String::new(),
        };

        // 정렬 정보
        let sort_info_str = if let Some(info) = self.sort_info {
            format!("[{}] ", info)
        } else {
            String::new()
        };

        // 필터 정보
        let filter_info_str = if let Some(info) = self.filter_info {
            format!("[{}] ", info)
        } else {
            String::new()
        };

        // 숨김 파일 표시 정보
        let hidden_info_str = if self.show_hidden {
            format!("[{}] ", i18n.tr(TextKey::Hidden))
        } else {
            String::new()
        };

        // IME 상태 표시
        let ime_info_str = if let Some(info) = self.ime_info {
            format!("[{}] ", info)
        } else {
            String::new()
        };

        // 가용 공간 계산 (unicode width 사용)
        let right_total_width = UnicodeWidthStr::width(ime_info_str.as_str())
            + UnicodeWidthStr::width(hidden_info_str.as_str())
            + UnicodeWidthStr::width(filter_info_str.as_str())
            + UnicodeWidthStr::width(sort_info_str.as_str());

        let left_len = left_info.len() + selected_info.len() + pending_info.len();
        let padding_len =
            area.width
                .saturating_sub(left_len as u16 + right_total_width as u16) as usize;
        let padding = " ".repeat(padding_len);

        // IME 상태 색상: 한글이면 노란색 경고, 영문이면 녹색
        let ime_color = if self.ime_info == Some("한글") {
            self.warning_color
        } else {
            self.success_color
        };

        let spans = vec![
            Span::styled(&left_info, Style::default().fg(self.fg_color)),
            Span::styled(&selected_info, Style::default().fg(self.warning_color)),
            Span::styled(&pending_info, Style::default().fg(self.accent_color)),
            Span::raw(padding),
            Span::styled(hidden_info_str, Style::default().fg(self.warning_color)),
            Span::styled(filter_info_str, Style::default().fg(self.success_color)),
            Span::styled(sort_info_str, Style::default().fg(self.accent_color)),
            Span::styled(ime_info_str, Style::default().fg(ime_color)),
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

    #[test]
    fn test_status_bar_with_ime() {
        let status_bar = StatusBar::new().ime_info(Some("한글"));
        assert_eq!(status_bar.ime_info, Some("한글"));

        let status_bar = StatusBar::new().ime_info(Some("EN"));
        assert_eq!(status_bar.ime_info, Some("EN"));

        let status_bar = StatusBar::new().ime_info(None);
        assert_eq!(status_bar.ime_info, None);
    }
}
