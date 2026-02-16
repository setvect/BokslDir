mod app;
mod core;
mod models;
mod system;
mod ui;
mod utils;

use app::App;
use core::actions::{find_action, Action};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::io;
use system::ime;
use ui::{
    ActivePanel, CommandBar, Dialog, DialogKind, DropdownMenu, LayoutMode, MenuBar, Panel,
    PanelStatus, StatusBar, WarningScreen,
};
use utils::{
    error::Result,
    formatter::{format_file_size, format_file_size_bytes},
};

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.area();

            // 레이아웃 업데이트
            app.layout.update(size);

            match app.layout.mode() {
                LayoutMode::TooSmall => {
                    // 경고 화면 표시
                    let (width, height) = app.layout.terminal_size();
                    let warning = WarningScreen::new().current_size(width, height);
                    f.render_widget(warning, size);
                }
                LayoutMode::DualPanel => {
                    render_main_ui(f, app);
                }
            }
        })?;

        // 파일 작업 진행 중일 때는 짧은 타임아웃으로 이벤트 체크
        let poll_timeout = if app.is_operation_processing() {
            std::time::Duration::from_millis(1)
        } else if app.pending_key.is_some() {
            std::time::Duration::from_millis(50)
        } else {
            std::time::Duration::from_millis(100)
        };

        // Handle events (작업 중에도 ESC 키 처리 가능)
        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.kind, KeyEventKind::Release) {
                    continue;
                }
                if app.is_dialog_active() {
                    // 다이얼로그 모드에서의 키 처리
                    handle_dialog_keys(app, key.modifiers, key.code);
                } else if app.is_menu_active() {
                    // 메뉴 모드에서의 키 처리
                    handle_menu_keys(app, key.modifiers, key.code);
                } else {
                    // 일반 모드에서의 키 처리
                    handle_normal_keys(app, key.modifiers, key.code);
                }
            }
        }

        // pending 키 타임아웃 체크
        if app.pending_key.is_some() && app.is_pending_key_expired() {
            app.clear_pending_key();
        }

        // 토스트 메시지 만료 체크
        app.clear_expired_toast();

        // IME 상태 폴링
        let new_ime = ime::get_current_ime();
        if new_ime != app.ime_status {
            app.ime_status = new_ime;
        }

        // 파일 작업 진행 중이면 다음 파일 처리
        if app.is_operation_processing() {
            if app.is_delete_operation() {
                app.process_next_delete();
            } else {
                app.process_next_file();
            }
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}

/// 일반 모드 키 처리 (액션 레지스트리 기반)
fn handle_normal_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    // 1) pending 키 시퀀스 처리 (gg, s+키, t+키)
    if let Some(pending) = app.pending_key {
        app.clear_pending_key();
        match (pending, &code) {
            ('g', KeyCode::Char('g')) => {
                app.execute_action(Action::GoToTop);
                return;
            }
            ('g', KeyCode::Char('m')) => {
                app.execute_action(Action::ShowMountPoints);
                return;
            }
            ('g', KeyCode::Char('p')) => {
                app.execute_action(Action::GoToPath);
                return;
            }
            ('s', KeyCode::Char('n')) => {
                app.execute_action(Action::SortByName);
                return;
            }
            ('s', KeyCode::Char('s')) => {
                app.execute_action(Action::SortBySize);
                return;
            }
            ('s', KeyCode::Char('d')) => {
                app.execute_action(Action::SortByDate);
                return;
            }
            ('s', KeyCode::Char('e')) => {
                app.execute_action(Action::SortByExt);
                return;
            }
            ('s', KeyCode::Char('r')) => {
                app.execute_action(Action::SortAscending);
                return;
            }
            ('t', KeyCode::Char('n')) => {
                app.execute_action(Action::TabNew);
                return;
            }
            ('t', KeyCode::Char('x')) => {
                app.execute_action(Action::TabClose);
                return;
            }
            ('t', KeyCode::Char('t')) => {
                app.execute_action(Action::ShowTabList);
                return;
            }
            ('t', KeyCode::Char('h')) => {
                app.execute_action(Action::ShowHistoryList);
                return;
            }
            ('t', KeyCode::Char('b')) => {
                app.execute_action(Action::ShowBookmarkList);
                return;
            }
            _ => {} // 잘못된 시퀀스, fall through
        }
    }

    // 2) 'g' 또는 's' 또는 't' 시작 시 시퀀스 모드 진입
    if modifiers == KeyModifiers::NONE
        && matches!(
            code,
            KeyCode::Char('g') | KeyCode::Char('s') | KeyCode::Char('t')
        )
    {
        if let KeyCode::Char(c) = code {
            app.set_pending_key(c);
        }
        return;
    }

    // 3) 테이블 조회 → 액션 실행
    if let Some(action) = find_action(modifiers, code) {
        app.execute_action(action);
    } else if let KeyCode::Char(c) = code {
        // 4) 한글 입력 감지: 액션 매칭 실패 + 한글 문자인 경우 경고
        if ('\u{AC00}'..='\u{D7A3}').contains(&c) || ('\u{3131}'..='\u{318E}').contains(&c) {
            app.show_message(
                "한글 입력 감지",
                "한영키를 눌러 영문 모드로 전환하세요.\n단축키는 영문 모드에서만 동작합니다.",
            );
        }
    }
}

