#![allow(dead_code)]
// Dropdown menu component - 드롭다운 메뉴 컴포넌트
//
// 2단계 드롭다운 메뉴 시스템

use crate::core::actions::get_shortcut_display;
use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Widget},
};

/// 메뉴 항목 종류
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemKind {
    /// 일반 액션 항목
    Action,
    /// 서브메뉴가 있는 항목
    Submenu,
    /// 구분선
    Separator,
}

/// 메뉴 항목
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// 항목 ID
    pub id: String,
    /// 표시 레이블
    pub label: String,
    /// 단축키 표시
    pub shortcut: Option<String>,
    /// 항목 종류
    pub kind: MenuItemKind,
    /// 활성화 여부
    pub enabled: bool,
    /// 서브메뉴 항목들 (kind가 Submenu일 때)
    pub submenu: Vec<MenuItem>,
}

impl MenuItem {
    /// 액션 항목 생성
    pub fn action(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            kind: MenuItemKind::Action,
            enabled: true,
            submenu: Vec::new(),
        }
    }

    /// 단축키 설정
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// 서브메뉴 항목 생성
    pub fn submenu(id: impl Into<String>, label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            kind: MenuItemKind::Submenu,
            enabled: true,
            submenu: items,
        }
    }

    /// 구분선 생성
    pub fn separator() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            shortcut: None,
            kind: MenuItemKind::Separator,
            enabled: false,
            submenu: Vec::new(),
        }
    }

    /// 활성화 여부 설정
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 구분선인지 확인
    pub fn is_separator(&self) -> bool {
        self.kind == MenuItemKind::Separator
    }

    /// 서브메뉴가 있는지 확인
    pub fn has_submenu(&self) -> bool {
        self.kind == MenuItemKind::Submenu && !self.submenu.is_empty()
    }
}

/// 메뉴 (드롭다운 하나)
#[derive(Debug, Clone)]
pub struct Menu {
    /// 메뉴 ID
    pub id: String,
    /// 메뉴 제목
    pub title: String,
    /// 단축키 (Alt+?)
    pub hotkey: Option<char>,
    /// 메뉴 항목들
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            hotkey: None,
            items: Vec::new(),
        }
    }

    pub fn hotkey(mut self, key: char) -> Self {
        self.hotkey = Some(key);
        self
    }

    pub fn items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
        self
    }

    /// 메뉴 항목 수 (구분선 제외)
    pub fn item_count(&self) -> usize {
        self.items.iter().filter(|i| !i.is_separator()).count()
    }

    /// 전체 항목 수 (구분선 포함)
    pub fn total_items(&self) -> usize {
        self.items.len()
    }
}

/// 메뉴 상태
#[derive(Debug, Clone, Default)]
pub struct MenuState {
    /// 메뉴가 열려있는지
    pub is_open: bool,
    /// 현재 선택된 메뉴 인덱스
    pub selected_menu: usize,
    /// 현재 선택된 항목 인덱스
    pub selected_item: usize,
    /// 서브메뉴가 열려있는지
    pub submenu_open: bool,
    /// 서브메뉴에서 선택된 항목 인덱스
    pub selected_submenu_item: usize,
}

