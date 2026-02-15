#![allow(dead_code)]
//! 액션 시스템 — 단일 진실 원천 (Single Source of Truth)
//!
//! 모든 키 바인딩, 메뉴 액션, 커맨드바 항목, 도움말 내용이
//! 이 모듈의 레지스트리를 참조합니다.

use crate::ui::components::command_bar::CommandItem;
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::LazyLock;

/// 모든 가능한 액션의 열거
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    GoToParent,
    EnterSelected,
    GoToTop,
    GoToBottom,
    PageUp,
    PageDown,
    TogglePanel,
    TabNew,
    TabClose,
    // File Operations
    Copy,
    Move,
    Delete,
    PermanentDelete,
    MakeDirectory,
    Rename,
    ShowProperties,
    // Selection
    ToggleSelection,
    InvertSelection,
    SelectAll,
    DeselectAll,
    // System
    ShowHelp,
    Refresh,
    OpenMenu,
    Quit,
    // Theme (메뉴 전용)
    ThemeDark,
    ThemeLight,
    ThemeContrast,
    // Sort (Phase 5.1)
    SortByName,
    SortBySize,
    SortByDate,
    SortByExt,
    SortAscending,
    SortDescending,
    // Filter (Phase 5.2)
    StartFilter,
    ClearFilter,
    // Settings
    ToggleIconMode,
    // View (Phase 5.3)
    ToggleHidden,
    ShowMountPoints,
    ShowTabList,
    HistoryBack,
    HistoryForward,
    ShowHistoryList,
    SizeFormatAuto,
    SizeFormatBytes,
    // About
    About,
}

/// 액션 카테고리
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCategory {
    Navigation,
    FileOperation,
    Selection,
    Sort,
    Filter,
    System,
}

/// 커맨드바 표시 정보
pub struct CommandBarEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub priority: u8,
}

/// 액션 정의 (메타데이터)
pub struct ActionDef {
    pub action: Action,
    pub id: &'static str,
    pub label: &'static str,
    pub category: ActionCategory,
    pub shortcut_display: Option<&'static str>,
    pub command_bar: Option<CommandBarEntry>,
}

/// 키 바인딩 정의
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: Option<KeyModifiers>, // None = any modifier
    pub action: Action,
}