/// 다이얼로그 모드 키 처리
fn handle_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    // 다이얼로그 종류에 따라 분기
    let dialog_kind = match &app.dialog {
        Some(kind) => kind.clone(),
        None => return,
    };

    match dialog_kind {
        DialogKind::Input { value, .. } => {
            handle_input_dialog_keys(app, modifiers, code, &value);
        }
        DialogKind::Confirm { .. } => {
            handle_confirm_dialog_keys(app, modifiers, code);
        }
        DialogKind::Conflict { .. } => {
            handle_conflict_dialog_keys(app, modifiers, code);
        }
        DialogKind::Progress { .. } => {
            handle_progress_dialog_keys(app, modifiers, code);
        }
        DialogKind::Error { .. } | DialogKind::Message { .. } => {
            handle_message_dialog_keys(app, modifiers, code);
        }
        DialogKind::DeleteConfirm { .. } => {
            handle_delete_confirm_dialog_keys(app, modifiers, code);
        }
        // Phase 3.4
        DialogKind::MkdirInput { .. } => {
            handle_mkdir_input_dialog_keys(app, modifiers, code);
        }
        DialogKind::RenameInput { .. } => {
            handle_rename_input_dialog_keys(app, modifiers, code);
        }
        DialogKind::Properties { .. } => {
            handle_message_dialog_keys(app, modifiers, code);
        }
        DialogKind::Help { .. } => {
            handle_help_dialog_keys(app, modifiers, code);
        }
        // Phase 5.2: 필터
        DialogKind::FilterInput { .. } => {
            handle_filter_input_dialog_keys(app, modifiers, code);
        }
        // Phase 5.3: 마운트 포인트
        DialogKind::MountPoints { .. } => {
            handle_mount_points_dialog_keys(app, code);
        }
        DialogKind::TabList { .. } => {
            handle_tab_list_dialog_keys(app, code);
        }
        DialogKind::HistoryList { .. } => {
            handle_history_list_dialog_keys(app, code);
        }
        DialogKind::BookmarkList { .. } => {
            handle_bookmark_list_dialog_keys(app, code);
        }
        DialogKind::BookmarkRenameInput { .. } => {
            handle_bookmark_rename_input_dialog_keys(app, modifiers, code);
        }
    }
}