impl MenuState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 메뉴 열기
    pub fn open(&mut self) {
        self.is_open = true;
        self.selected_item = 0;
        self.submenu_open = false;
    }

    /// 메뉴 닫기
    pub fn close(&mut self) {
        self.is_open = false;
        self.submenu_open = false;
    }

    /// 메뉴 토글
    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// 다음 메뉴로 이동
    pub fn next_menu(&mut self, menu_count: usize) {
        if menu_count > 0 {
            self.selected_menu = (self.selected_menu + 1) % menu_count;
            self.selected_item = 0;
            self.submenu_open = false;
        }
    }

    /// 이전 메뉴로 이동
    pub fn prev_menu(&mut self, menu_count: usize) {
        if menu_count > 0 {
            self.selected_menu = if self.selected_menu == 0 {
                menu_count - 1
            } else {
                self.selected_menu - 1
            };
            self.selected_item = 0;
            self.submenu_open = false;
        }
    }

    /// 다음 항목으로 이동
    pub fn next_item(&mut self, items: &[MenuItem]) {
        if items.is_empty() {
            return;
        }

        if self.submenu_open {
            // 서브메뉴 내에서 이동
            if let Some(item) = items.get(self.selected_item) {
                if item.has_submenu() {
                    let submenu_len = item.submenu.len();
                    self.selected_submenu_item = (self.selected_submenu_item + 1) % submenu_len;
                    // 구분선 건너뛰기
                    while item
                        .submenu
                        .get(self.selected_submenu_item)
                        .is_some_and(MenuItem::is_separator)
                    {
                        self.selected_submenu_item = (self.selected_submenu_item + 1) % submenu_len;
                    }
                }
            }
        } else {
            // 메인 메뉴에서 이동
            let len = items.len();
            self.selected_item = (self.selected_item + 1) % len;
            // 구분선 건너뛰기
            while items
                .get(self.selected_item)
                .is_some_and(MenuItem::is_separator)
            {
                self.selected_item = (self.selected_item + 1) % len;
            }
        }
    }

    /// 이전 항목으로 이동
    pub fn prev_item(&mut self, items: &[MenuItem]) {
        if items.is_empty() {
            return;
        }

        if self.submenu_open {
            // 서브메뉴 내에서 이동
            if let Some(item) = items.get(self.selected_item) {
                if item.has_submenu() {
                    let submenu_len = item.submenu.len();
                    self.selected_submenu_item = if self.selected_submenu_item == 0 {
                        submenu_len - 1
                    } else {
                        self.selected_submenu_item - 1
                    };
                    // 구분선 건너뛰기
                    while item
                        .submenu
                        .get(self.selected_submenu_item)
                        .is_some_and(MenuItem::is_separator)
                    {
                        self.selected_submenu_item = if self.selected_submenu_item == 0 {
                            submenu_len - 1
                        } else {
                            self.selected_submenu_item - 1
                        };
                    }
                }
            }
        } else {
            // 메인 메뉴에서 이동
            let len = items.len();
            self.selected_item = if self.selected_item == 0 {
                len - 1
            } else {
                self.selected_item - 1
            };
            // 구분선 건너뛰기
            while items
                .get(self.selected_item)
                .is_some_and(MenuItem::is_separator)
            {
                self.selected_item = if self.selected_item == 0 {
                    len - 1
                } else {
                    self.selected_item - 1
                };
            }
        }
    }

    /// 서브메뉴 열기 (오른쪽 화살표)
    pub fn open_submenu(&mut self, items: &[MenuItem]) {
        if let Some(item) = items.get(self.selected_item) {
            if item.has_submenu() {
                self.submenu_open = true;
                self.selected_submenu_item = 0;
                // 첫 항목이 구분선이면 건너뛰기
                while item
                    .submenu
                    .get(self.selected_submenu_item)
                    .is_some_and(MenuItem::is_separator)
                {
                    self.selected_submenu_item += 1;
                }
            }
        }
    }

    /// 서브메뉴 닫기 (왼쪽 화살표)
    pub fn close_submenu(&mut self) {
        self.submenu_open = false;
    }
}

/// 드롭다운 메뉴 위젯
pub struct DropdownMenu<'a> {
    /// 메뉴 정의
    menu: &'a Menu,
    /// 메뉴 상태
    state: &'a MenuState,
    /// 배경색
    bg_color: Color,
    /// 전경색
    fg_color: Color,
    /// 선택 배경색
    selected_bg: Color,
    /// 선택 전경색
    selected_fg: Color,
    /// 비활성 색상
    disabled_color: Color,
    /// 테두리 색상
    border_color: Color,
    /// 단축키 색상
    shortcut_color: Color,
}

