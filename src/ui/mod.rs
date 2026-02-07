// UI Layer
pub mod components;
pub mod layout;
pub mod renderer;
pub mod theme;

// Re-export layout types for convenience
pub use layout::{ActivePanel, LayoutManager, LayoutMode};

// Re-export components
pub use components::{
    create_default_menus, CommandBar, Dialog, DialogKind, DropdownMenu, Menu, MenuBar, MenuState,
    Panel, PanelStatus, StatusBar, WarningScreen,
};

// Re-export theme types
pub use theme::{Theme, ThemeManager};
