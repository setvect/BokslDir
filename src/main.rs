mod app;
mod core;
mod models;
mod system;
mod ui;
mod utils;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::io;
use ui::{
    ActivePanel, CommandBar, Dialog, DialogKind, DropdownMenu, LayoutMode, MenuBar, Panel,
    PanelStatus, StatusBar, WarningScreen,
};
use utils::{error::Result, formatter::format_file_size};

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
        } else {
            std::time::Duration::from_millis(100)
        };

        // Handle events (작업 중에도 ESC 키 처리 가능)
        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
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

/// 일반 모드 키 처리
fn handle_normal_keys(app: &mut App, modifiers: KeyModifiers, code: KeyCode) {
    match (modifiers, code) {
        // 종료: q, Esc, F10, Ctrl+C
        (_, KeyCode::Char('q')) => app.quit(),
        (_, KeyCode::F(10)) => app.quit(),
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => app.quit(),
        // 패널 전환: Tab
        (_, KeyCode::Tab) => app.toggle_panel(),
        // 메뉴 활성화: F9
        (_, KeyCode::F(9)) => app.open_menu(),
        // 파일 탐색 (Phase 2.3)
        (_, KeyCode::Up) => app.move_selection_up(),
        (_, KeyCode::Down) => app.move_selection_down(),
        (_, KeyCode::PageUp) => app.move_selection_page_up(),
        (_, KeyCode::PageDown) => app.move_selection_page_down(),
        (KeyModifiers::NONE, KeyCode::Enter) => app.enter_selected(),
        // 다중 선택 (Phase 3.1)
        (KeyModifiers::NONE, KeyCode::Char(' ')) => app.toggle_selection_and_move_down(),
        (KeyModifiers::NONE, KeyCode::Char('*')) => app.select_all(), // PRD: * 또는 Ctrl+A
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => app.select_all(),
        (KeyModifiers::NONE, KeyCode::Char('+')) => app.invert_selection(), // 선택 반전
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => app.deselect_all(),
        // 파일 복사/이동 (Phase 3.2)
        (_, KeyCode::F(5)) => app.start_copy(),
        (_, KeyCode::F(6)) => app.start_move(),
        // 파일 삭제 (Phase 3.3)
        (_, KeyCode::F(8)) => app.start_delete(),
        // Phase 3.4: 기타 파일 작업
        (_, KeyCode::F(7)) => app.start_mkdir(),
        (_, KeyCode::F(2)) => app.start_rename(),
        (KeyModifiers::ALT, KeyCode::Enter) => app.show_properties(),
        // Esc는 아무것도 안 함 (메뉴가 닫혀있을 때)
        (_, KeyCode::Esc) => {}
        _ => {}
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

/// 메인 UI 렌더링
fn render_main_ui(f: &mut ratatui::Frame<'_>, app: &App) {
    let areas = app.layout.areas();
    let active_panel = app.layout.active_panel();
    let theme = app.theme_manager.current();

    // 메뉴바 렌더링
    let menu_bar = MenuBar::new()
        .menus(&app.menus)
        .menu_active(app.is_menu_active())
        .selected_menu(app.menu_state.selected_menu)
        .theme(theme);
    f.render_widget(menu_bar, areas.menu_bar);

    // 좌측 패널 렌더링
    let left_path = app.left_panel.current_path.to_string_lossy();
    let show_parent_left = app.left_panel.current_path.parent().is_some();
    let left_panel = Panel::new()
        .title(&left_path)
        .status(if active_panel == ActivePanel::Left {
            PanelStatus::Active
        } else {
            PanelStatus::Inactive
        })
        .entries(&app.left_panel.entries)
        .selected_index(app.left_panel.selected_index)
        .scroll_offset(app.left_panel.scroll_offset)
        .show_parent(show_parent_left)
        .selected_items(&app.left_panel.selected_items)
        .theme(theme);
    f.render_widget(left_panel, areas.left_panel);

    // 우측 패널 렌더링 (듀얼 패널 모드일 때만)
    if app.layout.is_dual_panel() {
        let right_path = app.right_panel.current_path.to_string_lossy();
        let show_parent_right = app.right_panel.current_path.parent().is_some();
        let right_panel = Panel::new()
            .title(&right_path)
            .status(if active_panel == ActivePanel::Right {
                PanelStatus::Active
            } else {
                PanelStatus::Inactive
            })
            .entries(&app.right_panel.entries)
            .selected_index(app.right_panel.selected_index)
            .scroll_offset(app.right_panel.scroll_offset)
            .show_parent(show_parent_right)
            .selected_items(&app.right_panel.selected_items)
            .theme(theme);
        f.render_widget(right_panel, areas.right_panel);
    }

    // 상태바 렌더링
    let active_panel_state = app.active_panel_state();
    let file_count = active_panel_state.file_count();
    let dir_count = active_panel_state.dir_count();
    let total_size = format_file_size(active_panel_state.total_size());
    let selected_count = active_panel_state.selected_count();
    let selected_size = format_file_size(active_panel_state.selected_size());

    let status_bar = StatusBar::new()
        .file_count(file_count)
        .dir_count(dir_count)
        .total_size(&total_size)
        .selected_count(selected_count)
        .selected_size(&selected_size)
        .layout_mode(app.layout_mode_str())
        .theme(theme);
    f.render_widget(status_bar, areas.status_bar);

    // 커맨드바 렌더링
    let command_bar = CommandBar::new().theme(theme);
    f.render_widget(command_bar, areas.command_bar);

    // 드롭다운 메뉴 렌더링 (메뉴가 활성화되어 있을 때)
    if app.is_menu_active() {
        if let Some(menu) = app.menus.get(app.menu_state.selected_menu) {
            // 메뉴 위치 계산
            let menu_bar_widget = MenuBar::new().menus(&app.menus);
            let menu_x = menu_bar_widget.get_menu_x_position(app.menu_state.selected_menu);

            // 드롭다운 영역 (메뉴바 아래)
            let dropdown_area = Rect {
                x: menu_x,
                y: areas.menu_bar.y + 1,
                width: f.area().width.saturating_sub(menu_x),
                height: f.area().height.saturating_sub(areas.menu_bar.y + 1),
            };

            let dropdown = DropdownMenu::new(menu, &app.menu_state).theme(theme);
            f.render_widget(dropdown, dropdown_area);
        }
    }

    // 다이얼로그 렌더링 (최상위 레이어)
    if let Some(ref dialog_kind) = app.dialog {
        let dialog = Dialog::new(dialog_kind).theme(theme);
        f.render_widget(dialog, f.area());
    }
}