/// 입력 다이얼로그 키 처리
fn handle_input_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode, _value: &str) {
    match (modifiers, code) {
        // 확인 (선택된 버튼에 따라 동작)
        (_, KeyCode::Enter) => {
            let selected_button = app.get_dialog_input_selected_button().unwrap_or(0);
            if selected_button == 0 {
                // OK 버튼
                if let Some(value) = app.get_dialog_input_value() {
                    app.confirm_input_dialog(value);
                }
            } else {
                // Cancel 버튼
                app.close_dialog();
            }
        }
        // 취소
        (_, KeyCode::Esc) => {
            app.close_dialog();
        }
        // 버튼 전환 (Tab / Shift+Tab)
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_input_toggle_button();
        }
        // 선택 추천 적용
        (KeyModifiers::NONE, KeyCode::Right) | (KeyModifiers::CONTROL, KeyCode::Char(' ')) => {
            app.dialog_input_apply_selected_completion();
        }
        // 추천 순환 + 즉시 완성
        (KeyModifiers::NONE, KeyCode::Down) => {
            app.dialog_input_cycle_completion_next();
        }
        (KeyModifiers::NONE, KeyCode::Up) => {
            app.dialog_input_cycle_completion_prev();
        }
        // 문자 입력
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.dialog_input_char(c);
        }
        // 백스페이스
        (_, KeyCode::Backspace) => {
            app.dialog_input_backspace();
        }
        // Delete
        (_, KeyCode::Delete) => {
            app.dialog_input_delete();
        }
        // 커서 이동
        (_, KeyCode::Left) => {
            app.dialog_input_left();
        }
        (_, KeyCode::Right) => {
            app.dialog_input_right();
        }
        (_, KeyCode::Home) => {
            app.dialog_input_home();
        }
        (_, KeyCode::End) => {
            app.dialog_input_end();
        }
        _ => {}
    }
}

/// 확인 다이얼로그 키 처리
fn handle_confirm_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        // 버튼 이동 (Tab / Shift+Tab)
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_confirm_toggle();
        }
        // 확인
        (_, KeyCode::Enter) => {
            if let Some(selected) = app.get_dialog_selected_button() {
                if selected == 0 {
                    // OK
                    if let Some(value) = app.get_dialog_input_value() {
                        app.confirm_input_dialog(value);
                    } else {
                        app.close_dialog();
                    }
                } else {
                    // Cancel
                    app.close_dialog();
                }
            }
        }
        // 취소
        (_, KeyCode::Esc) => {
            app.close_dialog();
        }
        _ => {}
    }
}

/// 충돌 다이얼로그 키 처리
fn handle_conflict_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        // 옵션 이동 (Tab: 다음, Shift+Tab: 이전)
        (KeyModifiers::NONE, KeyCode::Tab) => {
            app.dialog_conflict_next();
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_conflict_prev();
        }
        // 선택
        (_, KeyCode::Enter) => {
            if let Some(resolution) = app.get_dialog_conflict_option() {
                app.handle_conflict(resolution);
            }
        }
        // 취소
        (_, KeyCode::Esc) => {
            app.close_dialog();
        }
        _ => {}
    }
}

/// 진행률 다이얼로그 키 처리
fn handle_progress_dialog_keys(app: &mut App, _modifiers: KeyModifiers, code: KeyCode) {
    if code == KeyCode::Esc {
        app.cancel_operation();
    }
}

/// 삭제 확인 다이얼로그 키 처리
fn handle_delete_confirm_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        // 버튼 이동 (Tab: 다음, Shift+Tab: 이전)
        (KeyModifiers::NONE, KeyCode::Tab) => {
            app.dialog_delete_confirm_next();
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_delete_confirm_prev();
        }
        // 선택
        (_, KeyCode::Enter) => {
            if let Some(button) = app.get_delete_confirm_button() {
                match button {
                    0 => app.confirm_delete(true),  // 휴지통
                    1 => app.confirm_delete(false), // 영구 삭제
                    _ => app.close_dialog(),        // 취소
                }
            }
        }
        // 취소
        (_, KeyCode::Esc) => {
            app.close_dialog();
        }
        _ => {}
    }
}

/// 새 디렉토리 입력 다이얼로그 키 처리
fn handle_mkdir_input_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (_, KeyCode::Enter) => {
            let selected_button = app.get_mkdir_selected_button().unwrap_or(0);
            if selected_button == 0 {
                if let Some((value, parent_path)) = app.get_mkdir_input_value() {
                    app.confirm_mkdir(value, parent_path);
                }
            } else {
                app.close_dialog();
            }
        }
        (_, KeyCode::Esc) => app.close_dialog(),
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_mkdir_toggle_button();
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.dialog_mkdir_input_char(c);
        }
        (_, KeyCode::Backspace) => app.dialog_mkdir_input_backspace(),
        (_, KeyCode::Delete) => app.dialog_mkdir_input_delete(),
        (_, KeyCode::Left) => app.dialog_mkdir_input_left(),
        (_, KeyCode::Right) => app.dialog_mkdir_input_right(),
        (_, KeyCode::Home) => app.dialog_mkdir_input_home(),
        (_, KeyCode::End) => app.dialog_mkdir_input_end(),
        _ => {}
    }
}

