#![allow(dead_code)]
// Menu bar component - 상단 메뉴바 컴포넌트
//
// 앱 이름, 메뉴 항목, 현재 경로 표시

use super::dropdown_menu::Menu;
use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

/// 메뉴바 컴포넌트
pub struct MenuBar<'a> {
    /// 앱 이름
    app_name: &'a str,
    /// 메뉴 목록
    menus: &'a [Menu],
    /// 메뉴가 활성화되어 있는지
    menu_active: bool,
    /// 현재 선택된 메뉴 인덱스
    selected_menu: usize,
    /// 배경색
    bg_color: Color,
    /// 전경색
    fg_color: Color,
    /// 강조색 (앱 이름)
    accent_color: Color,
    /// 선택된 메뉴 배경색
    selected_bg: Color,
    /// 선택된 메뉴 전경색
    selected_fg: Color,
}

impl<'a> Default for MenuBar<'a> {
    fn default() -> Self {
        Self {
            app_name: "복슬Dir",
            menus: &[],
            menu_active: false,
            selected_menu: 0,
            bg_color: Color::Rgb(30, 30, 30),
            fg_color: Color::Rgb(212, 212, 212),
            accent_color: Color::Rgb(0, 120, 212),
            selected_bg: Color::Rgb(0, 120, 212),
            selected_fg: Color::White,
        }
    }
}

impl<'a> MenuBar<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// 앱 이름 설정
    pub fn app_name(mut self, name: &'a str) -> Self {
        self.app_name = name;
        self
    }

    /// 메뉴 목록 설정
    pub fn menus(mut self, menus: &'a [Menu]) -> Self {
        self.menus = menus;
        self
    }

    /// 메뉴 활성화 상태 설정
    pub fn menu_active(mut self, active: bool) -> Self {
        self.menu_active = active;
        self
    }

    /// 선택된 메뉴 인덱스 설정
    pub fn selected_menu(mut self, index: usize) -> Self {
        self.selected_menu = index;
        self
    }

    /// 현재 경로 설정 (하위 호환성)
    pub fn current_path(self, _path: &'a str) -> Self {
        // 경로는 더 이상 메뉴바에 표시하지 않음 (패널 제목으로 이동)
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

    /// 강조색 설정
    pub fn accent_color(mut self, color: Color) -> Self {
        self.accent_color = color;
        self
    }

    /// 테마 적용
    pub fn theme(mut self, theme: &Theme) -> Self {
        self.bg_color = theme.menu_bar_bg.to_color();
        self.fg_color = theme.menu_bar_fg.to_color();
        self.accent_color = theme.accent.to_color();
        self.selected_bg = theme.accent.to_color();
        self.selected_fg = theme.menu_bar_fg.to_color();
        self
    }

    /// 메뉴 항목의 x 위치 계산
    pub fn get_menu_x_position(&self, menu_index: usize) -> u16 {
        // [앱이름] + 공백 - 한글 문자 너비 고려
        let app_name_text = format!("[{}] ", self.app_name);
        let mut x = app_name_text.width() as u16;

        for (i, menu) in self.menus.iter().enumerate() {
            // 메뉴 사이 추가 공백 (첫 메뉴 제외)
            if i > 0 {
                x += 1;
            }

            // 찾는 메뉴에 도달하면 위치 반환 (드롭다운 테두리가 하이라이트 시작 위치에 맞춰짐)
            if i == menu_index {
                break;
            }

            // 양쪽 공백(2) + 메뉴 제목 (한글 문자 너비 고려)
            x += menu.title.width() as u16 + 2;
        }

        x
    }
}

impl Widget for MenuBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 배경 채우기
        buf.set_style(area, Style::default().bg(self.bg_color));

        let mut spans = Vec::new();

        // 앱 이름 - 여백 최소화 (fg_color로 표시하여 가시성 확보)
        spans.push(Span::styled(
            format!("[{}] ", self.app_name),
            Style::default()
                .fg(self.fg_color)
                .add_modifier(Modifier::BOLD),
        ));

        // 메뉴 항목들
        for (i, menu) in self.menus.iter().enumerate() {
            let is_selected = self.menu_active && i == self.selected_menu;

            let style = if is_selected {
                Style::default().fg(self.selected_fg).bg(self.selected_bg)
            } else {
                Style::default().fg(self.fg_color)
            };

            // 메뉴 사이 여백 추가 (첫 메뉴 제외)
            if i > 0 {
                spans.push(Span::raw(" "));
            }

            // 양쪽 공백 동일하게 (안정감)
            spans.push(Span::styled(format!(" {} ", menu.title), style));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::components::dropdown_menu::create_default_menus;

    #[test]
    fn test_menu_bar_creation() {
        let menus = create_default_menus();
        let menu_bar = MenuBar::new()
            .app_name("TestApp")
            .menus(&menus)
            .menu_active(true)
            .selected_menu(0);

        assert_eq!(menu_bar.app_name, "TestApp");
        assert!(menu_bar.menu_active);
        assert_eq!(menu_bar.selected_menu, 0);
    }

    #[test]
    fn test_menu_x_position() {
        let menus = create_default_menus();
        let menu_bar = MenuBar::new().menus(&menus);

        // 첫 번째 메뉴 위치
        let x0 = menu_bar.get_menu_x_position(0);
        assert!(x0 > 0);

        // 두 번째 메뉴 위치는 첫 번째보다 커야 함
        let x1 = menu_bar.get_menu_x_position(1);
        assert!(x1 > x0);
    }
}