/// 모든 액션 메타데이터
pub static ACTION_DEFS: &[ActionDef] = &[
    // Navigation
    ActionDef {
        action: Action::MoveUp,
        id: "move_up",
        label: "Move up",
        category: ActionCategory::Navigation,
        shortcut_display: Some("j / k"),
        command_bar: Some(CommandBarEntry {
            key: "j/k",
            label: "Up/Dn",
            priority: 50,
        }),
    },
    ActionDef {
        action: Action::MoveDown,
        id: "move_down",
        label: "Move down",
        category: ActionCategory::Navigation,
        shortcut_display: None,
        command_bar: None,
    },
    ActionDef {
        action: Action::GoToParent,
        id: "go_parent",
        label: "Parent dir",
        category: ActionCategory::Navigation,
        shortcut_display: Some("h / l"),
        command_bar: Some(CommandBarEntry {
            key: "h/l",
            label: "Nav",
            priority: 51,
        }),
    },
    ActionDef {
        action: Action::EnterSelected,
        id: "enter",
        label: "Enter dir",
        category: ActionCategory::Navigation,
        shortcut_display: None,
        command_bar: None,
    },
    ActionDef {
        action: Action::GoToTop,
        id: "go_top",
        label: "Top",
        category: ActionCategory::Navigation,
        shortcut_display: Some("gg / G"),
        command_bar: Some(CommandBarEntry {
            key: "gg/G",
            label: "Top/Bot",
            priority: 52,
        }),
    },
    ActionDef {
        action: Action::GoToBottom,
        id: "go_bottom",
        label: "Bottom",
        category: ActionCategory::Navigation,
        shortcut_display: None,
        command_bar: None,
    },
    ActionDef {
        action: Action::PageUp,
        id: "page_up",
        label: "Half page up",
        category: ActionCategory::Navigation,
        shortcut_display: Some("^U / ^D"),
        command_bar: Some(CommandBarEntry {
            key: "^U/D",
            label: "Page",
            priority: 53,
        }),
    },
    ActionDef {
        action: Action::PageDown,
        id: "page_down",
        label: "Half page down",
        category: ActionCategory::Navigation,
        shortcut_display: None,
        command_bar: None,
    },
    ActionDef {
        action: Action::TogglePanel,
        id: "toggle_panel",
        label: "Switch panel",
        category: ActionCategory::Navigation,
        shortcut_display: Some("Tab"),
        command_bar: Some(CommandBarEntry {
            key: "Tab",
            label: "Panel",
            priority: 54,
        }),
    },
    ActionDef {
        action: Action::TabNew,
        id: "tab_new",
        label: "New tab",
        category: ActionCategory::Navigation,
        shortcut_display: Some("tn"),
        command_bar: None,
    },
    ActionDef {
        action: Action::TabClose,
        id: "tab_close",
        label: "Close tab",
        category: ActionCategory::Navigation,
        shortcut_display: Some("tx"),
        command_bar: None,
    },
    // File Operations
    ActionDef {
        action: Action::Copy,
        id: "copy",
        label: "Copy",
        category: ActionCategory::FileOperation,
        shortcut_display: Some("y"),
        command_bar: Some(CommandBarEntry {
            key: "y",
            label: "Copy",
            priority: 10,
        }),
    },
    ActionDef {
        action: Action::Move,
        id: "move",
        label: "Move",
        category: ActionCategory::FileOperation,
        shortcut_display: Some("x"),
        command_bar: Some(CommandBarEntry {
            key: "x",
            label: "Move",
            priority: 11,
        }),
    },
    ActionDef {
        action: Action::Delete,
        id: "delete",
        label: "Delete",
        category: ActionCategory::FileOperation,
        shortcut_display: Some("d"),
        command_bar: Some(CommandBarEntry {
            key: "d",
            label: "Del",
            priority: 12,
        }),
    },
    ActionDef {
        action: Action::PermanentDelete,
        id: "perm_delete",
        label: "Permanent delete",
        category: ActionCategory::FileOperation,
        shortcut_display: Some("D"),
        command_bar: Some(CommandBarEntry {
            key: "D",
            label: "PermDel",
            priority: 40,
        }),
    },
    ActionDef {
        action: Action::MakeDirectory,
        id: "new_dir",
        label: "New directory",
        category: ActionCategory::FileOperation,
        shortcut_display: Some("a"),
        command_bar: Some(CommandBarEntry {
            key: "a",
            label: "MkDir",
            priority: 13,
        }),
    },
    ActionDef {
        action: Action::Rename,
        id: "rename",
        label: "Rename",
        category: ActionCategory::FileOperation,
        shortcut_display: Some("r"),
        command_bar: Some(CommandBarEntry {
            key: "r",
            label: "Ren",
            priority: 14,
        }),
    },
    ActionDef {
        action: Action::ShowProperties,
        id: "file_info",
        label: "File properties",
        category: ActionCategory::FileOperation,
        shortcut_display: Some("i"),
        command_bar: Some(CommandBarEntry {
            key: "i",
            label: "Info",
            priority: 15,
        }),
    },
    // Selection
    ActionDef {
        action: Action::ToggleSelection,
        id: "toggle_sel",
        label: "Toggle select",
        category: ActionCategory::Selection,
        shortcut_display: Some("Space"),
        command_bar: Some(CommandBarEntry {
            key: "Sp",
            label: "Sel",
            priority: 30,
        }),
    },
    ActionDef {
        action: Action::InvertSelection,
        id: "invert_selection",
        label: "Invert selection",
        category: ActionCategory::Selection,
        shortcut_display: Some("v"),
        command_bar: Some(CommandBarEntry {
            key: "v",
            label: "InvSel",
            priority: 31,
        }),
    },
    ActionDef {
        action: Action::SelectAll,
        id: "select_all",
        label: "Select all",
        category: ActionCategory::Selection,
        shortcut_display: Some("^A"),
        command_bar: Some(CommandBarEntry {
            key: "^A",
            label: "SelAll",
            priority: 32,
        }),
    },
    ActionDef {
        action: Action::DeselectAll,
        id: "deselect",
        label: "Deselect all",
        category: ActionCategory::Selection,
        shortcut_display: Some("u"),
        command_bar: Some(CommandBarEntry {
            key: "u",
            label: "Desel",
            priority: 33,
        }),
    },
    // System
    ActionDef {
        action: Action::ShowHelp,
        id: "help_keys",
        label: "Keyboard help",
        category: ActionCategory::System,
        shortcut_display: Some("?"),
        command_bar: Some(CommandBarEntry {
            key: "?",
            label: "Keys",
            priority: 20,
        }),
    },
    ActionDef {
        action: Action::Refresh,
        id: "refresh",
        label: "Refresh",
        category: ActionCategory::System,
        shortcut_display: Some("^R"),
        command_bar: Some(CommandBarEntry {
            key: "^R",
            label: "Refresh",
            priority: 41,
        }),
    },
    ActionDef {
        action: Action::OpenMenu,
        id: "open_menu",
        label: "Menu",
        category: ActionCategory::System,
        shortcut_display: Some("F9"),
        command_bar: None,
    },
    ActionDef {
        action: Action::Quit,
        id: "quit",
        label: "Quit",
        category: ActionCategory::System,
        shortcut_display: Some("q"),
        command_bar: Some(CommandBarEntry {
            key: "q",
            label: "Quit",
            priority: 21,
        }),
    },
    // Theme (메뉴 전용)
    ActionDef {
        action: Action::ThemeDark,
        id: "theme_dark",
        label: "Dark theme",
        category: ActionCategory::System,
        shortcut_display: None,
        command_bar: None,
    },
    ActionDef {
        action: Action::ThemeLight,
        id: "theme_light",
        label: "Light theme",
        category: ActionCategory::System,
        shortcut_display: None,
        command_bar: None,
    },
    ActionDef {
        action: Action::ThemeContrast,
        id: "theme_contrast",
        label: "High Contrast",
        category: ActionCategory::System,
        shortcut_display: None,
        command_bar: None,
    },
    // Settings (메뉴 전용)
    ActionDef {
        action: Action::ToggleIconMode,
        id: "toggle_icons",
        label: "Toggle icons",
        category: ActionCategory::System,
        shortcut_display: None,
        command_bar: None,
    },
    // Sort (Phase 5.1)
    ActionDef {
        action: Action::SortByName,
        id: "sort_name",
        label: "Sort by name",
        category: ActionCategory::Sort,
        shortcut_display: Some("sn"),
        command_bar: None,
    },
    ActionDef {
        action: Action::SortBySize,
        id: "sort_size",
        label: "Sort by size",
        category: ActionCategory::Sort,
        shortcut_display: Some("ss"),
        command_bar: None,
    },
    ActionDef {
        action: Action::SortByDate,
        id: "sort_date",
        label: "Sort by date",
        category: ActionCategory::Sort,
        shortcut_display: Some("sd"),
        command_bar: None,
    },
    ActionDef {
        action: Action::SortByExt,
        id: "sort_ext",
        label: "Sort by extension",
        category: ActionCategory::Sort,
        shortcut_display: Some("se"),
        command_bar: None,
    },
    ActionDef {
        action: Action::SortAscending,
        id: "sort_asc",
        label: "Reverse order",
        category: ActionCategory::Sort,
        shortcut_display: Some("sr"),
        command_bar: None,
    },
    ActionDef {
        action: Action::SortDescending,
        id: "sort_desc",
        label: "Descending",
        category: ActionCategory::Sort,
        shortcut_display: None,
        command_bar: None,
    },
    // Filter (Phase 5.2)
    ActionDef {
        action: Action::StartFilter,
        id: "filter_start",
        label: "Filter",
        category: ActionCategory::Filter,
        shortcut_display: Some("/"),
        command_bar: Some(CommandBarEntry {
            key: "/",
            label: "Filter",
            priority: 22,
        }),
    },
    ActionDef {
        action: Action::ClearFilter,
        id: "filter_clear",
        label: "Clear filter",
        category: ActionCategory::Filter,
        shortcut_display: None,
        command_bar: None,
    },
    // View (Phase 5.3)
    ActionDef {
        action: Action::ToggleHidden,
        id: "toggle_hidden",
        label: "Toggle hidden files",
        category: ActionCategory::System,
        shortcut_display: Some("."),
        command_bar: None,
    },
    ActionDef {
        action: Action::ShowMountPoints,
        id: "mount_points",
        label: "Mount points",
        category: ActionCategory::Navigation,
        shortcut_display: Some("gm"),
        command_bar: None,
    },
    ActionDef {
        action: Action::ShowTabList,
        id: "tab_list",
        label: "Show tab list",
        category: ActionCategory::Navigation,
        shortcut_display: Some("tt"),
        command_bar: None,
    },
    ActionDef {
        action: Action::HistoryBack,
        id: "history_back",
        label: "History back",
        category: ActionCategory::Navigation,
        shortcut_display: Some("Alt+Left"),
        command_bar: None,
    },
    ActionDef {
        action: Action::HistoryForward,
        id: "history_forward",
        label: "History forward",
        category: ActionCategory::Navigation,
        shortcut_display: Some("Alt+Right"),
        command_bar: None,
    },
    ActionDef {
        action: Action::ShowHistoryList,
        id: "history_list",
        label: "Show history list",
        category: ActionCategory::Navigation,
        shortcut_display: Some("th"),
        command_bar: None,
    },
    ActionDef {
        action: Action::SizeFormatAuto,
        id: "size_auto",
        label: "Size: Auto",
        category: ActionCategory::System,
        shortcut_display: None,
        command_bar: None,
    },
    ActionDef {
        action: Action::SizeFormatBytes,
        id: "size_bytes",
        label: "Size: Bytes",
        category: ActionCategory::System,
        shortcut_display: None,
        command_bar: None,
    },
    // About
    ActionDef {
        action: Action::About,
        id: "about",
        label: "About",
        category: ActionCategory::System,
        shortcut_display: None,
        command_bar: None,
    },
];

