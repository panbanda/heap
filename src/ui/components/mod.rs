//! Reusable UI components.
//!
//! This module contains atomic and composite UI components used throughout
//! the application. Components are designed to be stateless where possible,
//! with styling driven by the theme system.

pub mod avatar;
pub mod badge;
pub mod button;
pub mod focusable_input;
pub mod icon;
pub mod input;
pub mod list;
mod notifications;
pub mod tooltip;

pub use avatar::{Avatar, AvatarGroup, AvatarShape, AvatarSize};
pub use badge::{Badge, BadgeSize, BadgeVariant, CountBadge, DotIndicator};
pub use button::{Button, ButtonSize, ButtonVariant, IconButton};
pub use focusable_input::{KeyInputResult, TextBuffer};
pub use icon::{Icon, IconLabel, IconName, IconSize};
pub use input::{InputSize, SearchInput, TextArea, TextInput};
pub use list::{EmptyState, ListDivider, ListHeader, ListItem, LoadingState, VirtualizedListState};
pub use notifications::{
    Notification, NotificationAction, NotificationManager, NotificationType, StatusBar,
    StatusMessage,
};
pub use tooltip::{HelpTooltip, KeyboardHint, Tooltip, TooltipBox, TooltipPosition};