/// 이름 변경 입력 다이얼로그 키 처리
fn handle_rename_input_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (_, KeyCode::Enter) => {
            let selected_button = app.get_rename_selected_button().unwrap_or(0);
            if selected_button == 0 {
                if let Some((value, original_path)) = app.get_rename_input_value() {
                    app.confirm_rename(value, original_path);
                }
            } else {
                app.close_dialog();
            }
        }
        (_, KeyCode::Esc) => app.close_dialog(),
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_rename_toggle_button();
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.dialog_rename_input_char(c);
        }
        (_, KeyCode::Backspace) => app.dialog_rename_input_backspace(),
        (_, KeyCode::Delete) => app.dialog_rename_input_delete(),
        (_, KeyCode::Left) => app.dialog_rename_input_left(),
        (_, KeyCode::Right) => app.dialog_rename_input_right(),
        (_, KeyCode::Home) => app.dialog_rename_input_home(),
        (_, KeyCode::End) => app.dialog_rename_input_end(),
        _ => {}
    }
}

/// 메시지/에러 다이얼로그 키 처리
fn handle_message_dialog_keys(app: &mut App, _modifiers: KeyModifiers, code: KeyCode) {
    match code {
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
            app.close_dialog();
        }
        _ => {}
    }
}

/// 도움말 다이얼로그 키 처리
fn handle_help_dialog_keys(app: &mut App, _modifiers: KeyModifiers, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.close_dialog();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.dialog_help_scroll_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.dialog_help_scroll_up();
        }
        _ => {}
    }
}

/// 마운트 포인트 다이얼로그 키 처리
fn handle_mount_points_dialog_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_dialog();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.mount_points_move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.mount_points_move_up();
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            app.mount_points_confirm();
        }
        _ => {}
    }
}

/// 탭 목록 다이얼로그 키 처리
fn handle_tab_list_dialog_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_dialog();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.tab_list_move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.tab_list_move_up();
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            app.tab_list_confirm();
        }
        _ => {}
    }
}

/// 히스토리 목록 다이얼로그 키 처리
fn handle_history_list_dialog_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_dialog();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.history_list_move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.history_list_move_up();
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            app.history_list_confirm();
        }
        KeyCode::Char('D') => {
            app.history_list_clear_all();
        }
        _ => {}
    }
}

/// 북마크 목록 다이얼로그 키 처리
fn handle_bookmark_list_dialog_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_dialog();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.bookmark_list_move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.bookmark_list_move_up();
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            app.bookmark_list_confirm();
        }
        KeyCode::Char('r') => {
            app.start_bookmark_rename_selected();
        }
        KeyCode::Char('d') => {
            app.bookmark_list_delete_selected();
        }
        _ => {}
    }
}

/// 북마크 이름 변경 입력 다이얼로그 키 처리
fn handle_bookmark_rename_input_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (_, KeyCode::Enter) => {
            let selected_button = app.get_bookmark_rename_selected_button().unwrap_or(0);
            if selected_button == 0 {
                if let Some((value, bookmark_index)) = app.get_bookmark_rename_input_value() {
                    app.confirm_bookmark_rename(value, bookmark_index);
                }
            } else {
                app.show_bookmark_list();
            }
        }
        (_, KeyCode::Esc) => app.show_bookmark_list(),
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_bookmark_rename_toggle_button();
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.dialog_bookmark_rename_input_char(c);
        }
        (_, KeyCode::Backspace) => app.dialog_bookmark_rename_input_backspace(),
        (_, KeyCode::Delete) => app.dialog_bookmark_rename_input_delete(),
        (_, KeyCode::Left) => app.dialog_bookmark_rename_input_left(),
        (_, KeyCode::Right) => app.dialog_bookmark_rename_input_right(),
        (_, KeyCode::Home) => app.dialog_bookmark_rename_input_home(),
        (_, KeyCode::End) => app.dialog_bookmark_rename_input_end(),
        _ => {}
    }
}