impl<'a> Default for DropdownMenu<'a> {
    fn default() -> Self {
        // dummy menu for default
        static EMPTY_MENU: Menu = Menu {
            id: String::new(),
            title: String::new(),
            hotkey: None,
            items: Vec::new(),
        };
        static EMPTY_STATE: MenuState = MenuState {
            is_open: false,
            selected_menu: 0,
            selected_item: 0,
            submenu_open: false,
            selected_submenu_item: 0,
        };
        Self {
            menu: &EMPTY_MENU,
            state: &EMPTY_STATE,
            bg_color: Color::Rgb(45, 45, 45),
            fg_color: Color::Rgb(212, 212, 212),
            selected_bg: Color::Rgb(0, 120, 212),
            selected_fg: Color::White,
            disabled_color: Color::Rgb(100, 100, 100),
            border_color: Color::Rgb(60, 60, 60),
            shortcut_color: Color::Rgb(150, 150, 150),
        }
    }
}

impl<'a> DropdownMenu<'a> {
    pub fn new(menu: &'a Menu, state: &'a MenuState) -> Self {
        Self {
            menu,
            state,
            ..Default::default()
        }
    }

    /// 테마 적용
    pub fn theme(mut self, theme: &Theme) -> Self {
        self.bg_color = theme.panel_bg.to_color();
        self.fg_color = theme.file_normal.to_color();
        self.selected_bg = theme.file_selected_bg.to_color();
        self.selected_fg = theme.file_selected.to_color();
        self.border_color = theme.panel_inactive_border.to_color();
        self
    }

    /// 메뉴의 너비 계산
    fn calculate_width(&self) -> u16 {
        let max_label = self
            .menu
            .items
            .iter()
            .map(|item| item.label.chars().count())
            .max()
            .unwrap_or(0);

        let max_shortcut = self
            .menu
            .items
            .iter()
            .filter_map(|item| item.shortcut.as_ref())
            .map(|s| s.chars().count())
            .max()
            .unwrap_or(0);

        // 레이블 + 간격 + 단축키 + 서브메뉴 화살표 + 패딩
        let width = max_label + 2 + max_shortcut + 2 + 2;
        (width as u16).max(15)
    }

    /// 서브메뉴의 너비 계산
    fn calculate_submenu_width(&self, submenu: &[MenuItem]) -> u16 {
        let max_label = submenu
            .iter()
            .map(|item| item.label.chars().count())
            .max()
            .unwrap_or(0);

        let max_shortcut = submenu
            .iter()
            .filter_map(|item| item.shortcut.as_ref())
            .map(|s| s.chars().count())
            .max()
            .unwrap_or(0);

        let width = max_label + 2 + max_shortcut + 2;
        (width as u16).max(12)
    }

    /// 메뉴 항목 렌더링
    fn render_item(
        &self,
        item: &MenuItem,
        is_selected: bool,
        width: u16,
        buf: &mut Buffer,
        area: Rect,
    ) {
        if item.is_separator() {
            // 구분선 렌더링
            let line = "─".repeat((width - 2) as usize);
            let span = Span::styled(&line, Style::default().fg(self.border_color));
            buf.set_span(area.x + 1, area.y, &span, width - 2);
            return;
        }

        let (bg, fg) = if is_selected && item.enabled {
            (self.selected_bg, self.selected_fg)
        } else if item.enabled {
            (self.bg_color, self.fg_color)
        } else {
            (self.bg_color, self.disabled_color)
        };

        // 배경 채우기
        for x in area.x..area.x + width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_bg(bg);
            }
        }

        // 레이블
        let label_style = Style::default().fg(fg).bg(bg);
        buf.set_span(
            area.x + 1,
            area.y,
            &Span::styled(&item.label, label_style),
            width - 2,
        );

        // 단축키 또는 서브메뉴 화살표
        let right_text = if item.has_submenu() {
            "▶".to_string()
        } else if let Some(ref shortcut) = item.shortcut {
            shortcut.clone()
        } else {
            String::new()
        };

        if !right_text.is_empty() {
            let right_style = if is_selected && item.enabled {
                Style::default().fg(self.selected_fg).bg(bg)
            } else {
                Style::default().fg(self.shortcut_color).bg(bg)
            };
            let right_x = area.x + width - right_text.chars().count() as u16 - 1;
            buf.set_span(
                right_x,
                area.y,
                &Span::styled(&right_text, right_style),
                right_text.chars().count() as u16,
            );
        }
    }
}

