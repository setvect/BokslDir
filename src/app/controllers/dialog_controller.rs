use super::super::*;

pub(in crate::app) fn execute(app: &mut App, action: Action) {
    match action {
        Action::ShowHelp => app.show_help(),
        Action::Refresh => app.refresh_current(),
        Action::OpenMenu => app.open_menu(),
        Action::ThemeDark => app.switch_theme_and_save("dark"),
        Action::ThemeLight => app.switch_theme_and_save("light"),
        Action::ThemeContrast => app.switch_theme_and_save("high_contrast"),
        Action::ToggleIconMode => {
            use crate::ui::components::panel::IconMode;
            app.icon_mode = match app.icon_mode {
                IconMode::Emoji => IconMode::Ascii,
                IconMode::Ascii => IconMode::Emoji,
            };
        }
        Action::SetDefaultEditorVi => app.set_default_editor_vi(),
        Action::SetDefaultEditorVim => app.set_default_editor_vim(),
        Action::SetDefaultEditorNano => app.set_default_editor_nano(),
        Action::SetDefaultEditorEmacs => app.set_default_editor_emacs(),
        Action::About => app.show_message(
            "복슬Dir 정보",
            "복슬Dir\nRust 기반 TUI 듀얼 패널 파일 매니저",
        ),
        _ => unreachable!("non-dialog action: {:?}", action),
    }
}
