#![allow(dead_code)]

use crate::core::actions::Action;
use crate::models::operation::{
    ConflictResolution, FlattenedEntryKind, FlattenedFile, OperationProgress, OperationState,
    OperationType, PendingOperation,
};
use crate::models::panel_state::{SortBy, SortOrder};
use crate::models::{FileEntry, PanelState, PanelTabs};
use crate::system::{
    create_archive, detect_archive_format, extract_archive, list_entries, list_extract_conflicts,
    supports_password, ArchiveCreateRequest, ArchiveEntry, ArchiveExtractRequest, ArchiveFormat,
    ArchiveProgressEvent, ArchiveSummary, FileSystem, ImeStatus,
};
use crate::ui::{
    create_default_menus, ActivePanel, DialogKind, InputPurpose, LayoutManager, LayoutMode, Menu,
    MenuState, ThemeManager,
};
use crate::utils::error::{BokslDirError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

mod controllers;
mod dialogs;
mod navigation;
mod operations;
mod text_edit;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedPanelHistory {
    entries: Vec<PathBuf>,
    index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedHistoriesState {
    left: PersistedPanelHistory,
    right: PersistedPanelHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedBookmark {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedAppState {
    version: u32,
    theme: String,
    history: PersistedHistoriesState,
    bookmarks: Vec<PersistedBookmark>,
}

#[derive(Debug, Clone)]
pub struct TerminalEditorRequest {
    pub editor_command: String,
    pub target_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveWorkerKind {
    Compress,
    Extract,
}

#[derive(Debug)]
struct ArchiveWorkerState {
    kind: ArchiveWorkerKind,
    progress_rx: Receiver<ArchiveProgressEvent>,
    join_handle:
        Option<JoinHandle<std::result::Result<ArchiveSummary, crate::utils::error::BokslDirError>>>,
    cancel_flag: Arc<AtomicBool>,
    progress: OperationProgress,
}

#[derive(Debug, Clone)]
enum ArchiveFlowContext {
    CreatePending {
        sources: Vec<PathBuf>,
    },
    ExtractPending {
        archive_path: PathBuf,
        format: ArchiveFormat,
    },
    ExtractNeedsPassword {
        request: ArchiveExtractRequest,
    },
    ExtractConflictPrompt {
        request: ArchiveExtractRequest,
        conflicts: Vec<String>,
        current_index: usize,
    },
    ExtractAutoNeedsPassword {
        archive_path: PathBuf,
        base_dir: PathBuf,
    },
    PreviewNeedsPassword {
        archive_path: PathBuf,
        panel: ActivePanel,
    },
    CopyFromPanel {
        view: ArchivePanelView,
        selected_entries: Vec<FileEntry>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelSlot {
    Left,
    Right,
}

impl From<ActivePanel> for PanelSlot {
    fn from(value: ActivePanel) -> Self {
        match value {
            ActivePanel::Left => PanelSlot::Left,
            ActivePanel::Right => PanelSlot::Right,
        }
    }
}

#[derive(Debug, Clone)]
struct ArchivePanelView {
    panel: PanelSlot,
    archive_path: PathBuf,
    base_dir: PathBuf,
    current_dir: String,
    all_entries: Vec<ArchiveEntry>,
    password: Option<String>,
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
    /// 진행 중인 압축 작업 워커
    archive_worker: Option<ArchiveWorkerState>,
    /// 압축 관련 다이얼로그 흐름 상태
    archive_flow: Option<ArchiveFlowContext>,
    /// 압축 패널 탐색 상태 (활성 패널 기준)
    archive_panel_view: Option<ArchivePanelView>,
    /// 압축 내부 복사용 임시 디렉토리 (작업 종료/취소 시 정리)
    archive_copy_temp_dir: Option<PathBuf>,
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
    /// 테스트에서 설정 저장 경로를 격리하기 위한 override
    state_store_override: Option<PathBuf>,
}

impl App {
    const MAX_TABS_PER_PANEL: usize = 5;
    const APP_STATE_VERSION: u32 = 1;
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
            archive_worker: None,
            archive_flow: None,
            archive_panel_view: None,
            archive_copy_temp_dir: None,
            pending_key: None,
            pending_key_time: None,
            toast_message: None,
            icon_mode: crate::ui::components::panel::IconMode::default(),
            size_format: SizeFormat::default(),
            ime_status: crate::system::get_current_ime(),
            default_terminal_editor: Self::resolve_default_terminal_editor_from_env(),
            pending_terminal_editor_request: None,
            bookmarks: Vec::new(),
            state_store_override: None,
        };
        app.load_persisted_state();
        Ok(app)
    }

    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static TEST_APP_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let current_dir = std::path::PathBuf::from(".");
        let suffix = TEST_APP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let state_store_override = std::env::temp_dir().join(format!(
            "boksldir-test-settings-{}-{}.toml",
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
            archive_worker: None,
            archive_flow: None,
            archive_panel_view: None,
            archive_copy_temp_dir: None,
            pending_key: None,
            pending_key_time: None,
            toast_message: None,
            icon_mode: crate::ui::components::panel::IconMode::default(),
            size_format: SizeFormat::default(),
            ime_status: ImeStatus::Unknown,
            default_terminal_editor: Self::FALLBACK_TERMINAL_EDITOR.to_string(),
            pending_terminal_editor_request: None,
            bookmarks: Vec::new(),
            state_store_override: Some(state_store_override),
        }
    }

    /// 종료
    pub fn quit(&mut self) {
        let _ = self.save_persisted_state();
        self.should_quit = true;
    }

    fn state_store_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.state_store_override {
            return Some(path.clone());
        }
        if let Ok(custom) = env::var("BOKSLDIR_SETTINGS_FILE") {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                return Some(PathBuf::from(trimmed));
            }
        }
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".boksldir").join("settings.toml"))
    }

    fn encode_app_state(&self) -> std::result::Result<String, toml::ser::Error> {
        let left = self.left_tabs.active();
        let right = self.right_tabs.active();
        let payload = PersistedAppState {
            version: Self::APP_STATE_VERSION,
            theme: self.current_theme_name().to_string(),
            history: PersistedHistoriesState {
                left: PersistedPanelHistory {
                    entries: left.history_entries.clone(),
                    index: left.history_index,
                },
                right: PersistedPanelHistory {
                    entries: right.history_entries.clone(),
                    index: right.history_index,
                },
            },
            bookmarks: self.bookmarks.clone(),
        };
        toml::to_string_pretty(&payload)
    }

    fn decode_app_state(data: &str) -> Option<PersistedAppState> {
        let parsed: PersistedAppState = toml::from_str(data).ok()?;
        if parsed.version != Self::APP_STATE_VERSION {
            return None;
        }
        if parsed.theme.trim().is_empty() {
            return None;
        }
        Some(parsed)
    }

    fn save_persisted_state(&self) -> std::io::Result<()> {
        let Some(path) = self.state_store_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = self
            .encode_app_state()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, data)
    }

    fn load_persisted_state(&mut self) {
        let Some(path) = self.state_store_path() else {
            return;
        };
        let Ok(data) = fs::read_to_string(path) else {
            return;
        };
        let Some(state) = Self::decode_app_state(&data) else {
            return;
        };

        self.apply_loaded_history(
            ActivePanel::Left,
            state.history.left.entries,
            state.history.left.index,
        );
        self.apply_loaded_history(
            ActivePanel::Right,
            state.history.right.entries,
            state.history.right.index,
        );
        self.bookmarks = state.bookmarks;
        let _ = self.theme_manager.switch_theme(&state.theme);
    }

    fn current_theme_name(&self) -> &str {
        self.theme_manager.current_name()
    }

    fn switch_theme_and_save(&mut self, theme_name: &str) {
        if self.theme_manager.switch_theme(theme_name).is_ok() {
            let _ = self.save_persisted_state();
        }
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
                archive_worker: None,
                archive_flow: None,
                archive_panel_view: None,
                archive_copy_temp_dir: None,
                pending_key: None,
                pending_key_time: None,
                toast_message: None,
                icon_mode: crate::ui::components::panel::IconMode::default(),
                size_format: SizeFormat::default(),
                ime_status: ImeStatus::Unknown,
                default_terminal_editor: Self::FALLBACK_TERMINAL_EDITOR.to_string(),
                pending_terminal_editor_request: None,
                bookmarks: Vec::new(),
                state_store_override: None,
            }
        })
    }
}

#[cfg(test)]
mod tests;
