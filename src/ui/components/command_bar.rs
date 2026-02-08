#![allow(dead_code)]
// Command bar component - 하단 커맨드 바 컴포넌트
//
// Vim 스타일 단축키 표시 (화면 너비에 따라 우선순위 기반 동적 표시)

use crate::core::actions::generate_command_bar_items;
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
    /// 단축키
    pub key: String,
    /// 레이블
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

    /// 렌더링 시 필요한 너비 계산 ("key:label" + 구분자 1)
    fn display_width(&self) -> usize {
        self.key.len() + 1 + self.label.len()
    }
}

/// 커맨드 바 컴포넌트
pub struct CommandBar {
    /// 커맨드 항목들 (우선순위 순서)
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

    /// 기본 커맨드 목록 (액션 레지스트리에서 생성)
    fn default_commands() -> Vec<CommandItem> {
        generate_command_bar_items()
    }

    /// 커맨드 목록 설정
    pub fn commands(mut self, commands: Vec<CommandItem>) -> Self {
        self.commands = commands;
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

        let available_width = area.width as usize;
        if available_width < 3 {
            return;
        }

        // 우선순위 순서로 항목을 채워넣기 (화면 너비에 맞게)
        let padding = 1; // 왼쪽 패딩
        let separator = 2; // 항목 간 구분자 " | "
        let mut used_width = padding;
        let mut visible_count = 0;

        for cmd in &self.commands {
            let item_width = cmd.display_width();
            let needed = if visible_count == 0 {
                item_width
            } else {
                separator + item_width
            };

            if used_width + needed > available_width {
                break;
            }

            used_width += needed;
            visible_count += 1;
        }

        // 스팬 생성
        let mut spans = Vec::new();
        spans.push(Span::raw(" ")); // 왼쪽 패딩

        let sep_style = Style::default().fg(Color::Rgb(80, 80, 80));

        for (i, cmd) in self.commands.iter().take(visible_count).enumerate() {
            if i > 0 {
                spans.push(Span::styled(" | ", sep_style));
            }

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
        let item = CommandItem::new("y", "Copy");
        assert_eq!(item.key, "y");
        assert_eq!(item.label, "Copy");
        assert!(item.enabled);
    }

    #[test]
    fn test_command_bar_default() {
        let bar = CommandBar::default();
        assert_eq!(bar.commands.len(), 19);
    }

    #[test]
    fn test_display_width() {
        let item = CommandItem::new("y", "Copy");
        assert_eq!(item.display_width(), 6); // "y:Copy"
    }

    #[test]
    fn test_width_based_visibility() {
        let bar = CommandBar::default();
        // 첫 번째 항목 "y:Copy" = 6자, 패딩 1 = 7자 필요
        // 매우 좁은 화면에서는 일부만 표시됨
        let first_item_width = bar.commands[0].display_width();
        assert_eq!(first_item_width, 6);
    }
}