impl DropdownMenu<'_> {
    /// 메인 메뉴 항목 루프 렌더링
    fn render_main_items(&self, dropdown_area: Rect, buf: &mut Buffer) {
        for (i, item) in self.menu.items.iter().enumerate() {
            if i as u16 + 1 >= dropdown_area.height - 1 {
                break;
            }

            let item_area = Rect {
                x: dropdown_area.x,
                y: dropdown_area.y + 1 + i as u16,
                width: dropdown_area.width,
                height: 1,
            };

            let is_selected = i == self.state.selected_item;
            self.render_item(item, is_selected, dropdown_area.width, buf, item_area);
        }
    }

    /// 서브메뉴 프레임 + 항목 렌더링
    fn render_submenu(&self, dropdown_area: Rect, area: Rect, buf: &mut Buffer) {
        if !self.state.submenu_open {
            return;
        }
        let Some(item) = self.menu.items.get(self.state.selected_item) else {
            return;
        };
        if !item.has_submenu() {
            return;
        }

        let submenu_width = self.calculate_submenu_width(&item.submenu);
        let submenu_height = item.submenu.len() as u16 + 2;

        let submenu_x =
            (dropdown_area.x + dropdown_area.width).min(area.x + area.width - submenu_width);
        let submenu_y = dropdown_area.y + 1 + self.state.selected_item as u16;

        let submenu_area = Rect {
            x: submenu_x,
            y: submenu_y.min(area.y + area.height - submenu_height),
            width: submenu_width.min(area.width.saturating_sub(submenu_x - area.x)),
            height: submenu_height.min(area.height.saturating_sub(submenu_y - area.y)),
        };

        Clear.render(submenu_area, buf);

        let submenu_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        submenu_block.render(submenu_area, buf);

        for (j, subitem) in item.submenu.iter().enumerate() {
            if j as u16 + 1 >= submenu_area.height - 1 {
                break;
            }

            let subitem_area = Rect {
                x: submenu_area.x,
                y: submenu_area.y + 1 + j as u16,
                width: submenu_area.width,
                height: 1,
            };

            let is_selected = j == self.state.selected_submenu_item;
            self.render_item(subitem, is_selected, submenu_area.width, buf, subitem_area);
        }
    }
}

impl Widget for DropdownMenu<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.menu.items.is_empty() {
            return;
        }

        let width = self.calculate_width();
        let height = self.menu.items.len() as u16 + 2;

        let dropdown_area = Rect {
            x: area.x,
            y: area.y,
            width: width.min(area.width),
            height: height.min(area.height),
        };

        Clear.render(dropdown_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.border_color))
            .style(Style::default().bg(self.bg_color));
        block.render(dropdown_area, buf);

        self.render_main_items(dropdown_area, buf);
        self.render_submenu(dropdown_area, area, buf);
    }
}

/// 메뉴 항목 생성 헬퍼 (레지스트리에서 단축키 자동 조회)
fn menu_action(id: &str, label: &str) -> MenuItem {
    let mut item = MenuItem::action(id, label);
    if let Some(shortcut) = get_shortcut_display(id) {
        item = item.shortcut(shortcut);
    }
    item
}

