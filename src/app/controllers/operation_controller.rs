use super::super::*;
use crate::ui::{I18n, MessageKey};

pub(in crate::app) fn execute(app: &mut App, action: Action) {
    match action {
        Action::Copy => app.start_copy(),
        Action::Move => app.start_move(),
        Action::OpenDefaultApp => app.start_open_default_app(),
        Action::OpenTerminalEditor => app.start_open_terminal_editor(),
        Action::Delete => app.start_delete(),
        Action::PermanentDelete => app.start_permanent_delete(),
        Action::MakeDirectory => app.start_mkdir(),
        Action::Rename => app.start_rename(),
        Action::ShowProperties => app.show_properties(),
        Action::ArchiveCompress => app.start_archive_compress(),
        Action::ArchiveExtract => app.start_archive_extract(),
        Action::ArchiveExtractAuto => app.start_archive_extract_auto(),
        Action::ArchivePreview => app.start_archive_preview(),
        Action::ToggleSelection => app.toggle_selection_and_move_down(),
        Action::InvertSelection => app.invert_selection(),
        Action::SelectAll => app.select_all(),
        Action::DeselectAll => app.deselect_all(),
        Action::SortByName => app.sort_active_panel(SortBy::Name),
        Action::SortBySize => app.sort_active_panel(SortBy::Size),
        Action::SortByDate => app.sort_active_panel(SortBy::Modified),
        Action::SortByExt => app.sort_active_panel(SortBy::Extension),
        Action::SortAscending => app.toggle_sort_order(),
        Action::SortDescending => {
            app.active_panel_state_mut()
                .set_sort_order(SortOrder::Descending);
            app.re_sort_active_panel();
        }
        Action::StartFilter => app.start_filter(),
        Action::ClearFilter => app.clear_filter(),
        Action::ToggleHidden => app.toggle_hidden(),
        Action::ShowMountPoints => app.show_mount_points(),
        Action::GoToPath => app.start_go_to_path(),
        Action::ShowTabList => app.show_tab_list(),
        Action::HistoryBack => app.history_back(),
        Action::HistoryForward => app.history_forward(),
        Action::ShowHistoryList => app.show_history_list(),
        Action::AddBookmark => app.add_bookmark_current_dir(),
        Action::ShowBookmarkList => app.show_bookmark_list(),
        Action::SizeFormatAuto => {
            app.size_format = SizeFormat::Auto;
            let i18n = I18n::new(app.language());
            app.set_toast(i18n.msg(MessageKey::SizeFormatAutoToast));
        }
        Action::SizeFormatBytes => {
            app.size_format = SizeFormat::Bytes;
            let i18n = I18n::new(app.language());
            app.set_toast(i18n.msg(MessageKey::SizeFormatBytesToast));
        }
        _ => unreachable!("non-operation action: {:?}", action),
    }
}