/// 필터 입력 다이얼로그 키 처리
fn handle_filter_input_dialog_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        (_, KeyCode::Enter) => {
            let selected_button = app.get_filter_selected_button().unwrap_or(0);
            if selected_button == 0 {
                // OK 버튼
                if let Some(value) = app.get_filter_input_value() {
                    app.confirm_filter(value);
                }
            } else {
                // Cancel 버튼 — 필터 해제
                app.cancel_filter();
            }
        }
        (_, KeyCode::Esc) => {
            app.cancel_filter();
        }
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            app.dialog_filter_toggle_button();
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.dialog_filter_input_char(c);
        }
        (_, KeyCode::Backspace) => app.dialog_filter_input_backspace(),
        (_, KeyCode::Delete) => app.dialog_filter_input_delete(),
        (_, KeyCode::Left) => app.dialog_filter_input_left(),
        (_, KeyCode::Right) => app.dialog_filter_input_right(),
        (_, KeyCode::Home) => app.dialog_filter_input_home(),
        (_, KeyCode::End) => app.dialog_filter_input_end(),
        _ => {}
    }
}

/// 메뉴 모드 키 처리
fn handle_menu_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        // 메뉴 닫기: Esc
        (_, KeyCode::Esc) => {
            if app.menu_state.submenu_open {
                app.close_submenu();
            } else {
                app.close_menu();
            }
        }
        // 메뉴 간 이동: Left/Right
        (_, KeyCode::Left) => {
            if app.menu_state.submenu_open {
                app.close_submenu();
            } else {
                app.prev_menu();
            }
        }
        (_, KeyCode::Right) => {
            // 서브메뉴가 열려있으면 닫고 다음 메뉴로 이동
            if app.menu_state.submenu_open {
                app.close_submenu();
            }
            app.next_menu();
        }
        // 항목 이동: Up/Down
        (_, KeyCode::Up) => app.prev_menu_item(),
        (_, KeyCode::Down) => app.next_menu_item(),
        // 항목 선택: Enter
        (_, KeyCode::Enter) => {
            // 서브메뉴가 있는 항목이면 서브메뉴 열기
            if let Some(menu) = app.menus.get(app.menu_state.selected_menu) {
                if let Some(item) = menu.items.get(app.menu_state.selected_item) {
                    if item.has_submenu() && !app.menu_state.submenu_open {
                        app.open_submenu();
                        return;
                    }
                }
            }
            // 액션이 있으면 실행
            if let Some(action_id) = app.get_selected_menu_action() {
                app.close_menu();
                app.execute_menu_action(&action_id);
            }
        }
        // 종료 단축키는 메뉴에서도 동작
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.quit(),
        (_, KeyCode::F(10)) => app.quit(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_t_b_sequence_opens_bookmark_flow() {
        let mut app = App::new_for_test();
        handle_normal_keys(&mut app, KeyModifiers::NONE, KeyCode::Char('t'));
        assert_eq!(app.pending_key, Some('t'));

        handle_normal_keys(&mut app, KeyModifiers::NONE, KeyCode::Char('b'));
        assert!(matches!(
            app.dialog,
            Some(DialogKind::BookmarkList { .. }) | Some(DialogKind::Message { .. })
        ));
    }

    #[test]
    fn test_g_p_sequence_opens_go_to_path_dialog() {
        let mut app = App::new_for_test();
        handle_normal_keys(&mut app, KeyModifiers::NONE, KeyCode::Char('g'));
        assert_eq!(app.pending_key, Some('g'));

        handle_normal_keys(&mut app, KeyModifiers::NONE, KeyCode::Char('p'));
        assert!(matches!(app.dialog, Some(DialogKind::Input { .. })));
    }

    #[test]
    fn test_bookmark_list_dialog_key_navigation_and_rename() {
        let mut app = App::new_for_test();
        app.dialog = Some(DialogKind::bookmark_list(
            vec![
                ("A".to_string(), std::path::PathBuf::from("/a")),
                ("B".to_string(), std::path::PathBuf::from("/b")),
            ],
            0,
        ));

        handle_bookmark_list_dialog_keys(&mut app, KeyCode::Char('j'));
        if let Some(DialogKind::BookmarkList { selected_index, .. }) = &app.dialog {
            assert_eq!(*selected_index, 1);
        } else {
            panic!("bookmark list dialog not shown");
        }

        handle_bookmark_list_dialog_keys(&mut app, KeyCode::Char('r'));
        assert!(matches!(
            app.dialog,
            Some(DialogKind::BookmarkRenameInput { .. })
        ));
    }

    #[test]
    fn test_input_dialog_tab_toggles_buttons() {
        let mut app = App::new_for_test();
        app.start_go_to_path();
        assert_eq!(app.get_dialog_input_selected_button(), Some(0));

        handle_input_dialog_keys(&mut app, KeyModifiers::NONE, KeyCode::Tab, "");
        assert_eq!(app.get_dialog_input_selected_button(), Some(1));
    }

    #[test]
    fn test_input_dialog_right_applies_selected_completion() {
        let mut app = App::new_for_test();
        app.dialog = Some(DialogKind::go_to_path_input(
            "",
            std::path::PathBuf::from("."),
        ));
        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            ..
        }) = &mut app.dialog
        {
            *completion_candidates = vec!["docs".to_string(), "downloads".to_string()];
            *completion_index = Some(0);
        }

        handle_input_dialog_keys(&mut app, KeyModifiers::NONE, KeyCode::Right, "");
        assert_eq!(app.get_dialog_input_value().as_deref(), Some("docs"));
    }

    #[test]
    fn test_input_dialog_up_down_cycles_completion() {
        let mut app = App::new_for_test();
        app.dialog = Some(DialogKind::go_to_path_input(
            "",
            std::path::PathBuf::from("."),
        ));
        if let Some(DialogKind::Input {
            completion_candidates,
            completion_index,
            ..
        }) = &mut app.dialog
        {
            *completion_candidates = vec!["alpha".to_string(), "beta".to_string()];
            *completion_index = Some(0);
        }

        handle_input_dialog_keys(&mut app, KeyModifiers::NONE, KeyCode::Down, "");
        assert_eq!(app.get_dialog_input_value().as_deref(), Some("beta"));

        handle_input_dialog_keys(&mut app, KeyModifiers::NONE, KeyCode::Up, "");
        assert_eq!(app.get_dialog_input_value().as_deref(), Some("alpha"));
    }

    #[test]
    fn test_input_dialog_j_k_stays_text_input() {
        let mut app = App::new_for_test();
        app.dialog = Some(DialogKind::go_to_path_input(
            "",
            std::path::PathBuf::from("."),
        ));

        handle_input_dialog_keys(&mut app, KeyModifiers::NONE, KeyCode::Char('j'), "");
        handle_input_dialog_keys(&mut app, KeyModifiers::NONE, KeyCode::Char('k'), "");

        assert_eq!(app.get_dialog_input_value().as_deref(), Some("jk"));
    }
}

