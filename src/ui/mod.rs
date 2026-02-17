// UI Layer
pub mod components;
pub mod i18n;
pub mod layout;
pub mod theme;

// Re-export layout types for convenience
pub use layout::{ActivePanel, LayoutManager, LayoutMode};

// Re-export components
pub use components::{
    create_default_menus, CommandBar, Dialog, DialogKind, DropdownMenu, InputPurpose, Menu,
    MenuBar, MenuState, Panel, PanelStatus, StatusBar, WarningScreen,
};
pub use i18n::{localize_runtime_text, I18n, Language, MessageKey, TextKey};

// Re-export theme types
pub use theme::{Theme, ThemeManager};
