use super::controllers;
use super::text_edit::TextBufferEdit;
use super::*;

impl App {
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
            Action::ShowHelp
            | Action::Refresh
            | Action::OpenMenu
            | Action::ThemeDark
            | Action::ThemeLight
            | Action::ThemeContrast
            | Action::ToggleIconMode
            | Action::SetDefaultEditorVi
            | Action::SetDefaultEditorVim
            | Action::SetDefaultEditorNano
            | Action::SetDefaultEditorEmacs
            | Action::About => controllers::dialog_controller::execute(self, action),
            Action::Copy
            | Action::Move
            | Action::OpenDefaultApp
            | Action::OpenTerminalEditor
            | Action::Delete
            | Action::PermanentDelete
            | Action::MakeDirectory
            | Action::Rename
            | Action::ShowProperties
            | Action::ArchiveCompress
            | Action::ArchiveExtract
            | Action::ArchiveExtractAuto
            | Action::ArchivePreview
            | Action::ToggleSelection
            | Action::InvertSelection
            | Action::SelectAll
            | Action::DeselectAll
            | Action::SortByName
            | Action::SortBySize
            | Action::SortByDate
            | Action::SortByExt
            | Action::SortAscending
            | Action::SortDescending
            | Action::StartFilter
            | Action::ClearFilter
            | Action::ToggleHidden
            | Action::ShowMountPoints
            | Action::GoToPath
            | Action::ShowTabList
            | Action::HistoryBack
            | Action::HistoryForward
            | Action::ShowHistoryList
            | Action::AddBookmark
            | Action::ShowBookmarkList
            | Action::SizeFormatAuto
            | Action::SizeFormatBytes => controllers::operation_controller::execute(self, action),
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
    pub(super) fn sort_active_panel(&mut self, sort_by: SortBy) {
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
    pub(super) fn toggle_sort_order(&mut self) {
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
    pub(super) fn re_sort_active_panel(&mut self) {
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
    pub(super) fn change_active_dir(
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
            let _ = self.save_persisted_state();
        }

        result.is_ok()
    }

    /// 상위 디렉토리로 이동 (h / Left)
    pub fn go_to_parent(&mut self) {
        if self.archive_view_go_parent() {
            return;
        }
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

    pub(super) fn format_user_error(
        action: &str,
        path: Option<&Path>,
        error: &str,
        hint: &str,
    ) -> String {
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

    pub(super) fn focus_active_entry_by_name(&mut self, name: &str) -> bool {
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
            TextBufferEdit::insert_char(search_query, search_cursor, c);
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
            TextBufferEdit::backspace(search_query, search_cursor);
            *scroll_offset = 0;
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
            TextBufferEdit::delete_prev_word(search_query, search_cursor);
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
            TextBufferEdit::delete(search_query, search_cursor);
            *scroll_offset = 0;
        }
    }

    pub fn dialog_help_cursor_left(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            ..
        }) = &mut self.dialog
        {
            TextBufferEdit::left(search_query, search_cursor);
        }
    }

    pub fn dialog_help_cursor_right(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            ..
        }) = &mut self.dialog
        {
            TextBufferEdit::right(search_query, search_cursor);
        }
    }

    pub fn dialog_help_cursor_home(&mut self) {
        if let Some(DialogKind::Help { search_cursor, .. }) = &mut self.dialog {
            TextBufferEdit::home(search_cursor);
        }
    }

    pub fn dialog_help_cursor_end(&mut self) {
        if let Some(DialogKind::Help {
            search_query,
            search_cursor,
            ..
        }) = &mut self.dialog
        {
            TextBufferEdit::end(search_query, search_cursor);
        }
    }

    /// 최대 인덱스 계산
    pub(super) fn get_max_index(&self) -> usize {
        let panel = self.active_panel_state();
        let has_parent = panel.current_path.parent().is_some();

        if has_parent {
            panel.entries.len()
        } else {
            panel.entries.len().saturating_sub(1)
        }
    }

    /// 페이지 크기 계산 (화면에 표시되는 항목 수)
    pub(super) fn get_page_size(&self) -> usize {
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
    pub(super) fn adjust_scroll_offset(&mut self) {
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
    pub(super) fn navigate_to_parent(&mut self, current_path: &std::path::Path) {
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
    pub(super) fn enter_directory(&mut self, path: PathBuf) {
        let _ = self.change_active_dir(path, true, None);
    }

    /// Enter 키 처리: 디렉토리 진입 / 상위 디렉토리 이동 / 압축 파일 미리보기
    pub fn enter_selected(&mut self) {
        if self.archive_view_enter_selected() {
            return;
        }
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

        if let Some((is_dir, path)) = entry_info {
            if is_dir {
                self.enter_directory(path);
            } else if detect_archive_format(&path).is_some() {
                self.start_archive_preview();
            }
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
}
