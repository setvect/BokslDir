use super::super::*;
use crate::ui::{I18n, Language, TextKey};

pub(in crate::app) fn execute(app: &mut App, action: Action) {
    match action {
        Action::ShowHelp => app.show_help(),
        Action::Refresh => app.refresh_current(),
        Action::OpenMenu => app.open_menu(),
        Action::ThemeDark => app.switch_theme_and_save("dark"),
        Action::ThemeLight => app.switch_theme_and_save("light"),
        Action::ThemeContrast => app.switch_theme_and_save("high_contrast"),
        Action::SetLanguageEnglish => app.set_language_and_save(Language::English),
        Action::SetLanguageKorean => app.set_language_and_save(Language::Korean),
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
        Action::About => {
            let i18n = I18n::new(app.language());
            app.show_message(i18n.tr(TextKey::AboutTitle), i18n.tr(TextKey::AboutBody));
        }
        _ => unreachable!("non-dialog action: {:?}", action),
    }
}
