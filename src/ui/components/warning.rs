#![allow(dead_code)]
// Warning screen component - 경고 화면 컴포넌트
//
// 터미널이 너무 작을 때 표시되는 경고 화면

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::ui::layout::{MIN_HEIGHT, MIN_WIDTH};
use crate::ui::Theme;

/// 경고 화면 컴포넌트
pub struct WarningScreen {
    /// 현재 터미널 크기
    current_size: (u16, u16),
    /// 경고 색상
    warning_color: Color,
    /// 배경색
    bg_color: Color,
    /// 전경색
    fg_color: Color,
    /// 에러 색상 (현재 크기)
    error_color: Color,
    /// 성공/권장 색상 (요구 크기)
    success_color: Color,
}

impl Default for WarningScreen {
    fn default() -> Self {
        Self {
            current_size: (0, 0),
            warning_color: Color::Yellow,
            bg_color: Color::Rgb(30, 30, 30),
            fg_color: Color::Rgb(212, 212, 212),
            error_color: Color::Red,
            success_color: Color::Green,
        }
    }
}

impl WarningScreen {
    pub fn new() -> Self {
        Self::default()
    }

    /// 현재 터미널 크기 설정
    pub fn current_size(mut self, width: u16, height: u16) -> Self {
        self.current_size = (width, height);
        self
    }

    /// 경고 색상 설정
    pub fn warning_color(mut self, color: Color) -> Self {
        self.warning_color = color;
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
        self.warning_color = theme.warning.to_color();
        self.bg_color = theme.bg_primary.to_color();
        self.fg_color = theme.fg_primary.to_color();
        self.error_color = theme.error.to_color();
        self.success_color = theme.success.to_color();
        self
    }
}

impl Widget for WarningScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 배경 채우기
        buf.set_style(area, Style::default().bg(self.bg_color));

        // 경고 메시지 구성
        let warning_icon = Line::from(vec![Span::styled(
            "⚠",
            Style::default()
                .fg(self.warning_color)
                .add_modifier(Modifier::BOLD),
        )]);

        let title_line = Line::from(vec![Span::styled(
            "Terminal Too Small",
            Style::default()
                .fg(self.warning_color)
                .add_modifier(Modifier::BOLD),
        )]);

        let current_size_line = Line::from(vec![
            Span::styled("Current: ", Style::default().fg(self.fg_color)),
            Span::styled(
                format!("{}x{}", self.current_size.0, self.current_size.1),
                Style::default()
                    .fg(self.error_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let required_size_line = Line::from(vec![
            Span::styled("Required: ", Style::default().fg(self.fg_color)),
            Span::styled(
                format!("{}x{}", MIN_WIDTH, MIN_HEIGHT),
                Style::default()
                    .fg(self.success_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let hint_line = Line::from(vec![Span::styled(
            "Please resize your terminal",
            Style::default()
                .fg(self.fg_color)
                .add_modifier(Modifier::DIM),
        )]);

        let lines = vec![
            warning_icon,
            Line::from(""),
            title_line,
            Line::from(""),
            current_size_line,
            required_size_line,
            Line::from(""),
            hint_line,
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.warning_color))
            .style(Style::default().bg(self.bg_color));

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(block);

        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warning_screen_creation() {
        let screen = WarningScreen::new().current_size(30, 10);

        assert_eq!(screen.current_size, (30, 10));
    }
}
