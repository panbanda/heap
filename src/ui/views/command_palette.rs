//! Command palette view.
//!
//! Fuzzy-searchable command launcher overlay.

use gpui::{
    div, prelude::FluentBuilder, px, Context, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, Styled, Window,
};

use crate::ui::theme::ThemeColors;

/// Command palette view component.
pub struct CommandPalette {
    colors: ThemeColors,
    query: String,
    commands: Vec<Command>,
    filtered_commands: Vec<usize>,
    selected_index: usize,
    visible: bool,
}

/// A command in the palette.
#[derive(Clone)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub category: CommandCategory,
}

/// Command categories.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    Navigation,
    Email,
    Compose,
    Settings,
    Ai,
}

impl CommandCategory {
    #[allow(dead_code)]
    fn label(&self) -> &str {
        match self {
            Self::Navigation => "Navigation",
            Self::Email => "Email Actions",
            Self::Compose => "Compose",
            Self::Settings => "Settings",
            Self::Ai => "AI",
        }
    }
}

impl CommandPalette {
    /// Create a new command palette.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        let commands = Self::default_commands();

        Self {
            colors: ThemeColors::dark(),
            query: String::new(),
            filtered_commands: (0..commands.len()).collect(),
            commands,
            selected_index: 0,
            visible: false,
        }
    }

    fn default_commands() -> Vec<Command> {
        vec![
            Command {
                id: "goto-inbox".to_string(),
                label: "Go to Inbox".to_string(),
                shortcut: Some("g i".to_string()),
                category: CommandCategory::Navigation,
            },
            Command {
                id: "goto-starred".to_string(),
                label: "Go to Starred".to_string(),
                shortcut: Some("g s".to_string()),
                category: CommandCategory::Navigation,
            },
            Command {
                id: "goto-sent".to_string(),
                label: "Go to Sent".to_string(),
                shortcut: Some("g t".to_string()),
                category: CommandCategory::Navigation,
            },
            Command {
                id: "goto-drafts".to_string(),
                label: "Go to Drafts".to_string(),
                shortcut: Some("g d".to_string()),
                category: CommandCategory::Navigation,
            },
            Command {
                id: "goto-archive".to_string(),
                label: "Go to Archive".to_string(),
                shortcut: Some("g a".to_string()),
                category: CommandCategory::Navigation,
            },
            Command {
                id: "compose".to_string(),
                label: "Compose New Email".to_string(),
                shortcut: Some("c".to_string()),
                category: CommandCategory::Compose,
            },
            Command {
                id: "reply".to_string(),
                label: "Reply".to_string(),
                shortcut: Some("r".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "reply-all".to_string(),
                label: "Reply All".to_string(),
                shortcut: Some("R".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "forward".to_string(),
                label: "Forward".to_string(),
                shortcut: Some("f".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "archive".to_string(),
                label: "Archive".to_string(),
                shortcut: Some("e".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "trash".to_string(),
                label: "Move to Trash".to_string(),
                shortcut: Some("#".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "star".to_string(),
                label: "Star/Unstar".to_string(),
                shortcut: Some("s".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "snooze".to_string(),
                label: "Snooze".to_string(),
                shortcut: Some("h".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "mark-read".to_string(),
                label: "Mark as Read".to_string(),
                shortcut: Some("u".to_string()),
                category: CommandCategory::Email,
            },
            Command {
                id: "ai-summarize".to_string(),
                label: "AI: Summarize Thread".to_string(),
                shortcut: None,
                category: CommandCategory::Ai,
            },
            Command {
                id: "ai-draft".to_string(),
                label: "AI: Draft Reply".to_string(),
                shortcut: None,
                category: CommandCategory::Ai,
            },
            Command {
                id: "settings".to_string(),
                label: "Open Settings".to_string(),
                shortcut: Some("Cmd+,".to_string()),
                category: CommandCategory::Settings,
            },
            Command {
                id: "toggle-theme".to_string(),
                label: "Toggle Theme".to_string(),
                shortcut: None,
                category: CommandCategory::Settings,
            },
        ]
    }

    /// Show the command palette.
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.filter_commands();
        self.selected_index = 0;
    }

    /// Hide the command palette.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set the search query.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.filter_commands();
        self.selected_index = 0;
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.filtered_commands.len() {
            self.selected_index += 1;
        }
    }

    /// Get the currently selected command.
    pub fn selected_command(&self) -> Option<&Command> {
        self.filtered_commands
            .get(self.selected_index)
            .and_then(|&idx| self.commands.get(idx))
    }

    fn filter_commands(&mut self) {
        if self.query.is_empty() {
            self.filtered_commands = (0..self.commands.len()).collect();
        } else {
            let query_lower = self.query.to_lowercase();
            self.filtered_commands = self
                .commands
                .iter()
                .enumerate()
                .filter(|(_, cmd)| cmd.label.to_lowercase().contains(&query_lower))
                .map(|(idx, _)| idx)
                .collect();
        }
    }

    fn render_search_input(&self) -> impl IntoElement {
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;

        let display = if self.query.is_empty() {
            "Type a command..."
        } else {
            &self.query
        };
        let color = if self.query.is_empty() {
            text_muted
        } else {
            text_primary
        };

        div()
            .px(px(16.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(self.colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_color(text_muted)
                            .child(SharedString::from(">")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(color)
                            .child(SharedString::from(display.to_string())),
                    ),
            )
    }

    fn render_command(&self, command: &Command, is_selected: bool) -> impl IntoElement {
        let bg = if is_selected {
            self.colors.surface_elevated
        } else {
            gpui::Hsla::transparent_black()
        };
        let hover_bg = self.colors.surface_elevated;
        let text_primary = self.colors.text_primary;
        let text_muted = self.colors.text_muted;

        div()
            .px(px(16.0))
            .py(px(8.0))
            .bg(bg)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(text_primary)
                            .child(SharedString::from(command.label.clone())),
                    )
                    .when_some(command.shortcut.clone(), |this, shortcut| {
                        this.child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(self.colors.surface)
                                .text_xs()
                                .text_color(text_muted)
                                .child(SharedString::from(shortcut)),
                        )
                    }),
            )
    }

    fn render_empty_state(&self) -> impl IntoElement {
        div()
            .px(px(16.0))
            .py(px(24.0))
            .text_color(self.colors.text_muted)
            .flex()
            .justify_center()
            .child(SharedString::from("No commands found"))
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().id("command-palette-hidden");
        }

        div()
            .id("command-palette-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(100.0))
            .bg(gpui::Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.5,
            })
            .child(
                div()
                    .w(px(500.0))
                    .max_h(px(400.0))
                    .rounded(px(8.0))
                    .bg(self.colors.surface)
                    .border_1()
                    .border_color(self.colors.border)
                    .overflow_hidden()
                    .child(self.render_search_input())
                    .child(
                        div()
                            .max_h(px(320.0))
                            .overflow_y_hidden()
                            .when(self.filtered_commands.is_empty(), |this| {
                                this.child(self.render_empty_state())
                            })
                            .when(!self.filtered_commands.is_empty(), |this| {
                                this.children(self.filtered_commands.iter().enumerate().map(
                                    |(display_idx, &cmd_idx)| {
                                        let cmd = &self.commands[cmd_idx];
                                        let is_selected = display_idx == self.selected_index;
                                        self.render_command(cmd, is_selected)
                                    },
                                ))
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_palette_visibility() {
        let mut palette = CommandPalette {
            colors: ThemeColors::dark(),
            query: String::new(),
            commands: Vec::new(),
            filtered_commands: Vec::new(),
            selected_index: 0,
            visible: false,
        };

        assert!(!palette.is_visible());

        palette.show();
        assert!(palette.is_visible());

        palette.hide();
        assert!(!palette.is_visible());
    }

    #[test]
    fn command_filtering() {
        let mut palette = CommandPalette {
            colors: ThemeColors::dark(),
            query: String::new(),
            commands: vec![
                Command {
                    id: "inbox".to_string(),
                    label: "Go to Inbox".to_string(),
                    shortcut: None,
                    category: CommandCategory::Navigation,
                },
                Command {
                    id: "compose".to_string(),
                    label: "Compose".to_string(),
                    shortcut: None,
                    category: CommandCategory::Compose,
                },
            ],
            filtered_commands: vec![0, 1],
            selected_index: 0,
            visible: true,
        };

        assert_eq!(palette.filtered_commands.len(), 2);

        palette.set_query("inbox".to_string());
        assert_eq!(palette.filtered_commands.len(), 1);
        assert_eq!(palette.commands[palette.filtered_commands[0]].id, "inbox");
    }

    #[test]
    fn selection_navigation() {
        let mut palette = CommandPalette {
            colors: ThemeColors::dark(),
            query: String::new(),
            commands: vec![
                Command {
                    id: "1".to_string(),
                    label: "Command 1".to_string(),
                    shortcut: None,
                    category: CommandCategory::Navigation,
                },
                Command {
                    id: "2".to_string(),
                    label: "Command 2".to_string(),
                    shortcut: None,
                    category: CommandCategory::Navigation,
                },
                Command {
                    id: "3".to_string(),
                    label: "Command 3".to_string(),
                    shortcut: None,
                    category: CommandCategory::Navigation,
                },
            ],
            filtered_commands: vec![0, 1, 2],
            selected_index: 0,
            visible: true,
        };

        assert_eq!(palette.selected_index, 0);

        palette.select_next();
        assert_eq!(palette.selected_index, 1);

        palette.select_next();
        assert_eq!(palette.selected_index, 2);

        palette.select_next(); // Should stay at 2
        assert_eq!(palette.selected_index, 2);

        palette.select_previous();
        assert_eq!(palette.selected_index, 1);
    }

    #[test]
    fn selected_command() {
        let palette = CommandPalette {
            colors: ThemeColors::dark(),
            query: String::new(),
            commands: vec![Command {
                id: "test".to_string(),
                label: "Test Command".to_string(),
                shortcut: None,
                category: CommandCategory::Navigation,
            }],
            filtered_commands: vec![0],
            selected_index: 0,
            visible: true,
        };

        let cmd = palette.selected_command().unwrap();
        assert_eq!(cmd.id, "test");
    }
}
