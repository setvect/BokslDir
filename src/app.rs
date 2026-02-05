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

    // === 파일 탐색 관련 메서드 (Phase 2.3) ===

    /// 선택을 위로 이동
    pub fn move_selection_up(&mut self) {
        let panel = self.active_panel_state_mut();

        if panel.selected_index > 0 {
            panel.selected_index -= 1;
            self.adjust_scroll_offset();
        }
    }

    /// 선택을 아래로 이동
    pub fn move_selection_down(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();

        // 최대 인덱스 계산
        // ".."이 있을 때: 0 (부모) + entries.len() (파일들) = entries.len()
        // ".."이 없을 때: entries.len() - 1
        let max_index = if has_parent {
            panel.entries.len()
        } else {
            panel.entries.len().saturating_sub(1)
        };

        let panel_mut = self.active_panel_state_mut();
        if panel_mut.selected_index < max_index {
            panel_mut.selected_index += 1;
            self.adjust_scroll_offset();
        }
    }

    /// 페이지 위로 이동
    pub fn move_selection_page_up(&mut self) {
        let page_size = self.get_page_size();
        let panel = self.active_panel_state_mut();

        panel.selected_index = panel.selected_index.saturating_sub(page_size);
        self.adjust_scroll_offset();
    }

    /// 페이지 아래로 이동
    pub fn move_selection_page_down(&mut self) {
        let page_size = self.get_page_size();
        let max_index = self.get_max_index();

        let panel = self.active_panel_state_mut();
        panel.selected_index = (panel.selected_index + page_size).min(max_index);
        self.adjust_scroll_offset();
    }

    /// 최대 인덱스 계산
    fn get_max_index(&self) -> usize {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();

        if has_parent {
            panel.entries.len()
        } else {
            panel.entries.len().saturating_sub(1)
        }
    }

    /// 페이지 크기 계산 (화면에 표시되는 항목 수)
    fn get_page_size(&self) -> usize {
        let panel = self.active_panel_state();
        let (_, terminal_height) = self.layout.terminal_size();
        let panel_inner_height = terminal_height.saturating_sub(4);
        let available_height = panel_inner_height
            .saturating_sub(2)
            .saturating_sub(2)
            .saturating_sub(if panel.current_path.parent().is_some() {
                1
            } else {
                0
            });

        (available_height as usize).max(1)
    }

    /// 스크롤 오프셋을 현재 선택 위치에 맞게 조정
    fn adjust_scroll_offset(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected = panel.selected_index;
        let scroll = panel.scroll_offset;

        // ".."이 선택된 경우 스크롤을 0으로
        if has_parent && selected == 0 {
            let panel_mut = self.active_panel_state_mut();
            panel_mut.scroll_offset = 0;
            return;
        }

        // selected_index를 entries 인덱스로 변환
        // (selected_index는 ".." 포함, scroll_offset은 entries 배열 인덱스)
        let entries_selected = if has_parent {
            selected.saturating_sub(1)
        } else {
            selected
        };

        // 패널 렌더링 가능 높이 계산
        // terminal_height - menu_bar(1) - status_bar(1) - command_bar(1)
        // - panel_borders(2) - header(1) - separator(1) - parent(1 if shown)
        let (_, terminal_height) = self.layout.terminal_size();
        let panel_inner_height = terminal_height.saturating_sub(4); // 메뉴/상태/커맨드바 제외
        let available_height = panel_inner_height
            .saturating_sub(2) // 테두리
            .saturating_sub(2) // 헤더 + 구분선
            .saturating_sub(if has_parent { 1 } else { 0 }); // ".." 항목

        let panel_mut = self.active_panel_state_mut();

        // 선택이 화면 위쪽을 벗어나면 스크롤 위로
        if entries_selected < scroll {
            panel_mut.scroll_offset = entries_selected;
        }
        // 선택이 화면 아래쪽을 벗어나면 스크롤 아래로
        else if entries_selected >= scroll + available_height as usize {
            panel_mut.scroll_offset = entries_selected.saturating_sub(available_height as usize - 1);
        }
    }

    /// Enter 키 처리: 디렉토리 진입 또는 상위 디렉토리 이동
    pub fn enter_selected(&mut self) {
        let panel = self.active_panel_state();
        let current_path = panel.current_path.clone();
        let selected_index = panel.selected_index;
        let has_parent = current_path.parent().is_some();

        // Case 1: ".." 선택 시 상위 디렉토리로 이동
        if selected_index == 0 && has_parent {
            if let Some(parent) = current_path.parent() {
                let parent_path = parent.to_path_buf();
                // 현재 디렉토리 이름을 기억 (상위 이동 후 포커스용)
                let current_dir_name = current_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string());

                // filesystem과 panel을 동시에 참조하기 위해 match 사용
                match self.active_panel() {
                    ActivePanel::Left => {
                        let _ = self.left_panel.change_directory_and_focus(
                            parent_path,
                            current_dir_name.as_deref(),
                            &self.filesystem,
                        );
                    }
                    ActivePanel::Right => {
                        let _ = self.right_panel.change_directory_and_focus(
                            parent_path,
                            current_dir_name.as_deref(),
                            &self.filesystem,
                        );
                    }
                }
                // 에러 발생 시 무시 (Phase 3에서 에러 다이얼로그 구현 예정)
            }
            return;
        }

        // Case 2: 일반 항목 선택 시
        let entry_info = {
            let panel = self.active_panel_state();
            // show_parent가 true면 entries는 index 1부터 시작
            let entry_index = if has_parent {
                selected_index.saturating_sub(1)
            } else {
                selected_index
            };
            panel
                .entries
                .get(entry_index)
                .map(|e| (e.is_directory(), e.path.clone()))
        };

        if let Some((is_dir, path)) = entry_info {
            if is_dir {
                // 디렉토리면 진입
                match self.active_panel() {
                    ActivePanel::Left => {
                        let _ = self.left_panel.change_directory(path, &self.filesystem);
                    }
                    ActivePanel::Right => {
                        let _ = self.right_panel.change_directory(path, &self.filesystem);
                    }
                }
                // 에러 발생 시 무시
            }
            // 파일이면 아무것도 안 함 (Phase 6에서 파일 뷰어 구현 예정)
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