/// 패널 위젯 생성 + 렌더링 (좌/우 공통)
#[allow(clippy::too_many_arguments)]
fn render_panel(
    f: &mut ratatui::Frame<'_>,
    panel_state: &crate::models::PanelState,
    tab_count: usize,
    is_active: bool,
    theme: &ui::Theme,
    area: Rect,
    icon_mode: ui::components::panel::IconMode,
    size_format: app::SizeFormat,
) {
    let path = panel_state.current_path.to_string_lossy();
    let show_parent = panel_state.current_path.parent().is_some();
    let panel = Panel::new()
        .title(&path)
        .tab_count(tab_count)
        .status(if is_active {
            PanelStatus::Active
        } else {
            PanelStatus::Inactive
        })
        .entries(&panel_state.entries)
        .selected_index(panel_state.selected_index)
        .scroll_offset(panel_state.scroll_offset)
        .show_parent(show_parent)
        .selected_items(&panel_state.selected_items)
        .icon_mode(icon_mode)
        .sort_state(panel_state.sort_by, panel_state.sort_order)
        .filter_pattern(panel_state.filter.as_deref())
        .size_format(size_format)
        .theme(theme);
    f.render_widget(panel, area);
}

/// 상태바 데이터 수집 + 렌더링
fn render_status_bar(f: &mut ratatui::Frame<'_>, app: &App, theme: &ui::Theme, area: Rect) {
    let active_panel_state = app.active_panel_state();
    let file_count = active_panel_state.file_count();
    let dir_count = active_panel_state.dir_count();
    let total_size = match app.size_format {
        app::SizeFormat::Auto => format_file_size(active_panel_state.total_size()),
        app::SizeFormat::Bytes => format_file_size_bytes(active_panel_state.total_size()),
    };
    let selected_count = active_panel_state.selected_count();
    let selected_size = match app.size_format {
        app::SizeFormat::Auto => format_file_size(active_panel_state.selected_size()),
        app::SizeFormat::Bytes => format_file_size_bytes(active_panel_state.selected_size()),
    };

    let pending_display = app.pending_key_display();
    let toast_display = app.toast_display().map(|s| s.to_string());
    let sort_display = active_panel_state.sort_indicator();
    let filter_display = active_panel_state.filter_indicator();
    let ime_label = app.ime_status.display_label();
    let status_bar = StatusBar::new()
        .file_count(file_count)
        .dir_count(dir_count)
        .total_size(&total_size)
        .selected_count(selected_count)
        .selected_size(&selected_size)
        .layout_mode(app.layout_mode_str())
        .pending_key(pending_display.as_deref())
        .toast(toast_display.as_deref())
        .sort_info(Some(&sort_display))
        .filter_info(filter_display.as_deref())
        .show_hidden(active_panel_state.show_hidden)
        .ime_info(if app.ime_status.should_display() {
            Some(ime_label)
        } else {
            None
        })
        .theme(theme);
    f.render_widget(status_bar, area);
}