/// 기본 메뉴 생성
pub fn create_default_menus() -> Vec<Menu> {
    vec![
        Menu::new("file", "파일(F)").hotkey('f').items(vec![
            menu_action("new_dir", "새 폴더"),
            MenuItem::separator(),
            menu_action("open_default", "기본 프로그램으로 열기"),
            MenuItem::separator(),
            menu_action("rename", "이름 변경"),
            menu_action("delete", "삭제"),
            menu_action("perm_delete", "영구 삭제"),
            MenuItem::separator(),
            menu_action("quit", "종료"),
        ]),
        Menu::new("edit", "편집(E)").hotkey('e').items(vec![
            menu_action("copy", "복사"),
            menu_action("move", "이동"),
            MenuItem::separator(),
            menu_action("select_all", "전체 선택"),
            menu_action("invert_selection", "선택 반전"),
            menu_action("deselect", "선택 해제"),
        ]),
        Menu::new("view", "보기(V)").hotkey('v').items(vec![
            menu_action("refresh", "새로고침"),
            menu_action("file_info", "파일 정보"),
            MenuItem::separator(),
            MenuItem::submenu(
                "sort_by",
                "정렬 기준",
                vec![
                    menu_action("sort_name", "이름"),
                    menu_action("sort_size", "크기"),
                    menu_action("sort_date", "수정 날짜"),
                    menu_action("sort_ext", "확장자"),
                ],
            ),
            MenuItem::submenu(
                "sort_order",
                "정렬 순서",
                vec![
                    menu_action("sort_asc", "오름차순"),
                    menu_action("sort_desc", "내림차순"),
                ],
            ),
            MenuItem::separator(),
            menu_action("filter_start", "필터링"),
            menu_action("filter_clear", "필터 해제"),
            MenuItem::separator(),
            menu_action("toggle_hidden", "숨김 파일 표시"),
            menu_action("mount_points", "마운트 포인트"),
            menu_action("goto_path", "경로로 이동"),
            menu_action("history_list", "디렉토리 히스토리"),
            menu_action("bookmark_list", "북마크"),
            MenuItem::submenu(
                "size_format",
                "크기 표시 형식",
                vec![
                    menu_action("size_auto", "자동 (KB/MB/GB)"),
                    menu_action("size_bytes", "바이트"),
                ],
            ),
        ]),
        Menu::new("settings", "설정(S)").hotkey('s').items(vec![
            MenuItem::submenu(
                "theme",
                "테마",
                vec![
                    menu_action("theme_dark", "Dark (기본)"),
                    menu_action("theme_light", "Light"),
                    menu_action("theme_contrast", "High Contrast"),
                ],
            ),
            MenuItem::separator(),
            menu_action("toggle_icons", "아이콘 전환"),
        ]),
        Menu::new("help", "도움말(H)").hotkey('h').items(vec![
            menu_action("help_keys", "단축키 도움말"),
            menu_action("about", "복슬Dir 정보"),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_item_creation() {
        let item = MenuItem::action("test", "Test Item").shortcut("Ctrl+T");
        assert_eq!(item.id, "test");
        assert_eq!(item.label, "Test Item");
        assert_eq!(item.shortcut, Some("Ctrl+T".to_string()));
        assert!(item.enabled);
    }

    #[test]
    fn test_menu_creation() {
        let menu = Menu::new("file", "파일(F)").hotkey('f').items(vec![
            MenuItem::action("new", "새 파일"),
            MenuItem::separator(),
            MenuItem::action("quit", "종료"),
        ]);

        assert_eq!(menu.id, "file");
        assert_eq!(menu.item_count(), 2); // 구분선 제외
        assert_eq!(menu.total_items(), 3); // 구분선 포함
    }

    #[test]
    fn test_menu_state() {
        let mut state = MenuState::new();
        assert!(!state.is_open);

        state.open();
        assert!(state.is_open);

        state.close();
        assert!(!state.is_open);
    }

    #[test]
    fn test_default_menus() {
        let menus = create_default_menus();
        assert_eq!(menus.len(), 5);
        assert_eq!(menus[0].title, "파일(F)");
    }
}
