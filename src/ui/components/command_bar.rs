#![allow(dead_code)]
// Command bar component - 하단 커맨드 바 컴포넌트
//
// F1~F12 단축키 표시

use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

/// 커맨드 항목
#[derive(Debug, Clone)]
pub struct CommandItem {
    /// 단축키 (F1, F2, ...)
    pub key: String,
    /// 레이블 (Help, Menu, ...)
    pub label: String,
    /// 활성화 여부
    pub enabled: bool,
}

impl CommandItem {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// 커맨드 바 컴포넌트
pub struct CommandBar {
    /// 커맨드 항목들
    commands: Vec<CommandItem>,
    /// 배경색
    bg_color: Color,
    /// 전경색 (키)
    key_fg_color: Color,
    /// 전경색 (레이블)
    label_fg_color: Color,
    /// 비활성 색상
    disabled_color: Color,
}

impl Default for CommandBar {
    fn default() -> Self {
        Self {
            commands: Self::default_commands(),
            bg_color: Color::Rgb(30, 30, 30),
            key_fg_color: Color::Rgb(0, 120, 212),
            label_fg_color: Color::Rgb(212, 212, 212),
            disabled_color: Color::Rgb(100, 100, 100),
        }
    }
}

impl CommandBar {
    pub fn new() -> Self {
        Self::default()
    }

    /// 기본 커맨드 목록
    fn default_commands() -> Vec<CommandItem> {
        vec![
            CommandItem::new("F1", "Help"),
            CommandItem::new("F2", "Menu"),
            CommandItem::new("F3", "View"),
            CommandItem::new("F4", "Edit"),
            CommandItem::new("F5", "Copy"),
            CommandItem::new("F6", "Move"),
            CommandItem::new("F7", "Dir"),
            CommandItem::new("F8", "Del"),
            CommandItem::new("F10", "Quit"),
        ]
    }

    /// 커맨드 목록 설정
    pub fn commands(mut self, commands: Vec<CommandItem>) -> Self {
        self.commands = commands;
        self
    }

    /// 배경색 설정
    pub fn bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// 키 전경색 설정
    pub fn key_fg_color(mut self, color: Color) -> Self {
        self.key_fg_color = color;
        self
    }

    /// 레이블 전경색 설정
    pub fn label_fg_color(mut self, color: Color) -> Self {
        self.label_fg_color = color;
        self
    }

    /// 테마 적용
    pub fn theme(mut self, theme: &Theme) -> Self {
        self.bg_color = theme.command_bar_bg.to_color();
        self.key_fg_color = theme.accent.to_color();
        self.label_fg_color = theme.command_bar_fg.to_color();
        self
    }
}

impl Widget for CommandBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 배경 채우기
        buf.set_style(area, Style::default().bg(self.bg_color));

        // 커맨드 항목들을 스팬으로 변환
        let mut spans = Vec::new();
        spans.push(Span::raw(" ")); // 왼쪽 패딩

        for (i, cmd) in self.commands.iter().enumerate() {
            let (key_style, label_style) = if cmd.enabled {
                (
                    Style::default()
                        .fg(self.key_fg_color)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(self.label_fg_color),
                )
            } else {
                (
                    Style::default().fg(self.disabled_color),
                    Style::default().fg(self.disabled_color),
                )
            };

            spans.push(Span::styled(&cmd.key, key_style));
            spans.push(Span::styled(":", label_style));
            spans.push(Span::styled(&cmd.label, label_style));

            // 마지막 항목이 아니면 구분자 추가
            if i < self.commands.len() - 1 {
                spans.push(Span::raw(" "));
            }
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_item_creation() {
        let item = CommandItem::new("F1", "Help");
        assert_eq!(item.key, "F1");
        assert_eq!(item.label, "Help");
        assert!(item.enabled);
    }

    #[test]
    fn test_command_bar_default() {
        let bar = CommandBar::default();
        assert_eq!(bar.commands.len(), 9);
    }
}
