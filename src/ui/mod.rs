//! UI components and views
//!
//! This module contains the gpui-based user interface for margin.
//! The UI is organized into:
//! - `theme`: Color schemes and styling
//! - `components`: Reusable UI primitives
//! - `views`: Full-screen application views
//! - `keybindings`: Keyboard shortcut management

pub mod components;
pub mod keybindings;
pub mod theme;
pub mod views;

pub use keybindings::{
    Key, KeyBinding, KeyContext, KeyResult, KeybindingConfig, KeybindingManager, Keystroke,
    Modifiers,
};
pub use theme::{Theme, ThemeColors, ThemeMode};
pub use views::MainWindow;
