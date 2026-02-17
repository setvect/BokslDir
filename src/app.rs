#![allow(dead_code)]

use crate::core::actions::Action;
use crate::models::operation::{
    ConflictResolution, FlattenedEntryKind, FlattenedFile, OperationState, OperationType,
    PendingOperation,
};
use crate::models::panel_state::{SortBy, SortOrder};
use crate::models::{PanelState, PanelTabs};
use crate::system::{FileSystem, ImeStatus};
use crate::ui::{
    create_default_menus, ActivePanel, DialogKind, InputPurpose, LayoutManager, LayoutMode, Menu,
    MenuState, ThemeManager,
};
use crate::utils::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedPanelHistory {
    entries: Vec<PathBuf>,
    index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedHistories {
    version: u32,
    left: PersistedPanelHistory,
    right: PersistedPanelHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedBookmark {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedBookmarks {
    version: u32,
    bookmarks: Vec<PersistedBookmark>,
}

#[derive(Debug, Clone)]
pub struct TerminalEditorRequest {
    pub editor_command: String,
    pub target_path: PathBuf,
}

/// 파일 크기 표시 형식
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeFormat {
    /// 자동 (B/KB/MB/GB)
    #[default]
    Auto,
    /// 정확한 바이트 (천 단위 콤마)
    Bytes,
}

/// 앱 상태
pub struct App {
    /// 종료 플래그
    pub should_quit: bool,
    /// 레이아웃 매니저
    pub layout: LayoutManager,
    /// 좌측 패널 탭 상태
    pub left_tabs: PanelTabs,
    /// 우측 패널 탭 상태
    pub right_tabs: PanelTabs,
    /// 파일 시스템
    pub filesystem: FileSystem,
    /// 메뉴 목록
    pub menus: Vec<Menu>,
    /// 메뉴 상태
    pub menu_state: MenuState,
    /// 테마 관리자
    pub theme_manager: ThemeManager,
    // Phase 3.2: 파일 복사/이동
    /// 현재 표시 중인 다이얼로그
    pub dialog: Option<DialogKind>,
    /// 대기 중인 파일 작업
    pub pending_operation: Option<PendingOperation>,
    // Phase 4: Vim 스타일 키 시퀀스
    /// 대기 중인 키 (예: 'g' for 'gg')
    pub pending_key: Option<char>,
    /// 대기 키 입력 시각
    pub pending_key_time: Option<Instant>,
    /// 토스트 메시지 (3초 후 자동 소멸)
    pub toast_message: Option<(String, Instant)>,
    /// 아이콘 표시 모드
    pub icon_mode: crate::ui::components::panel::IconMode,
    /// 파일 크기 표시 형식
    pub size_format: SizeFormat,
    /// 현재 IME 상태
    pub ime_status: ImeStatus,
    /// 기본 터미널 에디터 명령 (런타임 프리셋/환경변수 기반)
    default_terminal_editor: String,
    /// 메인 루프에서 처리할 터미널 에디터 실행 요청
    pending_terminal_editor_request: Option<TerminalEditorRequest>,
    /// 전역 북마크 목록
    bookmarks: Vec<PersistedBookmark>,
    /// 테스트에서 북마크 저장 경로를 격리하기 위한 override
    bookmarks_store_override: Option<PathBuf>,
}

impl App {
    const MAX_TABS_PER_PANEL: usize = 5;
    const HISTORY_STORE_VERSION: u32 = 1;
    const BOOKMARK_STORE_VERSION: u32 = 1;
    const FALLBACK_TERMINAL_EDITOR: &'static str = "vi";

    fn resolve_default_terminal_editor_from_env() -> String {
        for key in ["VISUAL", "EDITOR"] {
            if let Ok(value) = env::var(key) {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        Self::FALLBACK_TERMINAL_EDITOR.to_string()
    }

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

        let mut app = Self {
            should_quit: false,
            layout: LayoutManager::new(),
            left_tabs: PanelTabs::new(left_panel),
            right_tabs: PanelTabs::new(right_panel),
            filesystem,
            menus: create_default_menus(),
            menu_state: MenuState::new(),
            theme_manager: ThemeManager::new(),
            dialog: None,
            pending_operation: None,
            pending_key: None,
            pending_key_time: None,
            toast_message: None,
            icon_mode: crate::ui::components::panel::IconMode::default(),
            size_format: SizeFormat::default(),
            ime_status: crate::system::get_current_ime(),
            default_terminal_editor: Self::resolve_default_terminal_editor_from_env(),
            pending_terminal_editor_request: None,
            bookmarks: Vec::new(),
            bookmarks_store_override: None,
        };
        app.load_persisted_histories();
        app.load_persisted_bookmarks();
        Ok(app)
    }

    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_APP_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let current_dir = std::path::PathBuf::from(".");
        let suffix = TEST_APP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let bookmarks_store_override = std::env::temp_dir().join(format!(
            "boksldir-test-bookmarks-{}-{}.toml",
            std::process::id(),
            suffix
        ));

        Self {
            should_quit: false,
            layout: LayoutManager::new(),
            left_tabs: PanelTabs::new(PanelState::new(current_dir.clone())),
            right_tabs: PanelTabs::new(PanelState::new(current_dir)),
            filesystem: FileSystem::new(),
            menus: create_default_menus(),
            menu_state: MenuState::new(),
            theme_manager: ThemeManager::new(),
            dialog: None,
            pending_operation: None,
            pending_key: None,
            pending_key_time: None,
            toast_message: None,
            icon_mode: crate::ui::components::panel::IconMode::default(),
            size_format: SizeFormat::default(),
            ime_status: ImeStatus::Unknown,
            default_terminal_editor: Self::FALLBACK_TERMINAL_EDITOR.to_string(),
            pending_terminal_editor_request: None,
            bookmarks: Vec::new(),
            bookmarks_store_override: Some(bookmarks_store_override),
        }
    }

    /// 종료
    pub fn quit(&mut self) {
        let _ = self.save_persisted_histories();
        let _ = self.save_persisted_bookmarks();
        self.should_quit = true;
    }

    fn history_store_path() -> Option<PathBuf> {
        if let Ok(custom) = env::var("BOKSLDIR_HISTORY_FILE") {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".boksldir").join("history.toml"))
    }

    fn encode_histories(
        left_entries: &[PathBuf],
        left_index: usize,
        right_entries: &[PathBuf],
        right_index: usize,
    ) -> std::result::Result<String, toml::ser::Error> {
        let payload = PersistedHistories {
            version: Self::HISTORY_STORE_VERSION,
            left: PersistedPanelHistory {
                entries: left_entries.to_vec(),
                index: left_index,
            },
            right: PersistedPanelHistory {
                entries: right_entries.to_vec(),
                index: right_index,
            },
        };
        toml::to_string_pretty(&payload)
    }

    fn decode_histories(data: &str) -> Option<((Vec<PathBuf>, usize), (Vec<PathBuf>, usize))> {
        let parsed: PersistedHistories = toml::from_str(data).ok()?;
        if parsed.version != Self::HISTORY_STORE_VERSION {
            return None;
        }
        Some((
            (parsed.left.entries, parsed.left.index),
            (parsed.right.entries, parsed.right.index),
        ))
    }

    fn save_persisted_histories(&self) -> std::io::Result<()> {
        let Some(path) = Self::history_store_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let left = self.left_tabs.active();
        let right = self.right_tabs.active();
        let data = Self::encode_histories(
            &left.history_entries,
            left.history_index,
            &right.history_entries,
            right.history_index,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, data)
    }

    fn apply_loaded_history(
        &mut self,
        panel_kind: ActivePanel,
        entries: Vec<PathBuf>,
        index: usize,
    ) {
        let fallback = match panel_kind {
            ActivePanel::Left => self.left_tabs.active().current_path.clone(),
            ActivePanel::Right => self.right_tabs.active().current_path.clone(),
        };

        let mut valid_entries: Vec<PathBuf> = entries
            .into_iter()
            .filter(|p| p.exists() && p.is_dir())
            .collect();
        if valid_entries.is_empty() {
            valid_entries.push(fallback.clone());
        }

        let clamped_index = index.min(valid_entries.len().saturating_sub(1));
        let restore_path = valid_entries[clamped_index].clone();

        match panel_kind {
            ActivePanel::Left => {
                let panel = self.left_tabs.active_mut();
                panel.history_entries = valid_entries;
                panel.history_index = clamped_index;
                let _ = panel.change_directory(restore_path, &self.filesystem);
            }
            ActivePanel::Right => {
                let panel = self.right_tabs.active_mut();
                panel.history_entries = valid_entries;
                panel.history_index = clamped_index;
                let _ = panel.change_directory(restore_path, &self.filesystem);
            }
        }
    }

    fn load_persisted_histories(&mut self) {
        let Some(path) = Self::history_store_path() else {
            return;
        };
        if let Ok(data) = fs::read_to_string(&path) {
            if let Some((left, right)) = Self::decode_histories(&data) {
                self.apply_loaded_history(ActivePanel::Left, left.0, left.1);
                self.apply_loaded_history(ActivePanel::Right, right.0, right.1);
            }
        }
    }

    fn resolve_bookmarks_store_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.bookmarks_store_override {
            return Some(path.clone());
        }
        if let Ok(custom) = env::var("BOKSLDIR_BOOKMARKS_FILE") {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".boksldir").join("bookmarks.toml"))
    }

    fn encode_bookmarks(
        bookmarks: &[PersistedBookmark],
    ) -> std::result::Result<String, toml::ser::Error> {
        let payload = PersistedBookmarks {
            version: Self::BOOKMARK_STORE_VERSION,
            bookmarks: bookmarks.to_vec(),
        };
        toml::to_string_pretty(&payload)
    }

    fn decode_bookmarks(data: &str) -> Option<Vec<PersistedBookmark>> {
        let parsed: PersistedBookmarks = toml::from_str(data).ok()?;
        if parsed.version != Self::BOOKMARK_STORE_VERSION {
            return None;
        }
        Some(parsed.bookmarks)
    }

    fn save_persisted_bookmarks(&self) -> std::io::Result<()> {
        let Some(path) = self.resolve_bookmarks_store_path() else {
            return Ok(());
        };
        self.save_persisted_bookmarks_to_path(&path)
    }

    fn save_persisted_bookmarks_to_path(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = Self::encode_bookmarks(&self.bookmarks)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, data)
    }

    fn load_persisted_bookmarks(&mut self) {
        let Some(path) = self.resolve_bookmarks_store_path() else {
            return;
        };
        self.load_persisted_bookmarks_from_path(&path);
    }

    fn load_persisted_bookmarks_from_path(&mut self, path: &std::path::Path) {
        if let Ok(data) = fs::read_to_string(path) {
            if let Some(bookmarks) = Self::decode_bookmarks(&data) {
                self.bookmarks = bookmarks;
            }
        }
    }

    /// 종료 상태 확인
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// 패널 전환 (Tab)
    pub fn toggle_panel(&mut self) {
        self.layout.toggle_panel();
    }

    /// 활성 패널에 새 탭 생성
    pub fn new_tab_active_panel(&mut self) {
        match self.active_panel() {
            ActivePanel::Left => {
                if self.left_tabs.len() >= Self::MAX_TABS_PER_PANEL {
                    self.set_toast("Max 5 tabs per panel");
                    return;
                }
                let from = self.left_tabs.active().clone();
                let idx = self.left_tabs.create_tab(&from);
                self.set_toast(&format!("Tab created ({})", idx + 1));
            }
            ActivePanel::Right => {
                if self.right_tabs.len() >= Self::MAX_TABS_PER_PANEL {
                    self.set_toast("Max 5 tabs per panel");
                    return;
                }
                let from = self.right_tabs.active().clone();
                let idx = self.right_tabs.create_tab(&from);
                self.set_toast(&format!("Tab created ({})", idx + 1));
            }
        }
    }

    /// 활성 패널의 현재 탭 닫기
    pub fn close_tab_active_panel(&mut self) {
        let closed = match self.active_panel() {
            ActivePanel::Left => self.left_tabs.close_active_tab(),
            ActivePanel::Right => self.right_tabs.close_active_tab(),
        };

        if closed {
            self.set_toast("Tab closed");
        } else {
            self.set_toast("Cannot close last tab");
        }
    }

    /// 활성 패널의 이전 탭 전환
    pub fn prev_tab_active_panel(&mut self) {
        let idx = match self.active_panel() {
            ActivePanel::Left => {
                self.left_tabs.prev_tab();
                self.left_tabs.active_index()
            }
            ActivePanel::Right => {
                self.right_tabs.prev_tab();
                self.right_tabs.active_index()
            }
        };
        self.set_toast(&format!("Tab {}", idx + 1));
    }

    /// 활성 패널의 다음 탭 전환
    pub fn next_tab_active_panel(&mut self) {
        let idx = match self.active_panel() {
            ActivePanel::Left => {
                self.left_tabs.next_tab();
                self.left_tabs.active_index()
            }
            ActivePanel::Right => {
                self.right_tabs.next_tab();
                self.right_tabs.active_index()
            }
        };
        self.set_toast(&format!("Tab {}", idx + 1));
    }

    /// 활성 패널의 특정 탭(0-based) 전환
    pub fn switch_tab_active_panel(&mut self, index: usize) {
        let ok = match self.active_panel() {
            ActivePanel::Left => self.left_tabs.switch_to(index),
            ActivePanel::Right => self.right_tabs.switch_to(index),
        };

        if ok {
            self.set_toast(&format!("Tab {}", index + 1));
        } else {
            self.set_toast(&format!("No tab {}", index + 1));
        }
    }

    /// 활성 패널 반환
    pub fn active_panel(&self) -> ActivePanel {
        self.layout.active_panel()
    }

    /// 현재 활성 패널의 경로 반환
    pub fn active_path(&self) -> &std::path::Path {
        match self.layout.active_panel() {
            ActivePanel::Left => &self.left_tabs.active().current_path,
            ActivePanel::Right => &self.right_tabs.active().current_path,
        }
    }

    /// 좌측 활성 탭 상태 반환
    pub fn left_active_panel_state(&self) -> &PanelState {
        self.left_tabs.active()
    }

    /// 좌측 활성 탭 상태 반환 (mutable)
    pub fn left_active_panel_state_mut(&mut self) -> &mut PanelState {
        self.left_tabs.active_mut()
    }

    /// 우측 활성 탭 상태 반환
    pub fn right_active_panel_state(&self) -> &PanelState {
        self.right_tabs.active()
    }

    /// 우측 활성 탭 상태 반환 (mutable)
    pub fn right_active_panel_state_mut(&mut self) -> &mut PanelState {
        self.right_tabs.active_mut()
    }

    /// 활성 패널 상태 반환
    pub fn active_panel_state(&self) -> &PanelState {
        match self.layout.active_panel() {
            ActivePanel::Left => self.left_tabs.active(),
            ActivePanel::Right => self.right_tabs.active(),
        }
    }

    /// 활성 패널 상태 반환 (mutable)
    pub fn active_panel_state_mut(&mut self) -> &mut PanelState {
        match self.layout.active_panel() {
            ActivePanel::Left => self.left_tabs.active_mut(),
            ActivePanel::Right => self.right_tabs.active_mut(),
        }
    }

    /// 패널별 탭 타이틀 반환
    pub fn panel_tab_titles(&self, panel: ActivePanel) -> Vec<String> {
        match panel {
            ActivePanel::Left => self.left_tabs.titles(),
            ActivePanel::Right => self.right_tabs.titles(),
        }
    }

    /// 패널별 활성 탭 인덱스 반환
    pub fn panel_active_tab_index(&self, panel: ActivePanel) -> usize {
        match panel {
            ActivePanel::Left => self.left_tabs.active_index(),
            ActivePanel::Right => self.right_tabs.active_index(),
        }
    }

    /// 패널별 탭 개수 반환
    pub fn panel_tab_count(&self, panel: ActivePanel) -> usize {
        match panel {
            ActivePanel::Left => self.left_tabs.len(),
            ActivePanel::Right => self.right_tabs.len(),
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

    /// 액션 실행 (단일 진실 원천)
    pub fn execute_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit(),
            Action::TogglePanel => self.toggle_panel(),
            Action::MoveDown => self.move_selection_down(),
            Action::MoveUp => self.move_selection_up(),
            Action::GoToParent => self.go_to_parent(),
            Action::EnterSelected => self.enter_selected(),
            Action::GoToTop => self.go_to_top(),
            Action::GoToBottom => self.go_to_bottom(),
            Action::PageUp => self.move_selection_page_up(),
            Action::PageDown => self.move_selection_page_down(),
            Action::TabNew => self.new_tab_active_panel(),
            Action::TabClose => self.close_tab_active_panel(),
            Action::Copy => self.start_copy(),
            Action::Move => self.start_move(),
            Action::OpenDefaultApp => self.start_open_default_app(),
            Action::OpenTerminalEditor => self.start_open_terminal_editor(),
            Action::Delete => self.start_delete(),
            Action::PermanentDelete => self.start_permanent_delete(),
            Action::MakeDirectory => self.start_mkdir(),
            Action::Rename => self.start_rename(),
            Action::ShowProperties => self.show_properties(),
            Action::ToggleSelection => self.toggle_selection_and_move_down(),
            Action::InvertSelection => self.invert_selection(),
            Action::SelectAll => self.select_all(),
            Action::DeselectAll => self.deselect_all(),
            Action::ShowHelp => self.show_help(),
            Action::Refresh => self.refresh_current(),
            Action::OpenMenu => self.open_menu(),
            Action::ThemeDark => {
                let _ = self.theme_manager.switch_theme("dark");
            }
            Action::ThemeLight => {
                let _ = self.theme_manager.switch_theme("light");
            }
            Action::ThemeContrast => {
                let _ = self.theme_manager.switch_theme("high_contrast");
            }
            Action::ToggleIconMode => {
                use crate::ui::components::panel::IconMode;
                self.icon_mode = match self.icon_mode {
                    IconMode::Emoji => IconMode::Ascii,
                    IconMode::Ascii => IconMode::Emoji,
                };
            }
            Action::SetDefaultEditorVi => self.set_default_editor_vi(),
            Action::SetDefaultEditorVim => self.set_default_editor_vim(),
            Action::SetDefaultEditorNano => self.set_default_editor_nano(),
            Action::SetDefaultEditorEmacs => self.set_default_editor_emacs(),
            Action::SortByName => self.sort_active_panel(SortBy::Name),
            Action::SortBySize => self.sort_active_panel(SortBy::Size),
            Action::SortByDate => self.sort_active_panel(SortBy::Modified),
            Action::SortByExt => self.sort_active_panel(SortBy::Extension),
            Action::SortAscending => self.toggle_sort_order(),
            Action::SortDescending => {
                self.active_panel_state_mut()
                    .set_sort_order(SortOrder::Descending);
                self.re_sort_active_panel();
            }
            // Filter / Search (Phase 5.2)
            Action::StartFilter => self.start_filter(),
            Action::ClearFilter => self.clear_filter(),
            // View (Phase 5.3)
            Action::ToggleHidden => self.toggle_hidden(),
            Action::ShowMountPoints => self.show_mount_points(),
            Action::GoToPath => self.start_go_to_path(),
            Action::ShowTabList => self.show_tab_list(),
            Action::HistoryBack => self.history_back(),
            Action::HistoryForward => self.history_forward(),
            Action::ShowHistoryList => self.show_history_list(),
            Action::AddBookmark => self.add_bookmark_current_dir(),
            Action::ShowBookmarkList => self.show_bookmark_list(),
            Action::SizeFormatAuto => {
                self.size_format = SizeFormat::Auto;
                self.set_toast("Size format: Auto");
            }
            Action::SizeFormatBytes => {
                self.size_format = SizeFormat::Bytes;
                self.set_toast("Size format: Bytes");
            }
            // 미구현 액션은 무시
            _ => {}
        }
    }

    /// 메뉴 액션 실행 (action_id → Action 변환 후 위임)
    pub fn execute_menu_action(&mut self, action_id: &str) {
        if let Some(action) = Action::from_id(action_id) {
            self.execute_action(action);
        }
    }

    // === 정렬 관련 메서드 (Phase 5.1) ===

    /// 활성 패널 정렬 기준 변경 (같은 기준이면 순서 토글)
    fn sort_active_panel(&mut self, sort_by: SortBy) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();

        // 현재 포커스 파일명 저장
        let focused_name = {
            let entry_index = if has_parent {
                panel.selected_index.saturating_sub(1)
            } else {
                panel.selected_index
            };
            panel.entries.get(entry_index).map(|e| e.name.clone())
        };

        let panel = self.active_panel_state_mut();
        panel.set_sort(sort_by);
        panel.sort_entries();

        // 포커스 파일 위치 복원
        if let Some(name) = focused_name {
            let offset = if has_parent { 1 } else { 0 };
            if let Some(idx) = panel.entries.iter().position(|e| e.name == name) {
                panel.selected_index = idx + offset;
            }
        }

        // 다중 선택 초기화 (인덱스 무효화)
        panel.selected_items.clear();

        let indicator = panel.sort_indicator();
        self.set_toast(&indicator);
        self.adjust_scroll_offset();
    }

    /// 활성 패널 정렬 순서 토글
    fn toggle_sort_order(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();

        let focused_name = {
            let entry_index = if has_parent {
                panel.selected_index.saturating_sub(1)
            } else {
                panel.selected_index
            };
            panel.entries.get(entry_index).map(|e| e.name.clone())
        };

        let panel = self.active_panel_state_mut();
        panel.sort_order = match panel.sort_order {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        };
        panel.sort_entries();

        if let Some(name) = focused_name {
            let offset = if has_parent { 1 } else { 0 };
            if let Some(idx) = panel.entries.iter().position(|e| e.name == name) {
                panel.selected_index = idx + offset;
            }
        }

        panel.selected_items.clear();

        let indicator = panel.sort_indicator();
        self.set_toast(&indicator);
        self.adjust_scroll_offset();
    }

    /// 활성 패널 재정렬 (정렬 상태 변경 후 호출)
    fn re_sort_active_panel(&mut self) {
        let panel = self.active_panel_state_mut();
        panel.sort_entries();
        panel.selected_items.clear();

        let indicator = panel.sort_indicator();
        self.set_toast(&indicator);
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

    /// 맨 위로 이동 (Home / gg)
    pub fn go_to_top(&mut self) {
        let panel = self.active_panel_state_mut();
        panel.selected_index = 0;
        self.adjust_scroll_offset();
    }

    /// 맨 아래로 이동 (End / G)
    pub fn go_to_bottom(&mut self) {
        let max_index = self.get_max_index();
        let panel = self.active_panel_state_mut();
        panel.selected_index = max_index;
        self.adjust_scroll_offset();
    }

    /// 활성 패널 경로 변경 공통 처리
    ///
    /// `record_in_history`가 true이면 이동 성공 시 히스토리에 기록합니다.
    fn change_active_dir(
        &mut self,
        path: PathBuf,
        record_in_history: bool,
        focus_name: Option<&str>,
    ) -> bool {
        let path_for_history = path.clone();
        let result = match self.active_panel() {
            ActivePanel::Left => {
                if let Some(name) = focus_name {
                    self.left_tabs.active_mut().change_directory_and_focus(
                        path,
                        Some(name),
                        &self.filesystem,
                    )
                } else {
                    self.left_tabs
                        .active_mut()
                        .change_directory(path, &self.filesystem)
                }
            }
            ActivePanel::Right => {
                if let Some(name) = focus_name {
                    self.right_tabs.active_mut().change_directory_and_focus(
                        path,
                        Some(name),
                        &self.filesystem,
                    )
                } else {
                    self.right_tabs
                        .active_mut()
                        .change_directory(path, &self.filesystem)
                }
            }
        };

        if result.is_ok() && record_in_history {
            self.active_panel_state_mut()
                .record_history(path_for_history);
        }

        if result.is_ok() {
            let _ = self.save_persisted_histories();
        }

        result.is_ok()
    }

    /// 상위 디렉토리로 이동 (h / Left)
    pub fn go_to_parent(&mut self) {
        let panel = self.active_panel_state();
        let current_path = panel.current_path.clone();

        if let Some(parent) = current_path.parent() {
            let parent_path = parent.to_path_buf();
            let current_dir_name = current_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            let _ = self.change_active_dir(parent_path, true, current_dir_name.as_deref());
        }
    }

    /// 현재 패널 새로고침 (Ctrl+R)
    pub fn refresh_current(&mut self) {
        match self.active_panel() {
            ActivePanel::Left => {
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
    }

    /// 영구 삭제 시작 (D)
    pub fn start_permanent_delete(&mut self) {
        let sources = self.get_operation_sources();

        if sources.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Information",
                "No files selected for deletion.",
            ));
            return;
        }

        let items: Vec<String> = sources
            .iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if p.is_dir() {
                    format!("{}/", name)
                } else {
                    name
                }
            })
            .collect();

        let (total_bytes, total_files) = self
            .filesystem
            .calculate_total_size(&sources)
            .unwrap_or((0, 0));
        let total_size = format!(
            "{}, {}",
            crate::utils::formatter::pluralize(total_files, "file", "files"),
            crate::utils::formatter::format_file_size(total_bytes)
        );

        let mut pending = PendingOperation::new(OperationType::Delete, sources, PathBuf::new());
        pending.progress.total_bytes = total_bytes;
        pending.progress.total_files = total_files;
        self.pending_operation = Some(pending);

        // 영구 삭제 기본 선택 (selected_button: 1)
        self.dialog = Some(DialogKind::DeleteConfirm {
            items,
            total_size,
            selected_button: 1,
        });
    }

    /// 도움말 표시 (?)
    pub fn show_help(&mut self) {
        self.dialog = Some(DialogKind::help());
    }

    // === pending_key 시스템 (Phase 4) ===

    /// 대기 키 설정
    pub fn set_pending_key(&mut self, key: char) {
        self.pending_key = Some(key);
        self.pending_key_time = Some(Instant::now());
    }

    /// 대기 키 초기화
    pub fn clear_pending_key(&mut self) {
        self.pending_key = None;
        self.pending_key_time = None;
    }

    /// 대기 키 만료 여부 (800ms)
    pub fn is_pending_key_expired(&self) -> bool {
        self.pending_key_time
            .is_some_and(|t| t.elapsed().as_millis() > 800)
    }

    /// 대기 키 표시 문자열 (상태바용)
    pub fn pending_key_display(&self) -> Option<String> {
        self.pending_key.map(|k| format!("{}_", k))
    }

    /// 메시지 다이얼로그 표시
    pub fn show_message(&mut self, title: &str, message: &str) {
        self.dialog = Some(DialogKind::message(title, message));
    }

    fn format_user_error(action: &str, path: Option<&Path>, error: &str, hint: &str) -> String {
        let mut message = format!("{} failed.", action);
        if let Some(p) = path {
            message.push_str(&format!("\nPath: {}", p.display()));
        }
        message.push_str(&format!("\nReason: {}", error));
        if !hint.is_empty() {
            message.push_str(&format!("\nHint: {}", hint));
        }
        message
    }

    fn focus_active_entry_by_name(&mut self, name: &str) -> bool {
        let (idx_opt, offset) = {
            let panel = self.active_panel_state();
            let has_parent = panel.current_path.parent().is_some();
            let offset = if has_parent { 1 } else { 0 };
            (
                panel
                    .entries
                    .iter()
                    .position(|entry| entry.name.eq_ignore_ascii_case(name)),
                offset,
            )
        };
        if let Some(idx) = idx_opt {
            self.active_panel_state_mut().selected_index = idx + offset;
            self.adjust_scroll_offset();
            true
        } else {
            false
        }
    }

    /// 토스트 메시지 설정 (3초 후 자동 소멸)
    pub fn set_toast(&mut self, message: &str) {
        self.toast_message = Some((message.to_string(), Instant::now()));
    }

    /// 만료된 토스트 제거
    pub fn clear_expired_toast(&mut self) {
        if let Some((_, time)) = &self.toast_message {
            if time.elapsed().as_secs() >= 3 {
                self.toast_message = None;
            }
        }
    }

    /// 토스트 메시지 가져오기 (만료 안 된 경우만)
    pub fn toast_display(&self) -> Option<&str> {
        self.toast_message.as_ref().and_then(|(msg, time)| {
            if time.elapsed().as_secs() < 3 {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    /// 도움말 스크롤 아래로
    pub fn dialog_help_scroll_down(&mut self) {
        if let Some(DialogKind::Help { scroll_offset, .. }) = &mut self.dialog {
            *scroll_offset += 1;
        }
    }

    /// 도움말 스크롤 위로
    pub fn dialog_help_scroll_up(&mut self) {
        if let Some(DialogKind::Help { scroll_offset, .. }) = &mut self.dialog {
            *scroll_offset = scroll_offset.saturating_sub(1);
        }
    }

    pub fn dialog_help_start_search(&mut self) {
        if let Some(DialogKind::Help { search_mode, .. }) = &mut self.dialog {
            *search_mode = true;
        }
    }

    pub fn dialog_help_end_search(&mut self) {
        if let Some(DialogKind::Help { search_mode, .. }) = &mut self.dialog {
            *search_mode = false;
        }
    }

    pub fn dialog_help_clear_or_close(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            search_mode,
            scroll_offset,
        }) = &mut self.dialog
        {
            if !search_query.is_empty() || *search_mode {
                search_query.clear();
                *search_cursor = 0;
                *search_mode = false;
                *scroll_offset = 0;
            } else {
                self.close_dialog();
            }
        }
    }

    pub fn dialog_help_input_char(&mut self, c: char) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            search_query.insert(*search_cursor, c);
            *search_cursor += c.len_utf8();
            *scroll_offset = 0;
        }
    }

    pub fn dialog_help_backspace(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            if *search_cursor > 0 {
                let prev = search_query[..*search_cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                search_query.remove(prev);
                *search_cursor = prev;
                *scroll_offset = 0;
            }
        }
    }

    pub fn dialog_help_delete_prev_word(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            Self::delete_prev_word(search_query, search_cursor);
            *scroll_offset = 0;
        }
    }

    pub fn dialog_help_delete(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            scroll_offset,
            ..
        }) = &mut self.dialog
        {
            if *search_cursor < search_query.len() {
                search_query.remove(*search_cursor);
                *scroll_offset = 0;
            }
        }
    }

    pub fn dialog_help_cursor_left(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            ..
        }) = &mut self.dialog
        {
            if *search_cursor > 0 {
                *search_cursor = search_query[..*search_cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
    }

    pub fn dialog_help_cursor_right(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            ..
        }) = &mut self.dialog
        {
            if *search_cursor < search_query.len() {
                *search_cursor = search_query[*search_cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *search_cursor + i)
                    .unwrap_or(search_query.len());
            }
        }
    }

    pub fn dialog_help_cursor_home(&mut self) {
        if let Some(DialogKind::Help { search_cursor, .. }) = &mut self.dialog {
            *search_cursor = 0;
        }
    }

    pub fn dialog_help_cursor_end(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            ..
        }) = &mut self.dialog
        {
            *search_cursor = search_query.len();
        }
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
            panel_mut.scroll_offset =
                entries_selected.saturating_sub(available_height as usize - 1);
        }
    }

    /// ".." 선택 시 상위 디렉토리로 이동 + 포커스 복원
    fn navigate_to_parent(&mut self, current_path: &std::path::Path) {
        if let Some(parent) = current_path.parent() {
            let parent_path = parent.to_path_buf();
            let current_dir_name = current_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            let _ = self.change_active_dir(parent_path, true, current_dir_name.as_deref());
        }
    }

    /// 디렉토리 항목 진입
    fn enter_directory(&mut self, path: PathBuf) {
        let _ = self.change_active_dir(path, true, None);
    }

    /// Enter 키 처리: 디렉토리 진입 또는 상위 디렉토리 이동
    pub fn enter_selected(&mut self) {
        let panel = self.active_panel_state();
        let current_path = panel.current_path.clone();
        let selected_index = panel.selected_index;
        let has_parent = current_path.parent().is_some();

        if selected_index == 0 && has_parent {
            self.navigate_to_parent(&current_path);
            return;
        }

        let entry_info = {
            let panel = self.active_panel_state();
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

        if let Some((true, path)) = entry_info {
            self.enter_directory(path);
        }
    }

    // === 다중 선택 관련 메서드 (Phase 3.1) ===

    /// 현재 항목 선택 토글 + 커서 아래로 이동
    ///
    /// Space 키 동작: ".." 항목은 선택 불가
    pub fn toggle_selection_and_move_down(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        // ".." 항목(index 0)이면 선택하지 않음
        if has_parent && selected_index == 0 {
            // 그래도 커서는 아래로 이동
            self.move_selection_down();
            return;
        }

        // selected_index를 entries 인덱스로 변환
        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        // 선택 토글
        let panel_mut = self.active_panel_state_mut();
        panel_mut.toggle_selection(entry_index);

        // 커서 아래로 이동
        self.move_selection_down();
    }

    /// 전체 선택 (Ctrl+A)
    pub fn select_all(&mut self) {
        let panel_mut = self.active_panel_state_mut();
        panel_mut.select_all();
    }

    /// 선택 반전 (*)
    pub fn invert_selection(&mut self) {
        let panel_mut = self.active_panel_state_mut();
        panel_mut.invert_selection();
    }

    /// 전체 해제 (Ctrl+D)
    pub fn deselect_all(&mut self) {
        let panel_mut = self.active_panel_state_mut();
        panel_mut.deselect_all();
    }

    // === 파일 복사/이동 관련 메서드 (Phase 3.2) ===

    /// 비활성 패널 상태 반환
    pub fn inactive_panel_state(&self) -> &PanelState {
        match self.layout.active_panel() {
            ActivePanel::Left => self.right_tabs.active(),
            ActivePanel::Right => self.left_tabs.active(),
        }
    }

    /// 재귀 복사/이동 검사 (복수 소스)
    ///
    /// 디렉토리를 자기 자신 내부로 복사/이동하려는 경우 에러 메시지 반환
    fn check_recursive_operation(
        sources: &[PathBuf],
        operation_type: OperationType,
        dest_path: &std::path::Path,
    ) -> Option<String> {
        for source in sources {
            if !Self::is_recursive_path(source, dest_path) {
                continue;
            }
            let name = source.file_name().unwrap_or_default().to_string_lossy();
            return Some(format!(
                "Cannot {} '{}' into itself.\n\
                 The destination is inside the source directory.",
                operation_type.name().to_lowercase(),
                name
            ));
        }
        None
    }

    /// 재귀 경로 검사 (정적 메서드)
    fn is_recursive_path(source: &std::path::Path, dest: &std::path::Path) -> bool {
        if !source.is_dir() {
            return false;
        }
        let Ok(canonical_source) = source.canonicalize() else {
            return false;
        };
        let Ok(canonical_dest) = dest.canonicalize() else {
            return false;
        };
        canonical_dest.starts_with(&canonical_source)
    }

    /// 작업 대상 파일 목록 가져오기
    ///
    /// 선택된 항목이 있으면 선택된 항목들, 없으면 커서 위치의 항목
    pub fn get_operation_sources(&self) -> Vec<PathBuf> {
        let panel = self.active_panel_state();

        // 선택된 항목이 있으면 그것들 반환
        if !panel.selected_items.is_empty() {
            return panel
                .selected_entries()
                .iter()
                .map(|e| e.path.clone())
                .collect();
        }

        // 선택된 항목이 없으면 커서 위치의 항목 반환
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        // ".." 항목이면 빈 벡터 반환
        if has_parent && selected_index == 0 {
            return Vec::new();
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        panel
            .entries
            .get(entry_index)
            .map(|e| vec![e.path.clone()])
            .unwrap_or_default()
    }

    /// 다이얼로그 활성 여부
    pub fn is_dialog_active(&self) -> bool {
        self.dialog.is_some()
    }

    /// 다이얼로그 닫기
    pub fn close_dialog(&mut self) {
        self.dialog = None;
        self.pending_operation = None;
    }

    /// 진행 중인 작업 취소
    pub fn cancel_operation(&mut self) {
        if let Some(pending) = self.pending_operation.take() {
            // 패널 새로고침 (일부 복사된 파일 반영)
            self.refresh_both_panels();

            // 취소 토스트 표시
            self.dialog = None;
            self.set_toast(&format!(
                "{} cancelled ({}/{})",
                pending.operation_type.name(),
                pending.progress.files_completed,
                pending.progress.total_files
            ));
        } else {
            self.close_dialog();
        }
    }

    /// 복사 시작 (y)
    pub fn start_copy(&mut self) {
        self.start_file_operation(OperationType::Copy);
    }

    /// 이동 시작 (x)
    pub fn start_move(&mut self) {
        self.start_file_operation(OperationType::Move);
    }

    /// 경로 직접 이동 시작 (gp)
    pub fn start_go_to_path(&mut self) {
        let base_path = self.active_panel_state().current_path.clone();
        let initial = base_path.to_string_lossy().to_string();
        self.dialog = Some(DialogKind::go_to_path_input(initial, base_path));
        self.update_input_completion_state();
    }

    /// 파일 작업 시작 (공통)
    fn start_file_operation(&mut self, operation_type: OperationType) {
        let sources = self.get_operation_sources();

        // 작업할 파일이 없으면 종료
        if sources.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Information",
                "No files selected for operation.",
            ));
            return;
        }

        // 반대 패널의 경로를 기본 대상으로
        let dest_dir = self.inactive_panel_state().current_path.clone();
        let dest_path = dest_dir.to_string_lossy().to_string();

        // 대기 작업 저장
        self.pending_operation = Some(PendingOperation::new(
            operation_type,
            sources,
            dest_dir.clone(),
        ));

        // 입력 다이얼로그 표시
        let title = operation_type.name();
        let prompt = format!("{} to:", title);
        self.dialog = Some(DialogKind::operation_path_input(
            title, prompt, dest_path, dest_dir,
        ));
        self.update_input_completion_state();
    }

    fn home_dir() -> Option<PathBuf> {
        env::var_os("HOME").map(PathBuf::from)
    }

    fn split_path_input(value: &str) -> (&str, &str) {
        if let Some((idx, _)) = value
            .char_indices()
            .rev()
            .find(|(_, c)| std::path::is_separator(*c))
        {
            (&value[..=idx], &value[idx + 1..])
        } else {
            ("", value)
        }
    }

    fn input_parent_context(&self, value: &str, base_path: &Path) -> (PathBuf, String, String) {
        if value == "~" {
            if let Some(home) = Self::home_dir() {
                return (
                    home,
                    format!("~{}", std::path::MAIN_SEPARATOR),
                    String::new(),
                );
            }
        }

        let (raw_parent, raw_partial) = Self::split_path_input(value);
        if raw_parent.is_empty() {
            (
                base_path.to_path_buf(),
                String::new(),
                raw_partial.to_string(),
            )
        } else {
            (
                self.resolve_input_path(raw_parent, base_path),
                raw_parent.to_string(),
                raw_partial.to_string(),
            )
        }
    }

    fn first_segment_candidate(
        path: &Path,
        parent_path: &Path,
        display_prefix: &str,
        partial: &str,
    ) -> Option<String> {
        if !path.is_dir() {
            return None;
        }
        let rest = path.strip_prefix(parent_path).ok()?;
        let first_segment = rest.components().find_map(|c| match c {
            Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })?;
        if !first_segment.starts_with(partial) {
            return None;
        }
        Some(format!("{}{}", display_prefix, first_segment))
    }

    fn history_completion_candidates(&self, value: &str, base_path: &Path) -> Vec<String> {
        let (parent_path, display_prefix, partial) = self.input_parent_context(value, base_path);
        self.active_panel_state()
            .history_entries
            .iter()
            .rev()
            .filter_map(|path| {
                Self::first_segment_candidate(path, &parent_path, &display_prefix, &partial)
            })
            .collect()
    }

    fn filesystem_completion_candidates(&self, value: &str, base_path: &Path) -> Vec<String> {
        let (dir_path, display_prefix, partial) = self.input_parent_context(value, base_path);

        let mut candidates: Vec<String> = fs::read_dir(dir_path)
            .ok()
            .into_iter()
            .flat_map(|iter| iter.flatten())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_dir() {
                    return None;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if !partial.is_empty() && !name.starts_with(&partial) {
                    return None;
                }
                Some(format!("{}{}", display_prefix, name))
            })
            .collect();
        candidates.sort_unstable();
        candidates
    }

    fn collect_input_completion_candidates(&self, value: &str, base_path: &Path) -> Vec<String> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        for candidate in self
            .history_completion_candidates(value, base_path)
            .into_iter()
            .chain(self.filesystem_completion_candidates(value, base_path))
        {
            if seen.insert(candidate.clone()) {
                candidates.push(candidate);
            }
        }

        candidates
    }

    fn selected_input_completion(&self) -> Option<String> {
        match &self.dialog {
            Some(DialogKind::Input {
                completion_candidates,
                completion_index: Some(idx),
                ..
            }) => completion_candidates.get(*idx).cloned(),
            _ => None,
        }
    }

    fn update_input_completion_state(&mut self) {
        let (value, cursor_pos, base_path) = match &self.dialog {
            Some(DialogKind::Input {
                value,
                cursor_pos,
                base_path,
                ..
            }) => (value.clone(), *cursor_pos, base_path.clone()),
            _ => return,
        };

        let completion_candidates = self.collect_input_completion_candidates(&value, &base_path);
        let completion_index = if completion_candidates.is_empty() {
            None
        } else {
            Some(0)
        };
        let selected_completion = completion_index.and_then(|idx| completion_candidates.get(idx));
        let ghost_suffix = if cursor_pos == value.len() {
            selected_completion
                .and_then(|candidate| candidate.strip_prefix(&value))
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };

        if let Some(DialogKind::Input {
            completion_candidates: candidates,
            completion_index: selected_idx,
            ghost_suffix: ghost,
            ..
        }) = &mut self.dialog
        {
            *candidates = completion_candidates;
            *selected_idx = completion_index;
            *ghost = ghost_suffix;
        }
    }

    /// 경로 입력 다이얼로그: 현재 선택 추천 적용
    pub fn dialog_input_apply_selected_completion(&mut self) {
        let Some(candidate) = self.selected_input_completion() else {
            return;
        };

        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *value != candidate {
                *value = candidate;
                *cursor_pos = value.len();
            }
        }
        self.update_input_completion_state();
    }

    /// 경로 입력 다이얼로그: 다음 추천으로 순환 + 즉시 적용
    pub fn dialog_input_cycle_completion_next(&mut self) {
        let needs_seed = matches!(
            &self.dialog,
            Some(DialogKind::Input {
                completion_candidates,
                ..
            }) if completion_candidates.is_empty()
        );
        if needs_seed {
            self.update_input_completion_state();
        }

        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            value,
            cursor_pos,
            ghost_suffix,
            ..
        }) = &mut self.dialog
        {
            if completion_candidates.is_empty() {
                return;
            }
            let next = completion_index
                .map(|idx| (idx + 1) % completion_candidates.len())
                .unwrap_or(0);
            *completion_index = Some(next);
            *value = completion_candidates[next].clone();
            *cursor_pos = value.len();
            *ghost_suffix = String::new();
        }
    }

    /// 경로 입력 다이얼로그: 이전 추천으로 순환 + 즉시 적용
    pub fn dialog_input_cycle_completion_prev(&mut self) {
        let needs_seed = matches!(
            &self.dialog,
            Some(DialogKind::Input {
                completion_candidates,
                ..
            }) if completion_candidates.is_empty()
        );
        if needs_seed {
            self.update_input_completion_state();
        }

        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            value,
            cursor_pos,
            ghost_suffix,
            ..
        }) = &mut self.dialog
        {
            if completion_candidates.is_empty() {
                return;
            }
            let prev = completion_index
                .map(|idx| {
                    if idx == 0 {
                        completion_candidates.len() - 1
                    } else {
                        idx - 1
                    }
                })
                .unwrap_or(0);
            *completion_index = Some(prev);
            *value = completion_candidates[prev].clone();
            *cursor_pos = value.len();
            *ghost_suffix = String::new();
        }
    }

    fn resolve_input_path(&self, input: &str, base_path: &Path) -> PathBuf {
        let expanded = if input == "~" {
            Self::home_dir().unwrap_or_else(|| PathBuf::from(input))
        } else if let Some(rest) = input.strip_prefix("~/") {
            if let Some(home) = Self::home_dir() {
                home.join(rest)
            } else {
                PathBuf::from(input)
            }
        } else if let Some(rest) = input.strip_prefix("~\\") {
            if let Some(home) = Self::home_dir() {
                home.join(rest)
            } else {
                PathBuf::from(input)
            }
        } else {
            PathBuf::from(input)
        };

        if expanded.is_absolute() {
            expanded
        } else {
            base_path.join(expanded)
        }
    }

    /// 대상 경로 검증 (존재/디렉토리/재귀 검사). 실패 시 에러 메시지 반환.
    fn validate_operation_destination(
        sources: &[PathBuf],
        operation_type: OperationType,
        dest_path: &std::path::Path,
        dest_path_str: &str,
    ) -> std::result::Result<(), String> {
        if !dest_path.exists() {
            return Err(format!(
                "Destination path does not exist:\n{}",
                dest_path_str
            ));
        }
        if !dest_path.is_dir() {
            return Err(format!(
                "Destination is not a directory:\n{}",
                dest_path_str
            ));
        }
        if let Some(error_msg) = Self::check_recursive_operation(sources, operation_type, dest_path)
        {
            return Err(error_msg);
        }
        Ok(())
    }

    /// 소스 평탄화 + 크기 계산 + processing 시작
    fn prepare_and_start_operation(
        &mut self,
        pending: &mut PendingOperation,
        dest_path: &std::path::Path,
    ) {
        let flattened: Vec<FlattenedFile> =
            match self.filesystem.flatten_sources(&pending.sources, dest_path) {
                Ok(files) => files,
                Err(e) => {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        format!("Failed to scan files: {}", e),
                    ));
                    return;
                }
            };

        // 디렉토리는 size=0, 파일/링크는 size 누적
        let total_bytes: u64 = flattened.iter().map(|f| f.size).sum();
        let total_files = flattened.len();

        if pending.operation_type == OperationType::Move {
            pending.set_move_cleanup_dirs(self.filesystem.collect_move_cleanup_dirs(&flattened));
        } else {
            pending.set_move_cleanup_dirs(Vec::new());
        }

        pending.set_flattened_files(flattened);
        pending.start_processing(total_bytes, total_files);
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
    }

    fn remove_existing_path(path: &std::path::Path) {
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }

    /// 단일 파일/디렉토리 엔트리 처리 + 결과 기록
    fn execute_single_file_operation(
        &self,
        pending: &mut PendingOperation,
        file_entry: &FlattenedFile,
        file_name: &str,
    ) {
        if let Some(parent) = file_entry.dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let result = match file_entry.entry_kind {
            FlattenedEntryKind::Directory => std::fs::create_dir_all(&file_entry.dest)
                .map(|_| 0)
                .map_err(crate::utils::error::BokslDirError::Io),
            FlattenedEntryKind::File | FlattenedEntryKind::SymlinkFile => {
                match pending.operation_type {
                    OperationType::Copy => self
                        .filesystem
                        .copy_file(&file_entry.source, &file_entry.dest),
                    OperationType::Move => self
                        .filesystem
                        .move_file(&file_entry.source, &file_entry.dest),
                    OperationType::Delete => unreachable!("Delete uses process_next_delete"),
                }
            }
            FlattenedEntryKind::SymlinkDirectory => {
                let message = "Directory symlink is not supported for copy/move";
                match pending.operation_type {
                    OperationType::Copy => Err(crate::utils::error::BokslDirError::CopyFailed {
                        src: file_entry.source.clone(),
                        dest: file_entry.dest.clone(),
                        reason: message.to_string(),
                    }),
                    OperationType::Move => Err(crate::utils::error::BokslDirError::MoveFailed {
                        src: file_entry.source.clone(),
                        dest: file_entry.dest.clone(),
                        reason: message.to_string(),
                    }),
                    OperationType::Delete => unreachable!("Delete uses process_next_delete"),
                }
            }
        };

        match result {
            Ok(bytes) => pending.files_completed(bytes, 1),
            Err(e) => {
                pending.add_error(format!("{}: {}", file_name, e));
                pending.mark_item_failed();
                pending.file_skipped();
            }
        }

        pending.current_index += 1;
    }

    fn resolve_conflict(
        &mut self,
        pending: &mut PendingOperation,
        source: &std::path::Path,
        dest_path: &std::path::Path,
    ) -> bool {
        let skip_all = pending
            .conflict_resolution
            .is_some_and(|r| r == ConflictResolution::SkipAll);
        let overwrite_all = pending
            .conflict_resolution
            .is_some_and(|r| r == ConflictResolution::OverwriteAll);

        if skip_all {
            pending.file_skipped();
            pending.current_index += 1;
            return false;
        }
        if !overwrite_all {
            pending.state = OperationState::WaitingConflict;
            self.dialog = Some(DialogKind::conflict(
                source.to_path_buf(),
                dest_path.to_path_buf(),
            ));
            return false;
        }
        // overwrite_all이면 기존 경로를 삭제
        Self::remove_existing_path(dest_path);
        true
    }

    fn should_resolve_conflict(file_entry: &FlattenedFile) -> bool {
        match file_entry.entry_kind {
            FlattenedEntryKind::Directory => file_entry.dest.exists() && !file_entry.dest.is_dir(),
            FlattenedEntryKind::File
            | FlattenedEntryKind::SymlinkFile
            | FlattenedEntryKind::SymlinkDirectory => file_entry.dest.exists(),
        }
    }

    fn cleanup_moved_directories(&self, pending: &mut PendingOperation) {
        if pending.operation_type != OperationType::Move {
            return;
        }

        for dir in pending.move_cleanup_dirs.clone() {
            if let Err(e) = std::fs::remove_dir(&dir) {
                use std::io::ErrorKind;
                if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty) {
                    continue;
                }
                pending.add_error(format!(
                    "Failed to cleanup source directory {}: {}",
                    dir.display(),
                    e
                ));
            }
        }
    }

    /// 다음 파일 처리 (메인 루프에서 호출)
    pub fn process_next_file(&mut self) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        if pending.state != OperationState::Processing {
            self.pending_operation = Some(pending);
            return;
        }

        if pending.is_all_processed() {
            self.finish_operation(pending);
            return;
        }

        let file_entry = pending.flattened_files[pending.current_index].clone();
        let source = file_entry.source.clone();
        let dest_path = file_entry.dest.clone();

        let file_name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        pending.set_current_file(&file_name);
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));

        if file_entry.entry_kind != FlattenedEntryKind::Directory && source == dest_path {
            pending.add_error(format!("Source and destination are the same: {:?}", source));
            pending.mark_item_failed();
            pending.file_skipped();
            pending.current_index += 1;
            self.pending_operation = Some(pending);
            return;
        }

        if Self::should_resolve_conflict(&file_entry)
            && !self.resolve_conflict(&mut pending, &source, &dest_path)
        {
            self.pending_operation = Some(pending);
            return;
        }

        self.execute_single_file_operation(&mut pending, &file_entry, &file_name);

        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
        self.pending_operation = Some(pending);
    }

    /// 입력 다이얼로그에서 확인 처리
    pub fn confirm_input_dialog(&mut self, dest_path_str: String) {
        let Some(DialogKind::Input {
            purpose, base_path, ..
        }) = &self.dialog
        else {
            self.close_dialog();
            return;
        };

        let purpose = *purpose;
        let base_path = base_path.clone();
        let resolved_path = self.resolve_input_path(&dest_path_str, &base_path);
        let resolved_path_str = resolved_path.to_string_lossy().to_string();

        match purpose {
            InputPurpose::OperationDestination => {
                let Some(mut pending) = self.pending_operation.take() else {
                    self.close_dialog();
                    return;
                };

                if let Err(error_msg) = Self::validate_operation_destination(
                    &pending.sources,
                    pending.operation_type,
                    &resolved_path,
                    &resolved_path_str,
                ) {
                    self.dialog = Some(DialogKind::error("Error", error_msg));
                    self.pending_operation = Some(pending);
                    return;
                }

                pending.dest_dir = resolved_path.clone();
                self.prepare_and_start_operation(&mut pending, &resolved_path);
                self.pending_operation = Some(pending);
            }
            InputPurpose::GoToPath => {
                if !resolved_path.exists() {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        format!("Destination path does not exist:\n{}", resolved_path_str),
                    ));
                    return;
                }
                if !resolved_path.is_dir() {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        format!("Destination is not a directory:\n{}", resolved_path_str),
                    ));
                    return;
                }

                if self.change_active_dir(resolved_path, true, None) {
                    self.close_dialog();
                } else {
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        format!("Failed to open path:\n{}", resolved_path_str),
                    ));
                }
            }
        }
    }

    /// 진행 중인 작업 여부 확인
    pub fn is_operation_processing(&self) -> bool {
        self.pending_operation
            .as_ref()
            .is_some_and(|p| p.state == OperationState::Processing)
    }

    /// 작업 완료 처리
    fn finish_operation(&mut self, mut pending: PendingOperation) {
        self.cleanup_moved_directories(&mut pending);

        // 패널 새로고침
        self.refresh_both_panels();

        // 결과 표시
        if pending.errors.is_empty() {
            self.close_dialog();
            self.set_toast(&format!(
                "{} completed: {}",
                pending.operation_type.name(),
                crate::utils::formatter::pluralize(pending.completed_count, "file", "files")
            ));
        } else {
            let preview: Vec<String> = pending.errors.iter().take(5).cloned().collect();
            let detail = if pending.errors.len() > 5 {
                format!(
                    "{}\n... and {} more errors",
                    preview.join("\n"),
                    pending.errors.len() - 5
                )
            } else {
                preview.join("\n")
            };
            let error_msg = format!(
                "{} completed with errors.\nSucceeded: {}\nFailed: {}\n\n{}",
                pending.operation_type.name(),
                pending.completed_count,
                pending.errors.len(),
                detail
            );
            self.dialog = Some(DialogKind::error("Error", error_msg));
        }

        // 선택 상태 초기화
        self.active_panel_state_mut().deselect_all();
    }

    /// 파일 작업 실행 (레거시 호환용 - 충돌 해결 후 재개)
    pub fn execute_file_operation(&mut self) {
        if let Some(pending) = self.pending_operation.as_mut() {
            pending.state = OperationState::Processing;
        }
    }

    /// 대상 파일/디렉토리 삭제 (Overwrite/OverwriteAll 공용)
    fn remove_existing_dest(&self) {
        if let Some(DialogKind::Conflict { dest_path, .. }) = &self.dialog {
            let dest = dest_path.clone();
            if dest.is_dir() {
                let _ = std::fs::remove_dir_all(&dest);
            } else {
                let _ = std::fs::remove_file(&dest);
            }
        }
    }

    /// 현재 파일 건너뛰기 + 인덱스 증가 (Skip/SkipAll 공용)
    fn skip_current_file(&mut self) {
        if let Some(pending) = self.pending_operation.as_mut() {
            pending.file_skipped();
            pending.current_index += 1;
        }
    }

    /// 충돌 해결 처리
    pub fn handle_conflict(&mut self, resolution: ConflictResolution) {
        match resolution {
            ConflictResolution::Cancel => {
                if let Some(pending) = self.pending_operation.take() {
                    self.finish_operation(pending);
                } else {
                    self.close_dialog();
                }
            }
            ConflictResolution::Overwrite => {
                self.remove_existing_dest();
                self.execute_file_operation();
            }
            ConflictResolution::Skip => {
                self.skip_current_file();
                self.execute_file_operation();
            }
            ConflictResolution::OverwriteAll => {
                self.remove_existing_dest();
                if let Some(pending) = self.pending_operation.as_mut() {
                    pending.conflict_resolution = Some(ConflictResolution::OverwriteAll);
                }
                self.execute_file_operation();
            }
            ConflictResolution::SkipAll => {
                self.skip_current_file();
                if let Some(pending) = self.pending_operation.as_mut() {
                    pending.conflict_resolution = Some(ConflictResolution::SkipAll);
                }
                self.execute_file_operation();
            }
        }
    }

    // === 파일 삭제 관련 메서드 (Phase 3.3) ===

    /// 삭제 시작 (d)
    pub fn start_delete(&mut self) {
        let sources = self.get_operation_sources();

        if sources.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Information",
                "No files selected for deletion.",
            ));
            return;
        }

        // 파일명 목록 생성
        let items: Vec<String> = sources
            .iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if p.is_dir() {
                    format!("{}/", name)
                } else {
                    name
                }
            })
            .collect();

        // 총 크기 계산
        let (total_bytes, total_files) = self
            .filesystem
            .calculate_total_size(&sources)
            .unwrap_or((0, 0));
        let total_size = format!(
            "{}, {}",
            crate::utils::formatter::pluralize(total_files, "file", "files"),
            crate::utils::formatter::format_file_size(total_bytes)
        );

        // 대기 작업 저장
        let mut pending = PendingOperation::new(OperationType::Delete, sources, PathBuf::new());
        pending.progress.total_bytes = total_bytes;
        pending.progress.total_files = total_files;
        self.pending_operation = Some(pending);

        // 삭제 확인 다이얼로그 표시
        self.dialog = Some(DialogKind::delete_confirm(items, total_size));
    }

    /// 삭제 확인 처리
    pub fn confirm_delete(&mut self, use_trash: bool) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        if use_trash {
            // 휴지통으로 이동: 한 번에 처리
            match self.filesystem.trash_items(&pending.sources) {
                Ok(()) => {
                    self.refresh_both_panels();
                    self.active_panel_state_mut().deselect_all();
                    self.dialog = None;
                    self.set_toast(&format!(
                        "Moved {} to trash.",
                        crate::utils::formatter::pluralize(pending.sources.len(), "item", "items")
                    ));
                }
                Err(e) => {
                    self.refresh_both_panels();
                    self.dialog = Some(DialogKind::error(
                        "Error",
                        Self::format_user_error(
                            "Move to trash",
                            pending.sources.first().map(|p| p.as_path()),
                            &e.to_string(),
                            "Check permissions and available disk space.",
                        ),
                    ));
                }
            }
        } else {
            // 영구 삭제: Progress 다이얼로그 표시 + Processing 시작
            let total_bytes = pending.progress.total_bytes;
            let total_files = pending.sources.len();
            pending.start_processing(total_bytes, total_files);
            self.dialog = Some(DialogKind::progress(pending.progress.clone()));
            self.pending_operation = Some(pending);
        }
    }

    /// 파일/디렉토리 삭제 실행 + 결과 기록
    fn execute_single_delete(
        &self,
        pending: &mut PendingOperation,
        source: &std::path::Path,
        file_name: &str,
    ) {
        let result = if source.is_dir() {
            self.filesystem.delete_directory(source)
        } else {
            self.filesystem.delete_file(source)
        };

        match result {
            Ok(bytes) => pending.files_completed(bytes, 1),
            Err(e) => {
                pending.add_error(format!("{}: {}", file_name, e));
                pending.mark_item_failed();
                pending.file_skipped();
            }
        }

        pending.current_index += 1;
    }

    /// 다음 삭제 항목 처리 (메인 루프에서 호출)
    pub fn process_next_delete(&mut self) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        if pending.state != OperationState::Processing {
            self.pending_operation = Some(pending);
            return;
        }

        if pending.current_index >= pending.sources.len() {
            self.finish_operation(pending);
            return;
        }

        let source = pending.sources[pending.current_index].clone();
        let file_name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        pending.set_current_file(&file_name);
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));

        self.execute_single_delete(&mut pending, &source, &file_name);

        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
        self.pending_operation = Some(pending);
    }

    /// Delete 작업 여부 확인
    pub fn is_delete_operation(&self) -> bool {
        self.pending_operation
            .as_ref()
            .is_some_and(|p| p.operation_type == OperationType::Delete)
    }

    // === DeleteConfirm 다이얼로그 입력 처리 ===

    /// 삭제 확인 다이얼로그: 버튼 이동 (다음)
    pub fn dialog_delete_confirm_next(&mut self) {
        if let Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = (*selected_button + 1) % 3;
        }
    }

    /// 삭제 확인 다이얼로그: 버튼 이동 (이전)
    pub fn dialog_delete_confirm_prev(&mut self) {
        if let Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 {
                2
            } else {
                *selected_button - 1
            };
        }
    }

    /// 삭제 확인 다이얼로그: 선택된 버튼 반환
    pub fn get_delete_confirm_button(&self) -> Option<usize> {
        if let Some(DialogKind::DeleteConfirm {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    // === Phase 3.4: 기타 파일 작업 ===

    /// 새 디렉토리 생성 시작 (a)
    pub fn start_mkdir(&mut self) {
        let parent_path = self.active_panel_state().current_path.clone();
        self.dialog = Some(DialogKind::mkdir_input(parent_path));
    }

    /// 새 디렉토리 생성 확인
    pub fn confirm_mkdir(&mut self, dir_name: String, parent_path: PathBuf) {
        let dir_name = dir_name.trim().to_string();

        if dir_name.is_empty() {
            self.dialog = Some(DialogKind::error(
                "Error",
                "Create directory failed.\nReason: Name cannot be empty.\nHint: Enter at least one character.",
            ));
            return;
        }

        let new_path = parent_path.join(&dir_name);

        match self.filesystem.create_directory(&new_path) {
            Ok(()) => {
                self.refresh_both_panels();
                self.focus_active_entry_by_name(&dir_name);
                self.dialog = None;
                self.set_toast(&format!("Directory '{}' created.", dir_name));
            }
            Err(e) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Create directory",
                        Some(&new_path),
                        &e.to_string(),
                        "Use a valid name and check write permission.",
                    ),
                ));
            }
        }
    }

    /// 이름 변경 시작 (r)
    pub fn start_rename(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        // ".." 선택 시 무시
        if has_parent && selected_index == 0 {
            return;
        }

        // 커서 위치의 항목 이름 변경
        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        if let Some(entry) = panel.entries.get(entry_index) {
            let original_path = entry.path.clone();
            let current_name = entry.name.clone();
            self.dialog = Some(DialogKind::rename_input(original_path, current_name));
        }
    }

    fn focused_open_target(&self) -> std::result::Result<PathBuf, String> {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        if has_parent && selected_index == 0 {
            return Err("Cannot open parent entry ('..').".to_string());
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        let Some(entry) = panel.entries.get(entry_index) else {
            return Err("No file selected.".to_string());
        };

        if entry.is_directory() || entry.path.is_dir() {
            return Err("Only files can be opened in Phase 7.1.".to_string());
        }

        Ok(entry.path.clone())
    }

    fn apply_open_default_app_result(
        &mut self,
        target_path: &Path,
        result: crate::utils::error::Result<()>,
    ) {
        match result {
            Ok(()) => {
                let display_name = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| target_path.to_string_lossy().to_string());
                self.set_toast(&format!("Opened: {}", display_name));
            }
            Err(e) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open with default app",
                        Some(target_path),
                        &e.to_string(),
                        "Check file path and OS application association.",
                    ),
                ));
            }
        }
    }

    /// 기본 연결 앱으로 파일 열기 (o)
    pub fn start_open_default_app(&mut self) {
        let target_path = match self.focused_open_target() {
            Ok(path) => path,
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open with default app",
                        None,
                        &reason,
                        "Select a regular file and try again.",
                    ),
                ));
                return;
            }
        };

        let result = self.filesystem.open_with_default_app(&target_path);
        self.apply_open_default_app_result(&target_path, result);
    }

    fn focused_terminal_editor_target(&self) -> std::result::Result<PathBuf, String> {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        if has_parent && selected_index == 0 {
            return Err("Cannot edit parent entry ('..').".to_string());
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        let Some(entry) = panel.entries.get(entry_index) else {
            return Err("No file selected.".to_string());
        };

        if entry.is_directory() || entry.path.is_dir() {
            return Err("Only files can be edited in Phase 7.2.".to_string());
        }

        Ok(entry.path.clone())
    }

    /// 터미널 에디터로 파일 열기 (e) - 실행 자체는 main 루프에서 처리
    pub fn start_open_terminal_editor(&mut self) {
        let target_path = match self.focused_terminal_editor_target() {
            Ok(path) => path,
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open in terminal editor",
                        None,
                        &reason,
                        "Select a regular file and try again.",
                    ),
                ));
                return;
            }
        };

        self.pending_terminal_editor_request = Some(TerminalEditorRequest {
            editor_command: self.default_terminal_editor.clone(),
            target_path,
        });
    }

    pub fn take_pending_terminal_editor_request(&mut self) -> Option<TerminalEditorRequest> {
        self.pending_terminal_editor_request.take()
    }

    pub fn apply_terminal_editor_result(
        &mut self,
        request: &TerminalEditorRequest,
        result: std::result::Result<(), String>,
    ) {
        match result {
            Ok(()) => {
                let display_name = request
                    .target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| request.target_path.to_string_lossy().to_string());
                self.set_toast(&format!("Edited: {}", display_name));
            }
            Err(reason) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Open in terminal editor",
                        Some(&request.target_path),
                        &reason,
                        "Check editor command and file path.",
                    ),
                ));
            }
        }
    }

    fn set_default_terminal_editor(&mut self, editor: &str) {
        self.default_terminal_editor = editor.to_string();
        self.set_toast(&format!("Default editor: {}", editor));
    }

    pub fn set_default_editor_vi(&mut self) {
        self.set_default_terminal_editor("vi");
    }

    pub fn set_default_editor_vim(&mut self) {
        self.set_default_terminal_editor("vim");
    }

    pub fn set_default_editor_nano(&mut self) {
        self.set_default_terminal_editor("nano");
    }

    pub fn set_default_editor_emacs(&mut self) {
        self.set_default_terminal_editor("emacs");
    }

    /// 이름 변경 확인
    pub fn confirm_rename(&mut self, new_name: String, original_path: PathBuf) {
        let new_name = new_name.trim().to_string();

        if new_name.is_empty() {
            self.dialog = Some(DialogKind::error(
                "Error",
                "Rename failed.\nReason: Name cannot be empty.\nHint: Enter at least one character.",
            ));
            return;
        }

        let new_path = original_path
            .parent()
            .map(|p| p.join(&new_name))
            .unwrap_or_else(|| PathBuf::from(&new_name));

        match self.filesystem.rename_path(&original_path, &new_path) {
            Ok(()) => {
                self.refresh_both_panels();
                self.focus_active_entry_by_name(&new_name);
                self.dialog = None;
                self.set_toast("Rename completed");
            }
            Err(e) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    Self::format_user_error(
                        "Rename",
                        Some(&original_path),
                        &e.to_string(),
                        "Check duplicate names and write permission.",
                    ),
                ));
            }
        }
    }

    /// 디렉토리/파일 크기 문자열 생성
    fn format_size_display(&self, entry: &crate::models::file_entry::FileEntry) -> String {
        if entry.is_directory() {
            match self
                .filesystem
                .calculate_total_size(std::slice::from_ref(&entry.path))
            {
                Ok((bytes, files)) => format!(
                    "{} ({} bytes, {})",
                    crate::utils::formatter::format_file_size(bytes),
                    crate::utils::formatter::format_number_with_commas(bytes),
                    crate::utils::formatter::pluralize(files, "file", "files")
                ),
                Err(_) => "Unknown".to_string(),
            }
        } else {
            format!(
                "{} ({} bytes)",
                crate::utils::formatter::format_file_size(entry.size),
                crate::utils::formatter::format_number_with_commas(entry.size)
            )
        }
    }

    /// 하위 항목 개수 문자열 생성
    fn format_children_info(&self, entry: &crate::models::file_entry::FileEntry) -> Option<String> {
        if !entry.is_directory() {
            return None;
        }
        match self.filesystem.read_directory(&entry.path) {
            Ok(entries) => {
                let dirs = entries.iter().filter(|e| e.is_directory()).count();
                let files = entries.len() - dirs;
                Some(format!(
                    "{}, {}",
                    crate::utils::formatter::pluralize(files, "file", "files"),
                    crate::utils::formatter::pluralize(dirs, "dir", "dirs")
                ))
            }
            Err(_) => None,
        }
    }

    /// 파일 속성 보기 (Alt+Enter)
    pub fn show_properties(&mut self) {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();
        let selected_index = panel.selected_index;

        if has_parent && selected_index == 0 {
            return;
        }

        let entry_index = if has_parent {
            selected_index.saturating_sub(1)
        } else {
            selected_index
        };

        if let Some(entry) = panel.entries.get(entry_index).cloned() {
            let file_type_str = match entry.file_type {
                crate::models::file_entry::FileType::Directory => "Directory",
                crate::models::file_entry::FileType::File => "File",
                crate::models::file_entry::FileType::Symlink => "Symbolic Link",
                crate::models::file_entry::FileType::Executable => "Executable",
            };

            let size_str = self.format_size_display(&entry);
            let modified_str = crate::utils::formatter::format_date_full(entry.modified);
            let permissions_str =
                crate::utils::formatter::format_permissions(entry.permissions.as_ref());
            let children_info = self.format_children_info(&entry);

            self.dialog = Some(DialogKind::properties(
                &entry.name,
                entry.path.to_string_lossy(),
                file_type_str,
                &size_str,
                &modified_str,
                &permissions_str,
                children_info,
            ));
        }
    }

    // === MkdirInput 다이얼로그 입력 처리 ===

    pub fn dialog_mkdir_input_char(&mut self, c: char) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            value.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
        }
    }

    pub fn dialog_mkdir_input_backspace(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                // 이전 문자의 바이트 시작 위치 찾기
                let prev_char_boundary = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(prev_char_boundary);
                *cursor_pos = prev_char_boundary;
            }
        }
    }

    pub fn dialog_mkdir_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            Self::delete_prev_word(value, cursor_pos);
        }
    }

    pub fn dialog_mkdir_input_delete(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                value.remove(*cursor_pos);
            }
        }
    }

    pub fn dialog_mkdir_input_left(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                *cursor_pos = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
    }

    pub fn dialog_mkdir_input_right(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                *cursor_pos = value[*cursor_pos..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor_pos + i)
                    .unwrap_or(value.len());
            }
        }
    }

    pub fn dialog_mkdir_input_home(&mut self) {
        if let Some(DialogKind::MkdirInput { cursor_pos, .. }) = &mut self.dialog {
            *cursor_pos = 0;
        }
    }

    pub fn dialog_mkdir_input_end(&mut self) {
        if let Some(DialogKind::MkdirInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            *cursor_pos = value.len();
        }
    }

    pub fn dialog_mkdir_toggle_button(&mut self) {
        if let Some(DialogKind::MkdirInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_mkdir_input_value(&self) -> Option<(String, PathBuf)> {
        if let Some(DialogKind::MkdirInput {
            value, parent_path, ..
        }) = &self.dialog
        {
            Some((value.clone(), parent_path.clone()))
        } else {
            None
        }
    }

    pub fn get_mkdir_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::MkdirInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    // === RenameInput 다이얼로그 입력 처리 ===

    pub fn dialog_rename_input_char(&mut self, c: char) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            value.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
        }
    }

    pub fn dialog_rename_input_backspace(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                let prev_char_boundary = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(prev_char_boundary);
                *cursor_pos = prev_char_boundary;
            }
        }
    }

    pub fn dialog_rename_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            Self::delete_prev_word(value, cursor_pos);
        }
    }

    pub fn dialog_rename_input_delete(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                value.remove(*cursor_pos);
            }
        }
    }

    pub fn dialog_rename_input_left(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                *cursor_pos = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
    }

    pub fn dialog_rename_input_right(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                *cursor_pos = value[*cursor_pos..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor_pos + i)
                    .unwrap_or(value.len());
            }
        }
    }

    pub fn dialog_rename_input_home(&mut self) {
        if let Some(DialogKind::RenameInput { cursor_pos, .. }) = &mut self.dialog {
            *cursor_pos = 0;
        }
    }

    pub fn dialog_rename_input_end(&mut self) {
        if let Some(DialogKind::RenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            *cursor_pos = value.len();
        }
    }

    pub fn dialog_rename_toggle_button(&mut self) {
        if let Some(DialogKind::RenameInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_rename_input_value(&self) -> Option<(String, PathBuf)> {
        if let Some(DialogKind::RenameInput {
            value,
            original_path,
            ..
        }) = &self.dialog
        {
            Some((value.clone(), original_path.clone()))
        } else {
            None
        }
    }

    pub fn get_rename_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::RenameInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    pub fn dialog_bookmark_rename_input_char(&mut self, c: char) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            value.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
        }
    }

    pub fn dialog_bookmark_rename_input_backspace(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                let prev = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(prev);
                *cursor_pos = prev;
            }
        }
    }

    pub fn dialog_bookmark_rename_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            Self::delete_prev_word(value, cursor_pos);
        }
    }

    pub fn dialog_bookmark_rename_input_delete(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                value.remove(*cursor_pos);
            }
        }
    }

    pub fn dialog_bookmark_rename_input_left(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                *cursor_pos = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
    }

    pub fn dialog_bookmark_rename_input_right(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                *cursor_pos = value[*cursor_pos..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor_pos + i)
                    .unwrap_or(value.len());
            }
        }
    }

    pub fn dialog_bookmark_rename_input_home(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput { cursor_pos, .. }) = &mut self.dialog {
            *cursor_pos = 0;
        }
    }

    pub fn dialog_bookmark_rename_input_end(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            *cursor_pos = value.len();
        }
    }

    pub fn dialog_bookmark_rename_toggle_button(&mut self) {
        if let Some(DialogKind::BookmarkRenameInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_bookmark_rename_input_value(&self) -> Option<(String, usize)> {
        if let Some(DialogKind::BookmarkRenameInput {
            value,
            bookmark_index,
            ..
        }) = &self.dialog
        {
            Some((value.clone(), *bookmark_index))
        } else {
            None
        }
    }

    pub fn get_bookmark_rename_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::BookmarkRenameInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    // === 숨김 파일 토글 (Phase 5.3) ===

    /// 숨김 파일 표시/숨김 토글 (양쪽 패널 동시)
    pub fn toggle_hidden(&mut self) {
        let new_val = !self.left_active_panel_state().show_hidden;
        self.left_active_panel_state_mut().show_hidden = new_val;
        self.right_active_panel_state_mut().show_hidden = new_val;
        let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
        let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
        self.set_toast(if new_val {
            "Hidden files shown"
        } else {
            "Hidden files hidden"
        });
    }

    /// 마운트 포인트 다이얼로그 표시
    pub fn show_mount_points(&mut self) {
        let points = self.filesystem.list_mount_points();
        let items: Vec<(String, std::path::PathBuf)> =
            points.into_iter().map(|mp| (mp.name, mp.path)).collect();
        if items.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Mount Points",
                "No mount points found.",
            ));
        } else {
            self.dialog = Some(DialogKind::mount_points(items));
        }
    }

    /// 활성 패널 탭 목록 다이얼로그 표시
    pub fn show_tab_list(&mut self) {
        let active_panel = self.active_panel();
        let items = self.panel_tab_titles(active_panel);
        let selected_index = self.panel_active_tab_index(active_panel);
        self.dialog = Some(DialogKind::tab_list(items, selected_index));
    }

    /// 활성 패널 디렉토리 히스토리 목록 표시 (최신순)
    pub fn show_history_list(&mut self) {
        let items = self.active_panel_state().history_items_latest_first();
        if items.is_empty() {
            self.dialog = Some(DialogKind::message("History", "No history entries."));
            return;
        }
        let selected_index = items
            .iter()
            .position(|(_, _, is_current)| *is_current)
            .unwrap_or(0);
        self.dialog = Some(DialogKind::history_list(items, selected_index));
    }

    fn make_unique_bookmark_name(
        &self,
        desired_name: &str,
        exclude_index: Option<usize>,
    ) -> String {
        let desired = desired_name.trim();
        let base = if desired.is_empty() {
            "bookmark"
        } else {
            desired
        };
        if !self
            .bookmarks
            .iter()
            .enumerate()
            .any(|(idx, b)| Some(idx) != exclude_index && b.name.eq_ignore_ascii_case(base))
        {
            return base.to_string();
        }

        for n in 2.. {
            let candidate = format!("{} ({})", base, n);
            if !self.bookmarks.iter().enumerate().any(|(idx, b)| {
                Some(idx) != exclude_index && b.name.eq_ignore_ascii_case(&candidate)
            }) {
                return candidate;
            }
        }

        base.to_string()
    }

    fn bookmark_items(&self) -> Vec<(String, PathBuf)> {
        self.bookmarks
            .iter()
            .map(|b| (b.name.clone(), b.path.clone()))
            .collect()
    }

    pub fn add_bookmark_current_dir(&mut self) {
        let current_path = self.active_panel_state().current_path.clone();
        if self.bookmarks.iter().any(|b| b.path == current_path) {
            self.set_toast("Bookmark already exists");
            return;
        }

        let default_name = current_path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if current_path.parent().is_none() {
                    "/".to_string()
                } else {
                    current_path.to_string_lossy().to_string()
                }
            });
        let name = self.make_unique_bookmark_name(&default_name, None);
        self.bookmarks.push(PersistedBookmark {
            name: name.clone(),
            path: current_path,
        });
        let _ = self.save_persisted_bookmarks();
        self.set_toast(&format!("Bookmark added: {}", name));
    }

    pub fn show_bookmark_list(&mut self) {
        if self.bookmarks.is_empty() {
            self.dialog = Some(DialogKind::message("Bookmarks", "No bookmarks."));
            return;
        }

        let current_path = self.active_panel_state().current_path.clone();
        let items = self.bookmark_items();
        let selected_index = items
            .iter()
            .position(|(_, path)| *path == current_path)
            .unwrap_or(0);
        self.dialog = Some(DialogKind::bookmark_list(items, selected_index));
    }

    pub fn bookmark_list_move_down(&mut self) {
        if let Some(DialogKind::BookmarkList {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    pub fn bookmark_list_move_up(&mut self) {
        if let Some(DialogKind::BookmarkList { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    pub fn bookmark_list_confirm(&mut self) {
        let (selected_index, item_len) = if let Some(DialogKind::BookmarkList {
            items,
            selected_index,
        }) = &self.dialog
        {
            (*selected_index, items.len())
        } else {
            return;
        };

        if item_len == 0 || selected_index >= item_len {
            return;
        }

        let Some(bookmark) = self.bookmarks.get(selected_index).cloned() else {
            return;
        };

        if self.change_active_dir(bookmark.path, true, None) {
            self.dialog = None;
        } else {
            self.set_toast("Failed to open bookmark path");
        }
    }

    pub fn bookmark_list_delete_selected(&mut self) {
        let selected_index =
            if let Some(DialogKind::BookmarkList { selected_index, .. }) = &self.dialog {
                *selected_index
            } else {
                return;
            };

        if selected_index >= self.bookmarks.len() {
            return;
        }

        self.bookmarks.remove(selected_index);
        let _ = self.save_persisted_bookmarks();

        if self.bookmarks.is_empty() {
            self.dialog = None;
            self.set_toast("Bookmark deleted");
            return;
        }

        let new_index = selected_index.min(self.bookmarks.len().saturating_sub(1));
        self.dialog = Some(DialogKind::bookmark_list(self.bookmark_items(), new_index));
        self.set_toast("Bookmark deleted");
    }

    pub fn start_bookmark_rename_selected(&mut self) {
        let (selected_index, item_name) = if let Some(DialogKind::BookmarkList {
            items,
            selected_index,
        }) = &self.dialog
        {
            if items.is_empty() || *selected_index >= items.len() {
                return;
            }
            (*selected_index, items[*selected_index].0.clone())
        } else {
            return;
        };

        self.dialog = Some(DialogKind::bookmark_rename_input(item_name, selected_index));
    }

    pub fn confirm_bookmark_rename(&mut self, new_name: String, bookmark_index: usize) {
        if bookmark_index >= self.bookmarks.len() {
            self.dialog = None;
            return;
        }

        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            self.set_toast("Bookmark name cannot be empty");
            return;
        }

        let unique = self.make_unique_bookmark_name(trimmed, Some(bookmark_index));
        self.bookmarks[bookmark_index].name = unique;
        let _ = self.save_persisted_bookmarks();
        self.dialog = Some(DialogKind::bookmark_list(
            self.bookmark_items(),
            bookmark_index.min(self.bookmarks.len().saturating_sub(1)),
        ));
        self.set_toast("Bookmark renamed");
    }

    /// 탭 목록 다이얼로그에서 선택 이동 (아래)
    pub fn tab_list_move_down(&mut self) {
        if let Some(DialogKind::TabList {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    /// 탭 목록 다이얼로그에서 선택 이동 (위)
    pub fn tab_list_move_up(&mut self) {
        if let Some(DialogKind::TabList { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    /// 탭 목록 다이얼로그에서 선택 확인
    pub fn tab_list_confirm(&mut self) {
        let index = if let Some(DialogKind::TabList { selected_index, .. }) = &self.dialog {
            Some(*selected_index)
        } else {
            None
        };
        if let Some(index) = index {
            self.switch_tab_active_panel(index);
            self.dialog = None;
        }
    }

    /// 히스토리 목록 다이얼로그에서 선택 이동 (아래)
    pub fn history_list_move_down(&mut self) {
        if let Some(DialogKind::HistoryList {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    /// 히스토리 목록 다이얼로그에서 선택 이동 (위)
    pub fn history_list_move_up(&mut self) {
        if let Some(DialogKind::HistoryList { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    /// 히스토리 목록 다이얼로그에서 선택 확인
    pub fn history_list_confirm(&mut self) {
        let (selected_index, item_len) = if let Some(DialogKind::HistoryList {
            items,
            selected_index,
        }) = &self.dialog
        {
            (*selected_index, items.len())
        } else {
            return;
        };

        if item_len == 0 || selected_index >= item_len {
            return;
        }

        let target_index = item_len - 1 - selected_index;
        let (target_path, old_index) = {
            let panel = self.active_panel_state_mut();
            let old = panel.history_index;
            (panel.history_jump_to(target_index), old)
        };

        if let Some(path) = target_path {
            if self.change_active_dir(path, false, None) {
                self.dialog = None;
            } else {
                self.active_panel_state_mut().history_index = old_index;
                self.set_toast("Failed to open history path");
            }
        }
    }

    /// 현재 패널 히스토리 전체 삭제 (현재 경로만 유지)
    pub fn history_list_clear_all(&mut self) {
        self.active_panel_state_mut().clear_history_to_current();
        let items = self.active_panel_state().history_items_latest_first();
        self.dialog = Some(DialogKind::history_list(items, 0));
        let _ = self.save_persisted_histories();
        self.set_toast("History cleared");
    }

    /// 히스토리 뒤로 이동 (Alt+Left)
    pub fn history_back(&mut self) {
        let (target_path, old_index) = {
            let panel = self.active_panel_state_mut();
            let old = panel.history_index;
            (panel.history_back_target(), old)
        };

        if let Some(path) = target_path {
            if !self.change_active_dir(path, false, None) {
                self.active_panel_state_mut().history_index = old_index;
                self.set_toast("History back failed");
            }
        } else {
            self.set_toast("No back history");
        }
    }

    /// 히스토리 앞으로 이동 (Alt+Right)
    pub fn history_forward(&mut self) {
        let (target_path, old_index) = {
            let panel = self.active_panel_state_mut();
            let old = panel.history_index;
            (panel.history_forward_target(), old)
        };

        if let Some(path) = target_path {
            if !self.change_active_dir(path, false, None) {
                self.active_panel_state_mut().history_index = old_index;
                self.set_toast("History forward failed");
            }
        } else {
            self.set_toast("No forward history");
        }
    }

    /// 마운트 포인트로 이동
    pub fn go_to_mount_point(&mut self, path: std::path::PathBuf) {
        if self.change_active_dir(path, true, None) {
            self.dialog = None;
        }
    }

    /// 마운트 포인트 다이얼로그에서 선택 이동 (아래)
    pub fn mount_points_move_down(&mut self) {
        if let Some(DialogKind::MountPoints {
            items,
            selected_index,
        }) = &mut self.dialog
        {
            if *selected_index + 1 < items.len() {
                *selected_index += 1;
            }
        }
    }

    /// 마운트 포인트 다이얼로그에서 선택 이동 (위)
    pub fn mount_points_move_up(&mut self) {
        if let Some(DialogKind::MountPoints { selected_index, .. }) = &mut self.dialog {
            if *selected_index > 0 {
                *selected_index -= 1;
            }
        }
    }

    /// 마운트 포인트 다이얼로그에서 선택 확인
    pub fn mount_points_confirm(&mut self) {
        let path = if let Some(DialogKind::MountPoints {
            items,
            selected_index,
        }) = &self.dialog
        {
            items.get(*selected_index).map(|(_, p)| p.clone())
        } else {
            None
        };
        if let Some(path) = path {
            self.go_to_mount_point(path);
        }
    }

    // === 필터/검색 관련 메서드 (Phase 5.2) ===

    /// 필터 시작 (/)
    pub fn start_filter(&mut self) {
        let initial = self.active_panel_state().filter.clone();
        // 다이얼로그 취소 시 복원하기 위해 현재 필터 저장
        self.dialog = Some(DialogKind::filter_input(initial.as_deref()));
    }

    /// 필터 해제
    pub fn clear_filter(&mut self) {
        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut().set_filter(None);
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut().set_filter(None);
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
        self.set_toast("Filter cleared");
    }

    /// 필터 확인 적용
    pub fn confirm_filter(&mut self, pattern: String) {
        let pattern = pattern.trim().to_string();
        if pattern.is_empty() {
            self.clear_filter();
            self.dialog = None;
            return;
        }

        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut()
                    .set_filter(Some(pattern.clone()));
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut()
                    .set_filter(Some(pattern.clone()));
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
        self.dialog = None;
        self.set_toast(&format!("Filter: {}", pattern));
    }

    /// 라이브 필터 업데이트 (다이얼로그 입력 중 실시간 반영)
    pub fn apply_live_filter(&mut self, pattern: &str) {
        let filter = if pattern.is_empty() {
            None
        } else {
            Some(pattern.to_string())
        };
        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut().set_filter(filter);
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut().set_filter(filter);
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
    }

    /// 필터 취소 (다이얼로그 ESC — 필터 해제하고 다이얼로그 닫기)
    pub fn cancel_filter(&mut self) {
        match self.active_panel() {
            ActivePanel::Left => {
                self.left_active_panel_state_mut().set_filter(None);
                let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
            }
            ActivePanel::Right => {
                self.right_active_panel_state_mut().set_filter(None);
                let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
            }
        }
        self.dialog = None;
    }

    // === FilterInput 다이얼로그 입력 처리 ===

    fn prev_char_start(value: &str, cursor_pos: usize) -> usize {
        value[..cursor_pos]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn is_word_delimiter(ch: char) -> bool {
        ch.is_whitespace()
            || matches!(
                ch,
                '/' | '\\'
                    | ':'
                    | ';'
                    | ','
                    | '.'
                    | '|'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | '"'
                    | '\''
                    | '`'
            )
    }

    fn delete_prev_word(value: &mut String, cursor_pos: &mut usize) {
        if *cursor_pos == 0 {
            return;
        }

        let original = *cursor_pos;
        let mut pos = original;

        // 1) 커서 왼쪽의 구분자들을 먼저 건너뜀
        while pos > 0 {
            let prev = Self::prev_char_start(value, pos);
            let ch = value[prev..pos].chars().next().unwrap_or_default();
            if Self::is_word_delimiter(ch) {
                pos = prev;
            } else {
                break;
            }
        }

        // 2) 실제 단어 시작까지 이동
        while pos > 0 {
            let prev = Self::prev_char_start(value, pos);
            let ch = value[prev..pos].chars().next().unwrap_or_default();
            if Self::is_word_delimiter(ch) {
                break;
            }
            pos = prev;
        }

        value.replace_range(pos..original, "");
        *cursor_pos = pos;
    }

    pub fn dialog_filter_input_char(&mut self, c: char) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            value.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_backspace(&mut self) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                let prev = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(prev);
                *cursor_pos = prev;
            }
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_delete_prev_word(&mut self) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            Self::delete_prev_word(value, cursor_pos);
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_delete(&mut self) {
        let new_value = if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                value.remove(*cursor_pos);
            }
            Some(value.clone())
        } else {
            None
        };
        if let Some(v) = new_value {
            self.apply_live_filter(&v);
        }
    }

    pub fn dialog_filter_input_left(&mut self) {
        if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                *cursor_pos = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
    }

    pub fn dialog_filter_input_right(&mut self) {
        if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                *cursor_pos = value[*cursor_pos..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor_pos + i)
                    .unwrap_or(value.len());
            }
        }
    }

    pub fn dialog_filter_input_home(&mut self) {
        if let Some(DialogKind::FilterInput { cursor_pos, .. }) = &mut self.dialog {
            *cursor_pos = 0;
        }
    }

    pub fn dialog_filter_input_end(&mut self) {
        if let Some(DialogKind::FilterInput {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            *cursor_pos = value.len();
        }
    }

    pub fn dialog_filter_toggle_button(&mut self) {
        if let Some(DialogKind::FilterInput {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    pub fn get_filter_input_value(&self) -> Option<String> {
        if let Some(DialogKind::FilterInput { value, .. }) = &self.dialog {
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn get_filter_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::FilterInput {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    /// 양쪽 패널 새로고침
    pub fn refresh_both_panels(&mut self) {
        let _ = self.left_tabs.active_mut().refresh(&self.filesystem);
        let _ = self.right_tabs.active_mut().refresh(&self.filesystem);
    }

    // === 다이얼로그 입력 처리 메서드 ===

    /// 입력 다이얼로그: 문자 입력
    pub fn dialog_input_char(&mut self, c: char) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            value.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 백스페이스
    pub fn dialog_input_backspace(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                let prev = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(prev);
                *cursor_pos = prev;
            }
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 이전 단어 삭제 (Ctrl+W)
    pub fn dialog_input_delete_prev_word(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            Self::delete_prev_word(value, cursor_pos);
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: Delete
    pub fn dialog_input_delete(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                value.remove(*cursor_pos);
            }
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 커서 왼쪽
    pub fn dialog_input_left(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                *cursor_pos = value[..*cursor_pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 커서 오른쪽
    pub fn dialog_input_right(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                *cursor_pos = value[*cursor_pos..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor_pos + i)
                    .unwrap_or(value.len());
            }
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: Home
    pub fn dialog_input_home(&mut self) {
        if let Some(DialogKind::Input { cursor_pos, .. }) = &mut self.dialog {
            *cursor_pos = 0;
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: End
    pub fn dialog_input_end(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            *cursor_pos = value.len();
        }
        self.update_input_completion_state();
    }

    /// 입력 다이얼로그: 버튼 선택 변경 (Tab)
    pub fn dialog_input_toggle_button(&mut self) {
        if let Some(DialogKind::Input {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    /// 입력 다이얼로그: 선택된 버튼 반환
    pub fn get_dialog_input_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::Input {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    /// 확인 다이얼로그: 버튼 선택 변경
    pub fn dialog_confirm_toggle(&mut self) {
        if let Some(DialogKind::Confirm {
            selected_button, ..
        }) = &mut self.dialog
        {
            *selected_button = if *selected_button == 0 { 1 } else { 0 };
        }
    }

    /// 충돌 다이얼로그: 옵션 이동
    pub fn dialog_conflict_next(&mut self) {
        if let Some(DialogKind::Conflict {
            selected_option, ..
        }) = &mut self.dialog
        {
            *selected_option = (*selected_option + 1) % 5;
        }
    }

    /// 충돌 다이얼로그: 옵션 이동 (이전)
    pub fn dialog_conflict_prev(&mut self) {
        if let Some(DialogKind::Conflict {
            selected_option, ..
        }) = &mut self.dialog
        {
            *selected_option = if *selected_option == 0 {
                4
            } else {
                *selected_option - 1
            };
        }
    }

    /// 현재 다이얼로그 입력값 반환 (Input 다이얼로그용)
    pub fn get_dialog_input_value(&self) -> Option<String> {
        if let Some(DialogKind::Input { value, .. }) = &self.dialog {
            Some(value.clone())
        } else {
            None
        }
    }

    /// 현재 다이얼로그 선택 버튼 반환 (Confirm 다이얼로그용)
    pub fn get_dialog_selected_button(&self) -> Option<usize> {
        if let Some(DialogKind::Confirm {
            selected_button, ..
        }) = &self.dialog
        {
            Some(*selected_button)
        } else {
            None
        }
    }

    /// 현재 다이얼로그 선택 옵션 반환 (Conflict 다이얼로그용)
    pub fn get_dialog_conflict_option(&self) -> Option<ConflictResolution> {
        if let Some(DialogKind::Conflict {
            selected_option, ..
        }) = &self.dialog
        {
            Some(match selected_option {
                0 => ConflictResolution::Overwrite,
                1 => ConflictResolution::Skip,
                2 => ConflictResolution::OverwriteAll,
                3 => ConflictResolution::SkipAll,
                _ => ConflictResolution::Cancel,
            })
        } else {
            None
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
                left_tabs: PanelTabs::new(PanelState::new(current_dir.clone())),
                right_tabs: PanelTabs::new(PanelState::new(current_dir)),
                filesystem,
                menus: create_default_menus(),
                menu_state: MenuState::new(),
                theme_manager: ThemeManager::new(),
                dialog: None,
                pending_operation: None,
                pending_key: None,
                pending_key_time: None,
                toast_message: None,
                icon_mode: crate::ui::components::panel::IconMode::default(),
                size_format: SizeFormat::default(),
                ime_status: ImeStatus::Unknown,
                default_terminal_editor: Self::FALLBACK_TERMINAL_EDITOR.to_string(),
                pending_terminal_editor_request: None,
                bookmarks: Vec::new(),
                bookmarks_store_override: None,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::error::BokslDirError;
    use std::fs;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    fn make_test_app() -> App {
        App::new_for_test()
    }

    fn run_file_operation_until_done(app: &mut App) {
        let mut guard = 0usize;
        while app.pending_operation.is_some() && guard < 10_000 {
            app.process_next_file();
            guard += 1;
        }
        assert!(guard < 10_000, "operation loop guard exceeded");
    }

    /// 재귀 경로 검사 테스트: 디렉토리를 자기 자신 내부로 복사
    #[test]
    fn test_is_recursive_path_into_self() {
        let temp = TempDir::new().unwrap();
        let parent = temp.path().join("parent");
        let child = parent.join("child");

        fs::create_dir_all(&child).unwrap();

        // parent -> parent/child 는 재귀 복사
        assert!(App::is_recursive_path(&parent, &child));
    }

    /// 재귀 경로 검사 테스트: 서로 다른 디렉토리는 OK
    #[test]
    fn test_is_recursive_path_different_dirs() {
        let temp = TempDir::new().unwrap();
        let dir_a = temp.path().join("dir_a");
        let dir_b = temp.path().join("dir_b");

        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();

        // dir_a -> dir_b 는 재귀 아님
        assert!(!App::is_recursive_path(&dir_a, &dir_b));
    }

    /// 재귀 경로 검사 테스트: 파일은 재귀 검사 대상 아님
    #[test]
    fn test_is_recursive_path_file_not_checked() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file.txt");
        let dest = temp.path().join("dest");

        fs::write(&file, "test").unwrap();
        fs::create_dir_all(&dest).unwrap();

        // 파일은 항상 false
        assert!(!App::is_recursive_path(&file, &dest));
    }

    /// 재귀 경로 검사 테스트: 형제 디렉토리는 OK
    #[test]
    fn test_is_recursive_path_sibling_dirs() {
        let temp = TempDir::new().unwrap();
        let parent = temp.path().join("parent");
        let sibling = temp.path().join("sibling");

        fs::create_dir_all(&parent).unwrap();
        fs::create_dir_all(&sibling).unwrap();

        // parent -> sibling 은 재귀 아님
        assert!(!App::is_recursive_path(&parent, &sibling));
    }

    /// 재귀 경로 검사 테스트: 같은 디렉토리 (자기 자신)
    #[test]
    fn test_is_recursive_path_same_dir() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("dir");

        fs::create_dir_all(&dir).unwrap();

        // dir -> dir 자체도 재귀로 간주
        assert!(App::is_recursive_path(&dir, &dir));
    }

    /// check_recursive_operation 테스트: 재귀 발견 시 에러 메시지 반환
    #[test]
    fn test_check_recursive_operation_detects_recursive() {
        let temp = TempDir::new().unwrap();
        let parent = temp.path().join("parent");
        let child = parent.join("child");

        fs::create_dir_all(&child).unwrap();

        let sources = vec![parent.clone()];
        let result = App::check_recursive_operation(&sources, OperationType::Copy, &child);

        assert!(result.is_some());
        assert!(result.unwrap().contains("Cannot copy"));
    }

    /// check_recursive_operation 테스트: 정상 복사는 None 반환
    #[test]
    fn test_check_recursive_operation_allows_valid() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let dest = temp.path().join("dest");

        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&dest).unwrap();

        let sources = vec![source];
        let result = App::check_recursive_operation(&sources, OperationType::Copy, &dest);

        assert!(result.is_none());
    }

    /// check_recursive_operation 테스트: 여러 소스 중 하나라도 재귀면 에러
    #[test]
    fn test_check_recursive_operation_multiple_sources() {
        let temp = TempDir::new().unwrap();
        let ok_dir = temp.path().join("ok");
        let bad_dir = temp.path().join("bad");
        let dest = bad_dir.join("child");

        fs::create_dir_all(&ok_dir).unwrap();
        fs::create_dir_all(&dest).unwrap();

        let sources = vec![ok_dir, bad_dir.clone()];
        let result = App::check_recursive_operation(&sources, OperationType::Move, &dest);

        assert!(result.is_some());
        assert!(result.unwrap().contains("Cannot move"));
    }

    #[test]
    fn test_tab_create_close_and_guard_last() {
        let mut app = make_test_app();

        assert_eq!(app.left_tabs.len(), 1);
        app.new_tab_active_panel();
        assert_eq!(app.left_tabs.len(), 2);
        assert_eq!(app.left_tabs.active_index(), 1);

        app.close_tab_active_panel();
        assert_eq!(app.left_tabs.len(), 1);

        app.close_tab_active_panel();
        assert_eq!(app.left_tabs.len(), 1);
        assert_eq!(app.toast_display(), Some("Cannot close last tab"));
    }

    #[test]
    fn test_tab_prev_next_and_switch() {
        let mut app = make_test_app();
        app.new_tab_active_panel();
        app.new_tab_active_panel();
        assert_eq!(app.left_tabs.active_index(), 2);

        app.prev_tab_active_panel();
        assert_eq!(app.left_tabs.active_index(), 1);
        app.next_tab_active_panel();
        assert_eq!(app.left_tabs.active_index(), 2);

        app.switch_tab_active_panel(0);
        assert_eq!(app.left_tabs.active_index(), 0);
        app.switch_tab_active_panel(9);
        assert_eq!(app.left_tabs.active_index(), 0);
    }

    #[test]
    fn test_tab_state_persists_per_tab() {
        let mut app = make_test_app();

        app.active_panel_state_mut()
            .set_filter(Some("alpha".to_string()));
        app.new_tab_active_panel();
        app.active_panel_state_mut()
            .set_filter(Some("beta".to_string()));

        app.prev_tab_active_panel();
        assert_eq!(app.active_panel_state().filter.as_deref(), Some("alpha"));

        app.next_tab_active_panel();
        assert_eq!(app.active_panel_state().filter.as_deref(), Some("beta"));
    }

    #[test]
    fn test_tab_max_limit_is_five() {
        let mut app = make_test_app();

        for _ in 0..4 {
            app.new_tab_active_panel();
        }
        assert_eq!(app.left_tabs.len(), 5);

        app.new_tab_active_panel();
        assert_eq!(app.left_tabs.len(), 5);
        assert_eq!(app.toast_display(), Some("Max 5 tabs per panel"));
    }

    #[test]
    fn test_tab_list_dialog_select_and_switch() {
        let mut app = make_test_app();
        app.new_tab_active_panel();
        app.new_tab_active_panel();
        assert_eq!(app.left_tabs.active_index(), 2);

        app.show_tab_list();
        app.tab_list_move_up();
        app.tab_list_move_up();
        app.tab_list_confirm();

        assert_eq!(app.left_tabs.active_index(), 0);
        assert!(app.dialog.is_none());
    }

    #[test]
    fn test_directory_navigation_records_history() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let child = root.join("child");
        fs::create_dir_all(&child).unwrap();

        app.go_to_mount_point(root.clone());
        assert_eq!(app.active_panel_state().current_path, root);

        let fs = FileSystem::new();
        let _ = app.active_panel_state_mut().refresh(&fs);
        let has_parent = app.active_panel_state().current_path.parent().is_some();
        let offset = if has_parent { 1 } else { 0 };
        let entry_index = app
            .active_panel_state()
            .entries
            .iter()
            .position(|e| e.name == "child")
            .unwrap();
        app.active_panel_state_mut().selected_index = entry_index + offset;
        app.enter_selected();

        assert_eq!(app.active_panel_state().current_path, child);
        let history = &app.active_panel_state().history_entries;
        assert!(history.contains(&root));
        assert!(history.contains(&app.active_panel_state().current_path));
    }

    #[test]
    fn test_history_back_forward_index_based_navigation() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let p1 = temp.path().join("p1");
        let p2 = temp.path().join("p2");
        let p3 = temp.path().join("p3");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();
        fs::create_dir_all(&p3).unwrap();

        app.go_to_mount_point(p1.clone());
        app.go_to_mount_point(p2.clone());
        app.go_to_mount_point(p3.clone());
        assert_eq!(app.active_panel_state().current_path, p3);

        app.history_back();
        assert_eq!(app.active_panel_state().current_path, p2);
        app.history_back();
        assert_eq!(app.active_panel_state().current_path, p1);
        app.history_forward();
        assert_eq!(app.active_panel_state().current_path, p2);
    }

    #[test]
    fn test_history_list_default_selection_and_confirm() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let p1 = temp.path().join("p1");
        let p2 = temp.path().join("p2");
        let p3 = temp.path().join("p3");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();
        fs::create_dir_all(&p3).unwrap();

        app.go_to_mount_point(p1.clone());
        app.go_to_mount_point(p2.clone());
        app.go_to_mount_point(p3.clone());
        app.history_back();
        assert_eq!(app.active_panel_state().current_path, p2);

        app.show_history_list();
        if let Some(DialogKind::HistoryList {
            items,
            selected_index,
        }) = &app.dialog
        {
            assert_eq!(*selected_index, 1);
            assert!(items[*selected_index].2);
        } else {
            panic!("history list dialog not shown");
        }

        // 최신 항목(p3) 선택 후 이동
        app.history_list_move_up();
        app.history_list_confirm();
        assert_eq!(app.active_panel_state().current_path, p3);
        assert!(app.dialog.is_none());
    }

    #[test]
    fn test_history_is_independent_per_tab() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        let c = temp.path().join("c");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::create_dir_all(&c).unwrap();

        app.go_to_mount_point(a.clone());
        app.new_tab_active_panel();
        app.go_to_mount_point(b.clone());
        app.go_to_mount_point(c.clone());
        assert_eq!(app.active_panel_state().current_path, c);

        app.prev_tab_active_panel();
        assert_eq!(app.active_panel_state().current_path, a);
        app.history_back();
        assert_ne!(app.active_panel_state().current_path, b);

        app.next_tab_active_panel();
        assert_eq!(app.active_panel_state().current_path, c);
        app.history_back();
        assert_eq!(app.active_panel_state().current_path, b);
    }

    #[test]
    fn test_history_list_clear_all_keeps_current_only() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let p1 = temp.path().join("p1");
        let p2 = temp.path().join("p2");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        app.go_to_mount_point(p1);
        app.go_to_mount_point(p2.clone());
        assert!(app.active_panel_state().history_entries.len() >= 2);

        app.show_history_list();
        app.history_list_clear_all();

        assert_eq!(app.active_panel_state().history_entries, vec![p2.clone()]);
        assert_eq!(app.active_panel_state().history_index, 0);
        if let Some(DialogKind::HistoryList {
            items,
            selected_index,
        }) = &app.dialog
        {
            assert_eq!(*selected_index, 0);
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].1, p2);
            assert!(items[0].2);
        } else {
            panic!("history list dialog not shown");
        }
    }

    #[test]
    fn test_dialog_input_completion_prefers_active_tab_history() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let history_dir = base.join("docs_history").join("nested");
        let fs_dir = base.join("docs_fs");
        fs::create_dir_all(&history_dir).unwrap();
        fs::create_dir_all(&fs_dir).unwrap();

        app.go_to_mount_point(base.clone());
        {
            let panel = app.active_panel_state_mut();
            panel.history_entries = vec![base.clone(), history_dir.clone()];
            panel.history_index = 1;
        }

        app.dialog = Some(DialogKind::operation_path_input(
            "Copy",
            "Copy to:",
            "doc",
            base.clone(),
        ));
        app.update_input_completion_state();

        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            ..
        }) = &app.dialog
        {
            assert_eq!(
                completion_candidates.first().map(String::as_str),
                Some("docs_history")
            );
            assert_eq!(
                completion_candidates.get(1).map(String::as_str),
                Some("docs_fs")
            );
            assert_eq!(*completion_index, Some(0));
        } else {
            panic!("input dialog not shown");
        }

        app.dialog_input_apply_selected_completion();
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &app.dialog
        {
            assert_eq!(value, "docs_history");
            assert_eq!(*cursor_pos, value.len());
        } else {
            panic!("input dialog not shown");
        }

        app.dialog_input_toggle_button();
        if let Some(DialogKind::Input {
            selected_button,
            value,
            ..
        }) = &app.dialog
        {
            assert_eq!(*selected_button, 1);
            assert_eq!(value, "docs_history");
        } else {
            panic!("input dialog not shown");
        }
    }

    #[test]
    fn test_dialog_input_cycle_next_prev_applies_completion() {
        let mut app = make_test_app();
        app.dialog = Some(DialogKind::go_to_path_input("", PathBuf::from(".")));
        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            ..
        }) = &mut app.dialog
        {
            *completion_candidates = vec!["alpha".to_string(), "beta".to_string()];
            *completion_index = Some(0);
        }

        app.dialog_input_cycle_completion_next();
        assert_eq!(app.get_dialog_input_value().as_deref(), Some("beta"));

        app.dialog_input_cycle_completion_prev();
        assert_eq!(app.get_dialog_input_value().as_deref(), Some("alpha"));
    }

    #[test]
    fn test_go_to_path_relative_success() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let child = base.join("child");
        fs::create_dir_all(&child).unwrap();

        app.go_to_mount_point(base.clone());
        app.start_go_to_path();
        app.confirm_input_dialog("child".to_string());

        assert_eq!(app.active_panel_state().current_path, child);
        assert!(app.dialog.is_none());
    }

    #[test]
    fn test_go_to_path_fails_for_non_directory() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let file = base.join("file.txt");
        fs::create_dir_all(&base).unwrap();
        fs::write(&file, "data").unwrap();

        app.go_to_mount_point(base);
        app.start_go_to_path();
        app.confirm_input_dialog("file.txt".to_string());

        assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
    }

    #[test]
    fn test_start_open_default_app_rejects_parent_entry() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        fs::create_dir_all(&base).unwrap();

        app.go_to_mount_point(base);
        app.active_panel_state_mut().selected_index = 0;
        app.start_open_default_app();

        assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
    }

    #[test]
    fn test_start_open_default_app_rejects_directory() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let dir = base.join("docs");
        fs::create_dir_all(&dir).unwrap();

        app.go_to_mount_point(base.clone());
        let has_parent = app.active_panel_state().current_path.parent().is_some();
        let offset = if has_parent { 1 } else { 0 };
        let entry_index = app
            .active_panel_state()
            .entries
            .iter()
            .position(|e| e.path == dir)
            .expect("directory entry should exist");
        app.active_panel_state_mut().selected_index = entry_index + offset;
        app.start_open_default_app();

        assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
    }

    #[test]
    fn test_start_open_terminal_editor_rejects_parent_entry() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        fs::create_dir_all(&base).unwrap();

        app.go_to_mount_point(base);
        app.active_panel_state_mut().selected_index = 0;
        app.start_open_terminal_editor();

        assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
        assert!(app.take_pending_terminal_editor_request().is_none());
    }

    #[test]
    fn test_start_open_terminal_editor_rejects_directory() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let dir = base.join("docs");
        fs::create_dir_all(&dir).unwrap();

        app.go_to_mount_point(base.clone());
        let has_parent = app.active_panel_state().current_path.parent().is_some();
        let offset = if has_parent { 1 } else { 0 };
        let entry_index = app
            .active_panel_state()
            .entries
            .iter()
            .position(|e| e.path == dir)
            .expect("directory entry should exist");
        app.active_panel_state_mut().selected_index = entry_index + offset;
        app.start_open_terminal_editor();

        assert!(matches!(app.dialog, Some(DialogKind::Error { .. })));
        assert!(app.take_pending_terminal_editor_request().is_none());
    }

    #[test]
    fn test_start_open_terminal_editor_queues_request_for_file() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let file = base.join("notes.txt");
        fs::create_dir_all(&base).unwrap();
        fs::write(&file, "hello").unwrap();

        app.go_to_mount_point(base.clone());
        app.execute_action(Action::SetDefaultEditorVim);

        let has_parent = app.active_panel_state().current_path.parent().is_some();
        let offset = if has_parent { 1 } else { 0 };
        let entry_index = app
            .active_panel_state()
            .entries
            .iter()
            .position(|e| e.path == file)
            .expect("file entry should exist");
        app.active_panel_state_mut().selected_index = entry_index + offset;
        app.start_open_terminal_editor();

        let request = app
            .take_pending_terminal_editor_request()
            .expect("request should be queued");
        assert_eq!(request.editor_command, "vim");
        assert_eq!(request.target_path, file);
    }

    #[test]
    fn test_apply_terminal_editor_result_sets_toast_on_success() {
        let mut app = make_test_app();
        let request = TerminalEditorRequest {
            editor_command: "vi".to_string(),
            target_path: PathBuf::from("/tmp/example.txt"),
        };

        app.apply_terminal_editor_result(&request, Ok(()));

        assert_eq!(app.toast_display(), Some("Edited: example.txt"));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn test_apply_terminal_editor_result_shows_error_on_failure() {
        let mut app = make_test_app();
        let request = TerminalEditorRequest {
            editor_command: "vi".to_string(),
            target_path: PathBuf::from("/tmp/example.txt"),
        };

        app.apply_terminal_editor_result(
            &request,
            Err("Failed to start 'vi': not found".to_string()),
        );

        match &app.dialog {
            Some(DialogKind::Error { message, .. }) => {
                assert!(message.contains("Open in terminal editor failed."));
                assert!(message.contains("Failed to start 'vi'"));
            }
            other => panic!("expected error dialog, got {:?}", other),
        }
    }

    #[test]
    fn test_editor_preset_actions_update_default_editor() {
        let mut app = make_test_app();

        app.execute_action(Action::SetDefaultEditorVim);
        assert_eq!(app.default_terminal_editor, "vim");
        assert_eq!(app.toast_display(), Some("Default editor: vim"));

        app.execute_action(Action::SetDefaultEditorNano);
        assert_eq!(app.default_terminal_editor, "nano");
        assert_eq!(app.toast_display(), Some("Default editor: nano"));

        app.execute_action(Action::SetDefaultEditorEmacs);
        assert_eq!(app.default_terminal_editor, "emacs");
        assert_eq!(app.toast_display(), Some("Default editor: emacs"));

        app.execute_action(Action::SetDefaultEditorVi);
        assert_eq!(app.default_terminal_editor, "vi");
        assert_eq!(app.toast_display(), Some("Default editor: vi"));
    }

    #[test]
    fn test_apply_open_default_app_result_sets_toast_on_success() {
        let mut app = make_test_app();
        let file_path = PathBuf::from("/tmp/example.txt");

        app.apply_open_default_app_result(&file_path, Ok(()));

        assert_eq!(app.toast_display(), Some("Opened: example.txt"));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn test_apply_open_default_app_result_shows_error_on_failure() {
        let mut app = make_test_app();
        let file_path = PathBuf::from("/tmp/example.txt");
        let error = BokslDirError::ExternalOpenFailed {
            path: file_path.clone(),
            reason: "mock failure".to_string(),
        };

        app.apply_open_default_app_result(&file_path, Err(error));

        match &app.dialog {
            Some(DialogKind::Error { message, .. }) => {
                assert!(message.contains("Open with default app failed."));
                assert!(message.contains("mock failure"));
            }
            other => panic!("expected error dialog, got {:?}", other),
        }
    }

    #[test]
    fn test_operation_destination_accepts_relative_path_with_base() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let source = base.join("source.txt");
        let target = base.join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(&source, "payload").unwrap();

        app.pending_operation = Some(PendingOperation::new(
            OperationType::Copy,
            vec![source],
            base.clone(),
        ));
        app.dialog = Some(DialogKind::operation_path_input(
            "Copy",
            "Copy to:",
            "target",
            base.clone(),
        ));

        app.confirm_input_dialog("target".to_string());

        let pending = app.pending_operation.as_ref().expect("pending operation");
        assert_eq!(pending.dest_dir, target);
        assert!(matches!(app.dialog, Some(DialogKind::Progress { .. })));
    }

    #[test]
    fn test_operation_destination_accepts_absolute_path() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        let source = base.join("source.txt");
        let target = temp.path().join("target_abs");
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(&source, "payload").unwrap();

        app.pending_operation = Some(PendingOperation::new(
            OperationType::Copy,
            vec![source],
            base.clone(),
        ));
        app.dialog = Some(DialogKind::operation_path_input(
            "Copy",
            "Copy to:",
            target.to_string_lossy(),
            base,
        ));

        app.confirm_input_dialog(target.to_string_lossy().to_string());

        let pending = app.pending_operation.as_ref().expect("pending operation");
        assert_eq!(pending.dest_dir, target);
        assert!(matches!(app.dialog, Some(DialogKind::Progress { .. })));
    }

    #[test]
    fn test_history_persistence_encode_decode_roundtrip() {
        let left_entries = vec![
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/c"),
        ];
        let right_entries = vec![PathBuf::from("/x"), PathBuf::from("/y")];

        let text = App::encode_histories(&left_entries, 2, &right_entries, 1).unwrap();
        let decoded = App::decode_histories(&text).unwrap();

        assert_eq!(decoded.0 .0, left_entries);
        assert_eq!(decoded.0 .1, 2);
        assert_eq!(decoded.1 .0, right_entries);
        assert_eq!(decoded.1 .1, 1);
    }

    #[test]
    fn test_apply_loaded_history_keeps_non_consecutive_duplicates() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        app.apply_loaded_history(
            ActivePanel::Left,
            vec![a.clone(), b.clone(), a.clone(), b.clone(), a.clone()],
            3,
        );

        let history = &app.left_active_panel_state().history_entries;
        assert_eq!(history.len(), 5);
        assert_eq!(history[0], a);
        assert_eq!(history[1], b);
        assert_eq!(history[2], a);
        assert_eq!(history[3], b);
        assert_eq!(history[4], a);
        assert_eq!(app.left_active_panel_state().history_index, 3);
        assert_eq!(app.left_active_panel_state().current_path, b);
    }

    #[test]
    fn test_apply_loaded_history_clamps_index() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        app.apply_loaded_history(ActivePanel::Left, vec![a.clone(), b.clone()], 99);

        let panel = app.left_active_panel_state();
        assert_eq!(panel.history_entries, vec![a, b.clone()]);
        assert_eq!(panel.history_index, 1);
        assert_eq!(panel.current_path, b);
    }

    #[test]
    fn test_add_bookmark_stores_current_path_and_prevents_duplicate_path() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let p1 = temp.path().join("p1");
        fs::create_dir_all(&p1).unwrap();

        app.go_to_mount_point(p1.clone());
        app.add_bookmark_current_dir();
        assert_eq!(app.bookmarks.len(), 1);
        assert_eq!(app.bookmarks[0].path, p1);

        app.add_bookmark_current_dir();
        assert_eq!(app.bookmarks.len(), 1);
        assert_eq!(app.toast_display(), Some("Bookmark already exists"));
    }

    #[test]
    fn test_add_bookmark_assigns_unique_name_suffix() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let p1 = root.join("same");
        let p2 = root.join("x").join("same");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        app.go_to_mount_point(p1);
        app.add_bookmark_current_dir();
        app.go_to_mount_point(p2);
        app.add_bookmark_current_dir();

        assert_eq!(app.bookmarks.len(), 2);
        assert_eq!(app.bookmarks[0].name, "same");
        assert_eq!(app.bookmarks[1].name, "same (2)");
    }

    #[test]
    fn test_bookmark_list_confirm_moves_to_selected_path() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let p1 = temp.path().join("p1");
        let p2 = temp.path().join("p2");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        app.go_to_mount_point(p1.clone());
        app.add_bookmark_current_dir();
        app.go_to_mount_point(p2.clone());
        app.add_bookmark_current_dir();

        app.go_to_mount_point(p1);
        app.show_bookmark_list();
        app.bookmark_list_move_down();
        app.bookmark_list_confirm();

        assert_eq!(app.active_panel_state().current_path, p2);
        assert!(app.dialog.is_none());
    }

    #[test]
    fn test_bookmark_delete_reindexes_and_closes_on_last_delete() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let p1 = temp.path().join("p1");
        let p2 = temp.path().join("p2");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        app.go_to_mount_point(p1);
        app.add_bookmark_current_dir();
        app.go_to_mount_point(p2);
        app.add_bookmark_current_dir();

        app.show_bookmark_list();
        app.bookmark_list_move_down();
        app.bookmark_list_delete_selected();
        assert_eq!(app.bookmarks.len(), 1);
        if let Some(DialogKind::BookmarkList {
            items,
            selected_index,
        }) = &app.dialog
        {
            assert_eq!(items.len(), 1);
            assert_eq!(*selected_index, 0);
        } else {
            panic!("bookmark list dialog not shown");
        }

        app.bookmark_list_delete_selected();
        assert!(app.bookmarks.is_empty());
        assert!(app.dialog.is_none());
    }

    #[test]
    fn test_bookmark_rename_validates_and_applies_unique_suffix() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let p1 = temp.path().join("p1");
        let p2 = temp.path().join("p2");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        app.bookmarks = vec![
            PersistedBookmark {
                name: "Work".to_string(),
                path: p1,
            },
            PersistedBookmark {
                name: "Notes".to_string(),
                path: p2,
            },
        ];

        app.confirm_bookmark_rename("   ".to_string(), 0);
        assert_eq!(app.bookmarks[0].name, "Work");
        assert_eq!(app.toast_display(), Some("Bookmark name cannot be empty"));

        app.confirm_bookmark_rename("Notes".to_string(), 0);
        assert_eq!(app.bookmarks[0].name, "Notes (2)");
        if let Some(DialogKind::BookmarkList { selected_index, .. }) = &app.dialog {
            assert_eq!(*selected_index, 0);
        } else {
            panic!("bookmark list dialog not shown");
        }
    }

    #[test]
    fn test_bookmark_persistence_encode_decode_roundtrip() {
        let bookmarks = vec![
            PersistedBookmark {
                name: "A".to_string(),
                path: PathBuf::from("/a"),
            },
            PersistedBookmark {
                name: "B".to_string(),
                path: PathBuf::from("/b"),
            },
        ];

        let text = App::encode_bookmarks(&bookmarks).unwrap();
        let decoded = App::decode_bookmarks(&text).unwrap();
        assert_eq!(decoded, bookmarks);
    }

    #[test]
    fn test_load_persisted_bookmarks_restores_state() {
        let temp = TempDir::new().unwrap();
        let bookmarks_file = temp.path().join("bookmarks.toml");

        let mut app = make_test_app();
        app.bookmarks = vec![PersistedBookmark {
            name: "Temp".to_string(),
            path: PathBuf::from("/tmp"),
        }];
        app.save_persisted_bookmarks_to_path(&bookmarks_file)
            .unwrap();

        let mut loaded = make_test_app();
        loaded.bookmarks.clear();
        loaded.load_persisted_bookmarks_from_path(&bookmarks_file);
        assert_eq!(loaded.bookmarks.len(), 1);
        assert_eq!(loaded.bookmarks[0].name, "Temp");
        assert_eq!(loaded.bookmarks[0].path, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_move_operation_removes_source_directories_when_successful() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let src_root = temp.path().join("src_root");
        let empty_dir = src_root.join("empty");
        let nested_dir = src_root.join("nested");
        let nested_file = nested_dir.join("data.txt");
        let dest_root = temp.path().join("dest_root");

        fs::create_dir_all(&empty_dir).unwrap();
        fs::create_dir_all(&nested_dir).unwrap();
        fs::write(&nested_file, "payload").unwrap();
        fs::create_dir_all(&dest_root).unwrap();

        let mut pending = PendingOperation::new(
            OperationType::Move,
            vec![src_root.clone()],
            dest_root.clone(),
        );
        app.prepare_and_start_operation(&mut pending, &dest_root);
        app.pending_operation = Some(pending);

        run_file_operation_until_done(&mut app);

        let moved_root = dest_root.join("src_root");
        assert!(!src_root.exists());
        assert!(moved_root.join("empty").is_dir());
        assert!(moved_root.join("nested").is_dir());
        assert_eq!(
            fs::read_to_string(moved_root.join("nested").join("data.txt")).unwrap(),
            "payload"
        );
    }

    #[test]
    fn test_confirm_mkdir_uses_toast_and_focuses_new_directory() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        fs::create_dir_all(base.join("alpha")).unwrap();
        fs::create_dir_all(&base).unwrap();

        app.go_to_mount_point(base.clone());
        app.confirm_mkdir("new_dir".to_string(), base.clone());

        assert!(app.dialog.is_none());
        assert_eq!(app.toast_display(), Some("Directory 'new_dir' created."));
        assert_eq!(
            app.active_panel_state()
                .entries
                .get(app.active_panel_state().selected_index.saturating_sub(1))
                .map(|e| e.name.as_str()),
            Some("new_dir")
        );
    }

    #[test]
    fn test_confirm_rename_uses_toast_and_focuses_new_name() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let base = temp.path().join("base");
        fs::create_dir_all(&base).unwrap();
        let old = base.join("old_name");
        fs::write(&old, "x").unwrap();

        app.go_to_mount_point(base.clone());
        app.confirm_rename("new_name".to_string(), old);

        assert!(app.dialog.is_none());
        assert_eq!(app.toast_display(), Some("Rename completed"));
        assert_eq!(
            app.active_panel_state()
                .entries
                .get(app.active_panel_state().selected_index.saturating_sub(1))
                .map(|e| e.name.as_str()),
            Some("new_name")
        );
    }

    #[test]
    fn test_cancel_operation_uses_toast() {
        let mut app = make_test_app();
        let mut pending = PendingOperation::new(OperationType::Copy, Vec::new(), PathBuf::new());
        pending.start_processing(0, 3);
        pending.progress.files_completed = 1;
        app.pending_operation = Some(pending);
        app.dialog = Some(DialogKind::progress(
            app.pending_operation
                .as_ref()
                .expect("pending set")
                .progress
                .clone(),
        ));

        app.cancel_operation();

        assert!(app.dialog.is_none());
        assert_eq!(app.toast_display(), Some("Copy cancelled (1/3)"));
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_or_move_symlink_directory_fails_explicitly_and_continues() {
        let mut app = make_test_app();
        let temp = TempDir::new().unwrap();
        let src_root = temp.path().join("src_root");
        let target_dir = temp.path().join("target_dir");
        let dir_link = src_root.join("dir_link");
        let regular_file = src_root.join("regular.txt");
        let dest_root = temp.path().join("dest_root");

        fs::create_dir_all(&src_root).unwrap();
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("hidden.txt"), "target").unwrap();
        fs::write(&regular_file, "regular").unwrap();
        unix_fs::symlink(&target_dir, &dir_link).unwrap();
        fs::create_dir_all(&dest_root).unwrap();

        let mut pending = PendingOperation::new(
            OperationType::Copy,
            vec![dir_link.clone(), regular_file.clone()],
            dest_root.clone(),
        );
        app.prepare_and_start_operation(&mut pending, &dest_root);
        app.pending_operation = Some(pending);

        run_file_operation_until_done(&mut app);

        let dest_regular = dest_root.join("regular.txt");
        assert_eq!(fs::read_to_string(dest_regular).unwrap(), "regular");

        let error_text = match &app.dialog {
            Some(DialogKind::Error { message, .. }) => message.clone(),
            other => panic!("expected error dialog, got {:?}", other),
        };
        assert!(error_text.contains("Directory symlink is not supported"));
    }
}
