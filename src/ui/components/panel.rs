#![allow(dead_code)]
// Panel component - 파일 패널 컴포넌트
//
// 파일 리스트 표시, 선택 상태, 테두리 렌더링

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

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
    /// 내용 (임시 - 파일 리스트 구현 전)
    content: &'a str,
    /// 활성 테두리 색상
    active_border_color: Color,
    /// 비활성 테두리 색상
    inactive_border_color: Color,
    /// 패널 배경색
    bg_color: Color,
    /// 패널 전경색
    fg_color: Color,
}

impl<'a> Default for Panel<'a> {
    fn default() -> Self {
        Self {
            title: "",
            status: PanelStatus::default(),
            content: "",
            active_border_color: Color::Rgb(0, 120, 212),
            inactive_border_color: Color::Rgb(60, 60, 60),
            bg_color: Color::Rgb(30, 30, 30),
            fg_color: Color::Rgb(212, 212, 212),
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

    /// 내용 설정
    pub fn content(mut self, content: &'a str) -> Self {
        self.content = content;
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

    /// 전경색 설정
    pub fn fg_color(mut self, color: Color) -> Self {
        self.fg_color = color;
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
        let base = Style::default().fg(self.fg_color);
        match self.status {
            PanelStatus::Active => base.add_modifier(Modifier::BOLD),
            PanelStatus::Inactive => base,
        }
    }
}

impl Widget for Panel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 빈 영역은 렌더링하지 않음
        if area.width == 0 || area.height == 0 {
            return;
        }

        // 블록 생성
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color()))
            .title(Span::styled(
                format!(" {} ", self.title),
                self.title_style(),
            ))
            .style(Style::default().bg(self.bg_color));

        // 빈 패널 또는 내용 있는 패널 렌더링
        if self.content.is_empty() {
            // 빈 패널 상태 표시
            let inner = block.inner(area);
            block.render(area, buf);

            if inner.height > 0 {
                let empty_text = Line::from(vec![Span::styled(
                    "<empty>",
                    Style::default().fg(Color::Rgb(100, 100, 100)),
                )]);
                let paragraph = Paragraph::new(empty_text);
                paragraph.render(inner, buf);
            }
        } else {
            // 내용 렌더링
            let paragraph = Paragraph::new(self.content)
                .style(Style::default().fg(self.fg_color))
                .block(block);
            paragraph.render(area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = Panel::new().title("/home/user").active();

        assert_eq!(panel.title, "/home/user");
        assert_eq!(panel.status, PanelStatus::Active);
    }

    #[test]
    fn test_panel_status_toggle() {
        let active_panel = Panel::new().active();
        assert_eq!(active_panel.status, PanelStatus::Active);

        let inactive_panel = Panel::new().inactive();
        assert_eq!(inactive_panel.status, PanelStatus::Inactive);
    }
}
