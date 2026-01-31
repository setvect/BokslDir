// UI Layer
pub mod components;
pub mod layout;
pub mod renderer;
pub mod theme;

// Re-export layout types for convenience
pub use layout::{
    ActivePanel, LayoutAreas, LayoutManager, LayoutMode, LayoutState, PanelRatio,
    DUAL_PANEL_MIN_WIDTH, MIN_HEIGHT, MIN_WIDTH, STANDARD_HEIGHT,
};

// Re-export components
pub use components::{
    create_default_menus, CommandBar, CommandItem, DropdownMenu, Menu, MenuBar, MenuItem,
    MenuItemKind, MenuState, Panel, PanelStatus, StatusBar, WarningScreen,
};
