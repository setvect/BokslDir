#![allow(dead_code)]

use crate::models::operation::{
    ConflictResolution, FlattenedFile, OperationState, OperationType, PendingOperation,
};
use crate::models::PanelState;
use crate::system::FileSystem;
use crate::ui::{
    create_default_menus, ActivePanel, DialogKind, LayoutManager, LayoutMode, Menu, MenuState,
    ThemeManager,
};
use crate::utils::error::Result;
use std::env;
use std::path::PathBuf;

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
    // Phase 3.2: 파일 복사/이동
    /// 현재 표시 중인 다이얼로그
    pub dialog: Option<DialogKind>,
    /// 대기 중인 파일 작업
    pub pending_operation: Option<PendingOperation>,
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
            dialog: None,
            pending_operation: None,
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

            // 선택 관련 (Phase 3.1)
            "select_all" => self.select_all(),
            "invert_selection" => self.invert_selection(),
            "deselect" => self.deselect_all(),

            // 파일 복사/이동 (Phase 3.2)
            "copy" => self.start_copy(),
            "move" => self.start_move(),

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
            panel_mut.scroll_offset =
                entries_selected.saturating_sub(available_height as usize - 1);
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
            ActivePanel::Left => &self.right_panel,
            ActivePanel::Right => &self.left_panel,
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

            // 취소 메시지 표시
            let msg = format!(
                "{} cancelled.\nCompleted: {} / {} files",
                pending.operation_type.name(),
                pending.progress.files_completed,
                pending.progress.total_files
            );
            self.dialog = Some(DialogKind::message("Cancelled", msg));
        } else {
            self.close_dialog();
        }
    }

    /// 복사 시작 (F5)
    pub fn start_copy(&mut self) {
        self.start_file_operation(OperationType::Copy);
    }

    /// 이동 시작 (F6)
    pub fn start_move(&mut self) {
        self.start_file_operation(OperationType::Move);
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
        self.pending_operation = Some(PendingOperation::new(operation_type, sources, dest_dir));

        // 입력 다이얼로그 표시
        let title = operation_type.name();
        let prompt = format!("{} to:", title);
        self.dialog = Some(DialogKind::input(title, prompt, dest_path));
    }

    /// 입력 다이얼로그에서 확인 처리
    pub fn confirm_input_dialog(&mut self, dest_path_str: String) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        let dest_path = PathBuf::from(&dest_path_str);

        // 대상 경로가 디렉토리인지 확인
        if !dest_path.exists() {
            self.dialog = Some(DialogKind::error(
                "Error",
                format!("Destination path does not exist:\n{}", dest_path_str),
            ));
            // pending 복원
            self.pending_operation = Some(pending);
            return;
        }

        if !dest_path.is_dir() {
            self.dialog = Some(DialogKind::error(
                "Error",
                format!("Destination is not a directory:\n{}", dest_path_str),
            ));
            // pending 복원
            self.pending_operation = Some(pending);
            return;
        }

        // 재귀 복사/이동 방지: 디렉토리를 자기 자신 내부로 복사/이동하려는지 미리 확인
        if let Some(error_msg) =
            Self::check_recursive_operation(&pending.sources, pending.operation_type, &dest_path)
        {
            self.dialog = Some(DialogKind::error("Error", error_msg));
            // pending 복원
            self.pending_operation = Some(pending);
            return;
        }

        // 대상 경로 업데이트
        pending.dest_dir = dest_path.clone();

        // 소스를 평탄화하여 개별 파일 목록 생성
        let flattened = match self.filesystem.flatten_sources(&pending.sources, &dest_path) {
            Ok(files) => files
                .into_iter()
                .map(|(source, dest, size)| FlattenedFile { source, dest, size })
                .collect::<Vec<_>>(),
            Err(e) => {
                self.dialog = Some(DialogKind::error(
                    "Error",
                    format!("Failed to scan files: {}", e),
                ));
                self.pending_operation = Some(pending);
                return;
            }
        };

        // 전체 크기 및 파일 수 계산
        let total_bytes: u64 = flattened.iter().map(|f| f.size).sum();
        let total_files = flattened.len();

        // 평탄화된 파일 목록 설정
        pending.set_flattened_files(flattened);

        // 처리 시작
        pending.start_processing(total_bytes, total_files);

        // Progress 다이얼로그 표시
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
        self.pending_operation = Some(pending);
    }

    /// 진행 중인 작업 여부 확인
    pub fn is_operation_processing(&self) -> bool {
        self.pending_operation
            .as_ref()
            .is_some_and(|p| p.state == OperationState::Processing)
    }

    /// 다음 파일 처리 (메인 루프에서 호출)
    ///
    /// 한 번에 하나의 파일만 처리하고 반환하여 UI 업데이트 가능
    pub fn process_next_file(&mut self) {
        let Some(mut pending) = self.pending_operation.take() else {
            self.close_dialog();
            return;
        };

        // 처리 중 상태가 아니면 반환
        if pending.state != OperationState::Processing {
            self.pending_operation = Some(pending);
            return;
        }

        // 모든 파일 처리 완료 확인
        if pending.is_all_processed() {
            self.finish_operation(pending);
            return;
        }

        let operation_type = pending.operation_type;

        // 충돌 해결 방법 확인
        let skip_all = pending
            .conflict_resolution
            .is_some_and(|r| r == ConflictResolution::SkipAll);
        let overwrite_all = pending
            .conflict_resolution
            .is_some_and(|r| r == ConflictResolution::OverwriteAll);

        // 현재 파일 처리 (flattened_files 사용)
        let file_entry = pending.flattened_files[pending.current_index].clone();
        let source = file_entry.source;
        let dest_path = file_entry.dest;

        let file_name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // 진행 상태 업데이트
        pending.set_current_file(&file_name);
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));

        // 소스와 대상이 동일한지 확인
        if source == dest_path {
            pending.add_error(format!("Source and destination are the same: {:?}", source));
            pending.file_skipped();
            pending.current_index += 1;
            self.pending_operation = Some(pending);
            return;
        }

        // 대상 파일이 이미 존재하는지 확인
        if dest_path.exists() {
            if skip_all {
                pending.file_skipped();
                pending.current_index += 1;
                self.pending_operation = Some(pending);
                return;
            }
            if !overwrite_all {
                // 충돌 다이얼로그 표시
                pending.state = OperationState::WaitingConflict;
                self.dialog = Some(DialogKind::conflict(source, dest_path));
                self.pending_operation = Some(pending);
                return;
            }
            // overwrite_all이면 대상을 먼저 삭제
            let _ = std::fs::remove_file(&dest_path);
        }

        // 대상 디렉토리 생성 (필요시)
        if let Some(parent) = dest_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // 파일 복사 또는 이동
        let result = match operation_type {
            OperationType::Copy => self.filesystem.copy_file(&source, &dest_path),
            OperationType::Move => self.filesystem.move_file(&source, &dest_path),
        };

        match result {
            Ok(bytes) => pending.files_completed(bytes, 1),
            Err(e) => {
                pending.add_error(format!("{}: {}", file_name, e));
                pending.file_skipped();
            }
        }

        pending.current_index += 1;

        // 진행 상태 업데이트
        self.dialog = Some(DialogKind::progress(pending.progress.clone()));
        self.pending_operation = Some(pending);
    }

    /// 작업 완료 처리
    fn finish_operation(&mut self, pending: PendingOperation) {
        // 패널 새로고침
        self.refresh_both_panels();

        // 결과 표시
        if pending.errors.is_empty() {
            self.dialog = Some(DialogKind::message(
                "Complete",
                format!(
                    "{} completed: {} file(s)",
                    pending.operation_type.name(),
                    pending.completed_count
                ),
            ));
        } else {
            let error_msg = format!(
                "{} completed with errors:\n{} succeeded, {} failed\n\nErrors:\n{}",
                pending.operation_type.name(),
                pending.completed_count,
                pending.errors.len(),
                pending.errors.join("\n")
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

    /// 충돌 해결 처리
    pub fn handle_conflict(&mut self, resolution: ConflictResolution) {
        match resolution {
            ConflictResolution::Cancel => {
                // 작업 취소 - 현재까지 결과 표시
                if let Some(pending) = self.pending_operation.take() {
                    self.finish_operation(pending);
                } else {
                    self.close_dialog();
                }
            }
            ConflictResolution::Overwrite => {
                // 현재 파일만 덮어쓰기: 대상 파일을 먼저 삭제
                if let Some(DialogKind::Conflict { dest_path, .. }) = &self.dialog {
                    let dest = dest_path.clone();
                    if dest.is_dir() {
                        let _ = std::fs::remove_dir_all(&dest);
                    } else {
                        let _ = std::fs::remove_file(&dest);
                    }
                }
                // 처리 상태로 전환하여 다음 프레임에 계속 진행
                self.execute_file_operation();
            }
            ConflictResolution::Skip => {
                // 현재 파일만 건너뛰고 계속 진행
                if let Some(pending) = self.pending_operation.as_mut() {
                    pending.file_skipped();
                    pending.current_index += 1;
                }
                self.execute_file_operation();
            }
            ConflictResolution::OverwriteAll => {
                // 현재 파일 덮어쓰기 + 이후 모든 충돌도 덮어쓰기
                if let Some(DialogKind::Conflict { dest_path, .. }) = &self.dialog {
                    let dest = dest_path.clone();
                    if dest.is_dir() {
                        let _ = std::fs::remove_dir_all(&dest);
                    } else {
                        let _ = std::fs::remove_file(&dest);
                    }
                }
                if let Some(pending) = self.pending_operation.as_mut() {
                    pending.conflict_resolution = Some(ConflictResolution::OverwriteAll);
                }
                self.execute_file_operation();
            }
            ConflictResolution::SkipAll => {
                // 현재 파일 건너뛰기 + 이후 모든 충돌도 건너뛰기
                if let Some(pending) = self.pending_operation.as_mut() {
                    pending.file_skipped();
                    pending.current_index += 1;
                    pending.conflict_resolution = Some(ConflictResolution::SkipAll);
                }
                self.execute_file_operation();
            }
        }
    }

    /// 양쪽 패널 새로고침
    pub fn refresh_both_panels(&mut self) {
        let _ = self.left_panel.refresh(&self.filesystem);
        let _ = self.right_panel.refresh(&self.filesystem);
    }

    // === 다이얼로그 입력 처리 메서드 ===

    /// 입력 다이얼로그: 문자 입력
    pub fn dialog_input_char(&mut self, c: char) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            value.insert(*cursor_pos, c);
            *cursor_pos += 1;
        }
    }

    /// 입력 다이얼로그: 백스페이스
    pub fn dialog_input_backspace(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos > 0 {
                value.remove(*cursor_pos - 1);
                *cursor_pos -= 1;
            }
        }
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
    }

    /// 입력 다이얼로그: 커서 왼쪽
    pub fn dialog_input_left(&mut self) {
        if let Some(DialogKind::Input { cursor_pos, .. }) = &mut self.dialog {
            *cursor_pos = cursor_pos.saturating_sub(1);
        }
    }

    /// 입력 다이얼로그: 커서 오른쪽
    pub fn dialog_input_right(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            if *cursor_pos < value.len() {
                *cursor_pos += 1;
            }
        }
    }

    /// 입력 다이얼로그: Home
    pub fn dialog_input_home(&mut self) {
        if let Some(DialogKind::Input { cursor_pos, .. }) = &mut self.dialog {
            *cursor_pos = 0;
        }
    }

    /// 입력 다이얼로그: End
    pub fn dialog_input_end(&mut self) {
        if let Some(DialogKind::Input {
            value, cursor_pos, ..
        }) = &mut self.dialog
        {
            *cursor_pos = value.len();
        }
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
                left_panel: PanelState::new(current_dir.clone()),
                right_panel: PanelState::new(current_dir),
                filesystem,
                menus: create_default_menus(),
                menu_state: MenuState::new(),
                theme_manager: ThemeManager::new(),
                dialog: None,
                pending_operation: None,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
}
