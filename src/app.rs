#![allow(dead_code)]

use crate::ui::{create_default_menus, ActivePanel, LayoutManager, LayoutMode, Menu, MenuState, ThemeManager};
use crate::utils::error::Result;
use std::env;
use std::path::PathBuf;

/// 앱 상태
pub struct App {
    /// 종료 플래그
    pub should_quit: bool,
    /// 레이아웃 매니저
    pub layout: LayoutManager,
    /// 좌측 패널 경로
    pub left_path: PathBuf,
    /// 우측 패널 경로
    pub right_path: PathBuf,
    /// 메뉴 목록
    pub menus: Vec<Menu>,
    /// 메뉴 상태
    pub menu_state: MenuState,
    /// 테마 관리자
    pub theme_manager: ThemeManager,
}

impl App {
    pub fn new() -> Result<Self> {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        Ok(Self {
            should_quit: false,
            layout: LayoutManager::new(),
            left_path: current_dir.clone(),
            right_path: current_dir,
            menus: create_default_menus(),
            menu_state: MenuState::new(),
            theme_manager: ThemeManager::new(),
        })
    }

    /// 종료
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// 종료 상태 확인
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// 패널 전환 (Tab)
    pub fn toggle_panel(&mut self) {
        self.layout.toggle_panel();
    }

    /// 활성 패널 반환
    pub fn active_panel(&self) -> ActivePanel {
        self.layout.active_panel()
    }

    /// 현재 활성 패널의 경로 반환
    pub fn active_path(&self) -> &PathBuf {
        match self.layout.active_panel() {
            ActivePanel::Left => &self.left_path,
            ActivePanel::Right => &self.right_path,
        }
    }

    /// 레이아웃 모드 반환
    pub fn layout_mode(&self) -> LayoutMode {
        self.layout.mode()
    }

    /// 레이아웃 모드 문자열 반환
    pub fn layout_mode_str(&self) -> &str {
        match self.layout.mode() {
            LayoutMode::DualPanel => "DUAL",
            LayoutMode::SinglePanel => "SINGLE",
            LayoutMode::TooSmall => "WARN",
        }
    }

    // === 메뉴 관련 메서드 ===

    /// 메뉴 활성화 상태 확인
    pub fn is_menu_active(&self) -> bool {
        self.menu_state.is_open
    }

    /// 메뉴 토글
    pub fn toggle_menu(&mut self) {
        self.menu_state.toggle();
    }

    /// 메뉴 열기
    pub fn open_menu(&mut self) {
        self.menu_state.open();
    }

    /// 메뉴 닫기
    pub fn close_menu(&mut self) {
        self.menu_state.close();
    }

    /// 다음 메뉴로 이동
    pub fn next_menu(&mut self) {
        self.menu_state.next_menu(self.menus.len());
    }

    /// 이전 메뉴로 이동
    pub fn prev_menu(&mut self) {
        self.menu_state.prev_menu(self.menus.len());
    }

    /// 다음 항목으로 이동
    pub fn next_menu_item(&mut self) {
        if let Some(menu) = self.menus.get(self.menu_state.selected_menu) {
            self.menu_state.next_item(&menu.items);
        }
    }

    /// 이전 항목으로 이동
    pub fn prev_menu_item(&mut self) {
        if let Some(menu) = self.menus.get(self.menu_state.selected_menu) {
            self.menu_state.prev_item(&menu.items);
        }
    }

    /// 서브메뉴 열기
    pub fn open_submenu(&mut self) {
        if let Some(menu) = self.menus.get(self.menu_state.selected_menu) {
            self.menu_state.open_submenu(&menu.items);
        }
    }

    /// 서브메뉴 닫기
    pub fn close_submenu(&mut self) {
        self.menu_state.close_submenu();
    }

    /// 현재 선택된 메뉴 항목의 ID 반환
    pub fn get_selected_menu_action(&self) -> Option<String> {
        let menu = self.menus.get(self.menu_state.selected_menu)?;
        let item = menu.items.get(self.menu_state.selected_item)?;

        if self.menu_state.submenu_open && item.has_submenu() {
            let subitem = item.submenu.get(self.menu_state.selected_submenu_item)?;
            Some(subitem.id.clone())
        } else if !item.is_separator() && item.enabled {
            Some(item.id.clone())
        } else {
            None
        }
    }

    /// 메뉴 액션 실행
    pub fn execute_menu_action(&mut self, action_id: &str) {
        match action_id {
            // 종료
            "quit" => self.quit(),

            // 테마 전환
            "theme_dark" => {
                let _ = self.theme_manager.switch_theme("dark");
            }
            "theme_light" => {
                let _ = self.theme_manager.switch_theme("light");
            }
            "theme_contrast" => {
                let _ = self.theme_manager.switch_theme("high_contrast");
            }

            // TODO: 다른 액션들 추가
            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

        Self {
            should_quit: false,
            layout: LayoutManager::new(),
            left_path: current_dir.clone(),
            right_path: current_dir,
            menus: create_default_menus(),
            menu_state: MenuState::new(),
            theme_manager: ThemeManager::new(),
        }
    }
}
