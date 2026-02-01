#![allow(dead_code)]

use crate::models::PanelState;
use crate::system::FileSystem;
use crate::ui::{
    create_default_menus, ActivePanel, LayoutManager, LayoutMode, Menu, MenuState, ThemeManager,
};
use crate::utils::error::Result;
use std::env;

/// 앱 상태
pub struct App {
    /// 종료 플래그
    pub should_quit: bool,
    /// 레이아웃 매니저
    pub layout: LayoutManager,
    /// 좌측 패널 상태
    pub left_panel: PanelState,
    /// 우측 패널 상태
    pub right_panel: PanelState,
    /// 파일 시스템
    pub filesystem: FileSystem,
    /// 메뉴 목록
    pub menus: Vec<Menu>,
    /// 메뉴 상태
    pub menu_state: MenuState,
    /// 테마 관리자
    pub theme_manager: ThemeManager,
}

impl App {
    pub fn new() -> Result<Self> {
        let current_dir = env::current_dir().unwrap_or_else(|_| {
            #[cfg(unix)]
            {
                std::path::PathBuf::from("/")
            }
            #[cfg(windows)]
            {
                std::path::PathBuf::from("C:\\")
            }
            #[cfg(not(any(unix, windows)))]
            {
                std::path::PathBuf::from(".")
            }
        });

        let filesystem = FileSystem::new();

        // 패널 상태 초기화 및 파일 목록 로드
        let mut left_panel = PanelState::new(current_dir.clone());
        left_panel.refresh(&filesystem)?;

        let mut right_panel = PanelState::new(current_dir);
        right_panel.refresh(&filesystem)?;

        Ok(Self {
            should_quit: false,
            layout: LayoutManager::new(),
            left_panel,
            right_panel,
            filesystem,
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
    pub fn active_path(&self) -> &std::path::Path {
        match self.layout.active_panel() {
            ActivePanel::Left => &self.left_panel.current_path,
            ActivePanel::Right => &self.right_panel.current_path,
        }
    }

    /// 활성 패널 상태 반환
    pub fn active_panel_state(&self) -> &PanelState {
        match self.layout.active_panel() {
            ActivePanel::Left => &self.left_panel,
            ActivePanel::Right => &self.right_panel,
        }
    }

    /// 활성 패널 상태 반환 (mutable)
    pub fn active_panel_state_mut(&mut self) -> &mut PanelState {
        match self.layout.active_panel() {
            ActivePanel::Left => &mut self.left_panel,
            ActivePanel::Right => &mut self.right_panel,
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
        // Default는 에러를 무시하고 기본값 사용
        Self::new().unwrap_or_else(|_| {
            let current_dir = std::path::PathBuf::from(".");
            let filesystem = FileSystem::new();

            Self {
                should_quit: false,
                layout: LayoutManager::new(),
                left_panel: PanelState::new(current_dir.clone()),
                right_panel: PanelState::new(current_dir),
                filesystem,
                menus: create_default_menus(),
                menu_state: MenuState::new(),
                theme_manager: ThemeManager::new(),
            }
        })
    }
}
