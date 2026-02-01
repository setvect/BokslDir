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
    ActivePanel, CommandBar, DropdownMenu, LayoutMode, MenuBar, Panel, PanelStatus, StatusBar,
    WarningScreen,
};
use utils::error::Result;

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

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.is_menu_active() {
                    // 메뉴 모드에서의 키 처리
                    handle_menu_keys(app, key.modifiers, key.code);
                } else {
                    // 일반 모드에서의 키 처리
                    handle_normal_keys(app, key.modifiers, key.code);
                }
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
        // Esc는 아무것도 안 함 (메뉴가 닫혀있을 때)
        (_, KeyCode::Esc) => {}
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
    let left_path = app.left_path.to_string_lossy();
    let left_panel = Panel::new()
        .title(&left_path)
        .status(if active_panel == ActivePanel::Left {
            PanelStatus::Active
        } else {
            PanelStatus::Inactive
        })
        .content("Press 'F9' to open menu\nPress 'Tab' to switch panels\nPress 'q' to quit")
        .theme(theme);
    f.render_widget(left_panel, areas.left_panel);

    // 우측 패널 렌더링 (듀얼 패널 모드일 때만)
    if app.layout.is_dual_panel() {
        let right_path = app.right_path.to_string_lossy();
        let right_panel =
            Panel::new()
                .title(&right_path)
                .status(if active_panel == ActivePanel::Right {
                    PanelStatus::Active
                } else {
                    PanelStatus::Inactive
                })
                .theme(theme);
        f.render_widget(right_panel, areas.right_panel);
    }

    // 상태바 렌더링
    let status_bar = StatusBar::new()
        .file_count(0)
        .dir_count(0)
        .total_size("0B")
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
}