fn build_key_bindings() -> Vec<KeyBinding> {
    vec![
        // 종료
        KeyBinding {
            code: KeyCode::Char('q'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::Quit,
        },
        KeyBinding {
            code: KeyCode::Char('c'),
            modifiers: Some(KeyModifiers::CONTROL),
            action: Action::Quit,
        },
        // 패널/메뉴
        KeyBinding {
            code: KeyCode::Tab,
            modifiers: None,
            action: Action::TogglePanel,
        },
        KeyBinding {
            code: KeyCode::F(9),
            modifiers: None,
            action: Action::OpenMenu,
        },
        // 탐색: Vim
        KeyBinding {
            code: KeyCode::Char('j'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::MoveDown,
        },
        KeyBinding {
            code: KeyCode::Down,
            modifiers: None,
            action: Action::MoveDown,
        },
        KeyBinding {
            code: KeyCode::Char('k'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::MoveUp,
        },
        KeyBinding {
            code: KeyCode::Up,
            modifiers: None,
            action: Action::MoveUp,
        },
        KeyBinding {
            code: KeyCode::Char('h'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::GoToParent,
        },
        KeyBinding {
            code: KeyCode::Left,
            modifiers: Some(KeyModifiers::NONE),
            action: Action::GoToParent,
        },
        KeyBinding {
            code: KeyCode::Left,
            modifiers: Some(KeyModifiers::ALT),
            action: Action::HistoryBack,
        },
        KeyBinding {
            code: KeyCode::Right,
            modifiers: Some(KeyModifiers::ALT),
            action: Action::HistoryForward,
        },
        KeyBinding {
            code: KeyCode::Char('l'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::EnterSelected,
        },
        KeyBinding {
            code: KeyCode::Enter,
            modifiers: Some(KeyModifiers::NONE),
            action: Action::EnterSelected,
        },
        // G / Home / End
        KeyBinding {
            code: KeyCode::Char('G'),
            modifiers: None,
            action: Action::GoToBottom,
        },
        KeyBinding {
            code: KeyCode::Home,
            modifiers: None,
            action: Action::GoToTop,
        },
        KeyBinding {
            code: KeyCode::End,
            modifiers: None,
            action: Action::GoToBottom,
        },
        // 페이지
        KeyBinding {
            code: KeyCode::Char('u'),
            modifiers: Some(KeyModifiers::CONTROL),
            action: Action::PageUp,
        },
        KeyBinding {
            code: KeyCode::PageUp,
            modifiers: None,
            action: Action::PageUp,
        },
        KeyBinding {
            code: KeyCode::Char('d'),
            modifiers: Some(KeyModifiers::CONTROL),
            action: Action::PageDown,
        },
        KeyBinding {
            code: KeyCode::PageDown,
            modifiers: None,
            action: Action::PageDown,
        },
        // 파일 조작
        KeyBinding {
            code: KeyCode::Char('y'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::Copy,
        },
        KeyBinding {
            code: KeyCode::Char('x'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::Move,
        },
        KeyBinding {
            code: KeyCode::Char('d'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::Delete,
        },
        KeyBinding {
            code: KeyCode::Char('D'),
            modifiers: None,
            action: Action::PermanentDelete,
        },
        KeyBinding {
            code: KeyCode::Char('a'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::MakeDirectory,
        },
        KeyBinding {
            code: KeyCode::Char('r'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::Rename,
        },
        KeyBinding {
            code: KeyCode::Char('i'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::ShowProperties,
        },
        // 선택
        KeyBinding {
            code: KeyCode::Char(' '),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::ToggleSelection,
        },
        KeyBinding {
            code: KeyCode::Char('v'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::InvertSelection,
        },
        KeyBinding {
            code: KeyCode::Char('a'),
            modifiers: Some(KeyModifiers::CONTROL),
            action: Action::SelectAll,
        },
        KeyBinding {
            code: KeyCode::Char('u'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::DeselectAll,
        },
        // 필터/검색 (Phase 5.2)
        KeyBinding {
            code: KeyCode::Char('/'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::StartFilter,
        },
        // 숨김 파일 토글 (Phase 5.3)
        KeyBinding {
            code: KeyCode::Char('.'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::ToggleHidden,
        },
        // 시스템
        KeyBinding {
            code: KeyCode::Char('?'),
            modifiers: Some(KeyModifiers::NONE),
            action: Action::ShowHelp,
        },
        KeyBinding {
            code: KeyCode::Char('r'),
            modifiers: Some(KeyModifiers::CONTROL),
            action: Action::Refresh,
        },
    ]
}

static KEY_BINDINGS: LazyLock<Vec<KeyBinding>> = LazyLock::new(build_key_bindings);

/// 키 바인딩 목록 조회 (1회 초기화 후 재사용)
pub fn key_bindings() -> &'static [KeyBinding] {
    KEY_BINDINGS.as_slice()
}

/// 키 입력으로 액션 조회
pub fn find_action(modifiers: KeyModifiers, code: KeyCode) -> Option<Action> {
    for binding in key_bindings() {
        let code_matches = binding.code == code;
        let mod_matches = match binding.modifiers {
            None => true, // any modifier
            Some(required) => modifiers == required,
        };
        if code_matches && mod_matches {
            return Some(binding.action);
        }
    }
    None
}

/// action_id 문자열로 Action 조회
impl Action {
    pub fn from_id(id: &str) -> Option<Action> {
        ACTION_DEFS.iter().find(|d| d.id == id).map(|d| d.action)
    }
}

/// 커맨드바용 항목 생성 (priority 순 정렬)
pub fn generate_command_bar_items() -> Vec<CommandItem> {
    let mut entries: Vec<(&CommandBarEntry, &ActionDef)> = ACTION_DEFS
        .iter()
        .filter_map(|def| def.command_bar.as_ref().map(|cb| (cb, def)))
        .collect();

    entries.sort_by_key(|(cb, _)| cb.priority);

    entries
        .into_iter()
        .map(|(cb, _)| CommandItem::new(cb.key, cb.label))
        .collect()
}

/// 도움말 다이얼로그용 엔트리 생성
///
/// 반환: (카테고리명, Vec<(단축키, 설명)>) 목록
pub fn generate_help_entries() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    let categories = [
        (ActionCategory::Navigation, "Navigation"),
        (ActionCategory::FileOperation, "File Operations"),
        (ActionCategory::Selection, "Selection"),
        (ActionCategory::Sort, "Sort"),
        (ActionCategory::Filter, "Filter / Search"),
        (ActionCategory::System, "System"),
    ];

    categories
        .iter()
        .map(|(cat, name)| {
            let items: Vec<(&'static str, &'static str)> = ACTION_DEFS
                .iter()
                .filter(|d| d.category == *cat && d.shortcut_display.is_some())
                .map(|d| (d.shortcut_display.unwrap(), d.label))
                .collect();
            (*name, items)
        })
        .filter(|(_, items)| !items.is_empty())
        .collect()
}

/// 메뉴 단축키 표시용 조회
pub fn get_shortcut_display(id: &str) -> Option<&'static str> {
    ACTION_DEFS
        .iter()
        .find(|d| d.id == id)
        .and_then(|d| d.shortcut_display)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_from_id() {
        assert_eq!(Action::from_id("copy"), Some(Action::Copy));
        assert_eq!(Action::from_id("quit"), Some(Action::Quit));
        assert_eq!(Action::from_id("tab_new"), Some(Action::TabNew));
        assert_eq!(Action::from_id("tab_list"), Some(Action::ShowTabList));
        assert_eq!(
            Action::from_id("history_list"),
            Some(Action::ShowHistoryList)
        );
        assert_eq!(Action::from_id("nonexistent"), None);
    }

    #[test]
    fn test_find_action_vim_keys() {
        assert_eq!(
            find_action(KeyModifiers::NONE, KeyCode::Char('j')),
            Some(Action::MoveDown)
        );
        assert_eq!(
            find_action(KeyModifiers::NONE, KeyCode::Char('k')),
            Some(Action::MoveUp)
        );
        assert_eq!(
            find_action(KeyModifiers::NONE, KeyCode::Char('y')),
            Some(Action::Copy)
        );
        assert_eq!(
            find_action(KeyModifiers::NONE, KeyCode::Char('q')),
            Some(Action::Quit)
        );
    }

    #[test]
    fn test_find_action_arrow_keys() {
        assert_eq!(
            find_action(KeyModifiers::NONE, KeyCode::Down),
            Some(Action::MoveDown)
        );
        assert_eq!(
            find_action(KeyModifiers::NONE, KeyCode::Up),
            Some(Action::MoveUp)
        );
    }

    #[test]
    fn test_find_action_ctrl_keys() {
        assert_eq!(
            find_action(KeyModifiers::CONTROL, KeyCode::Char('c')),
            Some(Action::Quit)
        );
        assert_eq!(
            find_action(KeyModifiers::CONTROL, KeyCode::Char('r')),
            Some(Action::Refresh)
        );
    }

    #[test]
    fn test_find_action_alt_history_keys() {
        assert_eq!(
            find_action(KeyModifiers::ALT, KeyCode::Left),
            Some(Action::HistoryBack)
        );
        assert_eq!(
            find_action(KeyModifiers::ALT, KeyCode::Right),
            Some(Action::HistoryForward)
        );
    }

    #[test]
    fn test_find_action_any_modifier() {
        // Tab should work with any modifier
        assert_eq!(
            find_action(KeyModifiers::NONE, KeyCode::Tab),
            Some(Action::TogglePanel)
        );
        assert_eq!(
            find_action(KeyModifiers::SHIFT, KeyCode::Tab),
            Some(Action::TogglePanel)
        );
    }

    #[test]
    fn test_generate_command_bar_items() {
        let items = generate_command_bar_items();
        assert!(!items.is_empty());
        // 첫 항목은 priority 10 (Copy)
        assert_eq!(items[0].key, "y");
        assert_eq!(items[0].label, "Copy");
    }

    #[test]
    fn test_generate_help_entries() {
        let entries = generate_help_entries();
        assert!(!entries.is_empty());
        assert_eq!(entries[0].0, "Navigation");
        let nav_items = &entries[0].1;
        assert!(nav_items.iter().any(|(k, _)| *k == "tn"));
    }

    #[test]
    fn test_get_shortcut_display() {
        assert_eq!(get_shortcut_display("copy"), Some("y"));
        assert_eq!(get_shortcut_display("quit"), Some("q"));
        assert_eq!(get_shortcut_display("theme_dark"), None);
    }

    #[test]
    fn test_command_bar_count() {
        let items = generate_command_bar_items();
        // 20 items with command_bar entries (19 + Filter)
        assert_eq!(items.len(), 20);
    }
}