/// 메뉴 드롭다운 조건부 렌더링
fn render_dropdown_if_active(
    f: &mut ratatui::Frame<'_>,
    app: &App,
    theme: &ui::Theme,
    menu_bar_area: Rect,
) {
    if !app.is_menu_active() {
        return;
    }
    if let Some(menu) = app.menus.get(app.menu_state.selected_menu) {
        let menu_bar_widget = MenuBar::new().menus(&app.menus);
        let menu_x = menu_bar_widget.get_menu_x_position(app.menu_state.selected_menu);

        let dropdown_area = Rect {
            x: menu_x,
            y: menu_bar_area.y + 1,
            width: f.area().width.saturating_sub(menu_x),
            height: f.area().height.saturating_sub(menu_bar_area.y + 1),
        };

        let dropdown = DropdownMenu::new(menu, &app.menu_state).theme(theme);
        f.render_widget(dropdown, dropdown_area);
    }
}

/// 메인 UI 렌더링
fn render_main_ui(f: &mut ratatui::Frame<'_>, app: &App) {
    let areas = app.layout.areas();
    let active_panel = app.layout.active_panel();
    let theme = app.theme_manager.current();

    let menu_bar = MenuBar::new()
        .menus(&app.menus)
        .menu_active(app.is_menu_active())
        .selected_menu(app.menu_state.selected_menu)
        .theme(theme);
    f.render_widget(menu_bar, areas.menu_bar);

    let left_tab_count = app.panel_tab_count(ActivePanel::Left);
    let right_tab_count = app.panel_tab_count(ActivePanel::Right);

    render_panel(
        f,
        app.left_active_panel_state(),
        left_tab_count,
        active_panel == ActivePanel::Left,
        theme,
        areas.left_panel,
        app.icon_mode,
        app.size_format,
    );

    if app.layout.is_dual_panel() {
        render_panel(
            f,
            app.right_active_panel_state(),
            right_tab_count,
            active_panel == ActivePanel::Right,
            theme,
            areas.right_panel,
            app.icon_mode,
            app.size_format,
        );
    }

    render_status_bar(f, app, theme, areas.status_bar);

    let command_bar = CommandBar::new().theme(theme);
    f.render_widget(command_bar, areas.command_bar);

    render_dropdown_if_active(f, app, theme, areas.menu_bar);

    if let Some(ref dialog_kind) = app.dialog {
        let dialog = Dialog::new(dialog_kind).theme(theme);
        f.render_widget(dialog, f.area());
    }
}
