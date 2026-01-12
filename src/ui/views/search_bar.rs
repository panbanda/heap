//! Search bar component.
//!
//! Provides a search interface with:
//! - Full-text and semantic search modes
//! - Gmail-style operators (from:, to:, has:attachment, etc.)
//! - Search suggestions and history
//! - Keyboard navigation

use gpui::{
    div, prelude::*, px, rgba, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};

use crate::services::{SearchFolder, SearchMode};

/// A search operator for filtering.
#[derive(Debug, Clone)]
pub struct SearchOperator {
    /// Operator name (e.g., "from").
    pub name: &'static str,
    /// Example usage.
    pub example: &'static str,
    /// Description.
    pub description: &'static str,
}

impl SearchOperator {
    /// Returns all available operators.
    pub fn all() -> Vec<SearchOperator> {
        vec![
            SearchOperator {
                name: "from",
                example: "from:alice@example.com",
                description: "Emails from a specific sender",
            },
            SearchOperator {
                name: "to",
                example: "to:bob@example.com",
                description: "Emails sent to a specific recipient",
            },
            SearchOperator {
                name: "subject",
                example: "subject:meeting",
                description: "Emails with subject containing text",
            },
            SearchOperator {
                name: "has",
                example: "has:attachment",
                description: "Emails with attachments",
            },
            SearchOperator {
                name: "is",
                example: "is:unread",
                description: "Filter by status (unread, starred, read)",
            },
            SearchOperator {
                name: "in",
                example: "in:inbox",
                description: "Search in specific folder",
            },
            SearchOperator {
                name: "before",
                example: "before:2024-01-01",
                description: "Emails before a date",
            },
            SearchOperator {
                name: "after",
                example: "after:2024-01-01",
                description: "Emails after a date",
            },
            SearchOperator {
                name: "older_than",
                example: "older_than:7d",
                description: "Emails older than duration (d/w/m/y)",
            },
            SearchOperator {
                name: "newer_than",
                example: "newer_than:24h",
                description: "Emails newer than duration",
            },
        ]
    }
}

/// A search suggestion.
#[derive(Debug, Clone)]
pub struct SearchSuggestion {
    /// Suggestion text.
    pub text: String,
    /// Type of suggestion.
    pub kind: SuggestionKind,
}

/// Kind of search suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    /// Recent search query.
    Recent,
    /// Operator completion.
    Operator,
    /// Contact suggestion.
    Contact,
    /// Saved search.
    Saved,
}

impl SuggestionKind {
    /// Returns the icon for this kind.
    pub fn icon(&self) -> &'static str {
        match self {
            SuggestionKind::Recent => "history",
            SuggestionKind::Operator => "code",
            SuggestionKind::Contact => "user",
            SuggestionKind::Saved => "bookmark",
        }
    }
}

/// The search bar view component.
pub struct SearchBar {
    /// Whether the search bar is focused/expanded.
    expanded: bool,
    /// Current search query.
    query: String,
    /// Search mode.
    mode: SearchMode,
    /// Folder filter.
    folder: Option<SearchFolder>,
    /// Suggestions.
    suggestions: Vec<SearchSuggestion>,
    /// Selected suggestion index.
    selected_suggestion: Option<usize>,
    /// Whether search is in progress.
    searching: bool,
    /// Show operators help.
    show_operators: bool,
}

impl SearchBar {
    /// Creates a new search bar.
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            expanded: false,
            query: String::new(),
            mode: SearchMode::Hybrid,
            folder: None,
            suggestions: Vec::new(),
            selected_suggestion: None,
            searching: false,
            show_operators: false,
        }
    }

    /// Expands/focuses the search bar.
    pub fn expand(&mut self) {
        self.expanded = true;
        self.update_suggestions();
    }

    /// Collapses the search bar.
    pub fn collapse(&mut self) {
        self.expanded = false;
        self.query.clear();
        self.suggestions.clear();
        self.selected_suggestion = None;
        self.show_operators = false;
    }

    /// Returns whether the search bar is expanded.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Sets the search query.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.update_suggestions();
    }

    /// Returns the current query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Sets the search mode.
    pub fn set_mode(&mut self, mode: SearchMode) {
        self.mode = mode;
    }

    /// Returns the current mode.
    pub fn mode(&self) -> SearchMode {
        self.mode
    }

    /// Sets the folder filter.
    pub fn set_folder(&mut self, folder: Option<SearchFolder>) {
        self.folder = folder;
    }

    /// Toggles operators help.
    pub fn toggle_operators_help(&mut self) {
        self.show_operators = !self.show_operators;
    }

    /// Selects the next suggestion.
    pub fn select_next(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected_suggestion = match self.selected_suggestion {
            Some(i) if i < self.suggestions.len() - 1 => Some(i + 1),
            Some(_) => Some(0),
            None => Some(0),
        };
    }

    /// Selects the previous suggestion.
    pub fn select_previous(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected_suggestion = match self.selected_suggestion {
            Some(0) => Some(self.suggestions.len() - 1),
            Some(i) => Some(i - 1),
            None => Some(self.suggestions.len() - 1),
        };
    }

    /// Accepts the selected suggestion.
    pub fn accept_suggestion(&mut self) -> Option<String> {
        let suggestion = self
            .selected_suggestion
            .and_then(|i| self.suggestions.get(i))
            .map(|s| s.text.clone())?;
        self.query = suggestion.clone();
        self.update_suggestions();
        Some(suggestion)
    }

    /// Updates suggestions based on current query.
    fn update_suggestions(&mut self) {
        self.suggestions.clear();
        self.selected_suggestion = None;

        let query_lower = self.query.to_lowercase();

        // If query is empty, show recent searches
        if self.query.is_empty() {
            self.suggestions.push(SearchSuggestion {
                text: "deployment issue".to_string(),
                kind: SuggestionKind::Recent,
            });
            self.suggestions.push(SearchSuggestion {
                text: "from:alice@example.com".to_string(),
                kind: SuggestionKind::Recent,
            });
            return;
        }

        // Check if typing an operator
        if query_lower.ends_with(':') || self.query.ends_with(' ') {
            let ops = SearchOperator::all();
            for op in ops {
                if query_lower.contains(&format!("{}:", op.name)) {
                    continue; // Already using this operator
                }
                self.suggestions.push(SearchSuggestion {
                    text: format!("{} {}:", self.query.trim(), op.name),
                    kind: SuggestionKind::Operator,
                });
                if self.suggestions.len() >= 5 {
                    break;
                }
            }
        }

        // Add operator suggestions if query starts with operator prefix
        for op in SearchOperator::all() {
            if op.name.starts_with(&query_lower) && !query_lower.contains(':') {
                self.suggestions.push(SearchSuggestion {
                    text: format!("{}:", op.name),
                    kind: SuggestionKind::Operator,
                });
            }
        }
    }

    /// Sets searching state.
    pub fn set_searching(&mut self, searching: bool) {
        self.searching = searching;
    }

    fn render_mode_selector(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let modes = [
            (SearchMode::Hybrid, "Hybrid", "AI + Keywords"),
            (SearchMode::FullText, "Keywords", "Fast exact match"),
            (SearchMode::Semantic, "AI", "Semantic search"),
        ];

        div()
            .flex()
            .gap(px(4.0))
            .children(modes.iter().map(|(mode, label, _desc)| {
                let is_selected = self.mode == *mode;
                div()
                    .id(SharedString::from(format!("mode-{:?}", mode)))
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(is_selected, |d| d.bg(rgba(0x3B82F6FF)))
                    .when(!is_selected, |d| {
                        d.bg(rgba(0x27272AFF)).hover(|d| d.bg(rgba(0x3F3F46FF)))
                    })
                    .child(
                        div()
                            .text_xs()
                            .when(is_selected, |d| d.text_color(rgba(0xFFFFFFFF)))
                            .when(!is_selected, |d| d.text_color(rgba(0xA1A1AAFF)))
                            .child(*label),
                    )
            }))
    }

    fn render_suggestion(&self, suggestion: &SearchSuggestion, index: usize) -> impl IntoElement {
        let is_selected = self.selected_suggestion == Some(index);
        let text = suggestion.text.clone();
        let icon = suggestion.kind.icon();

        div()
            .id(SharedString::from(format!("suggestion-{}", index)))
            .h(px(36.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .when(is_selected, |d| d.bg(rgba(0x3B82F620)))
            .when(!is_selected, |d| d.hover(|d| d.bg(rgba(0xFFFFFF08))))
            .child(div().text_sm().text_color(rgba(0x52525BFF)).child(icon))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(rgba(0xE4E4E7FF))
                    .child(text),
            )
    }

    fn render_operators_help(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .children(SearchOperator::all().iter().map(|op| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .w(px(100.0))
                            .px(px(6.0))
                            .py(px(2.0))
                            .bg(rgba(0x27272AFF))
                            .rounded(px(4.0))
                            .text_xs()
                            .font_family("monospace")
                            .text_color(rgba(0x60A5FAFF))
                            .child(op.example),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(rgba(0x71717AFF))
                            .child(op.description),
                    )
            }))
    }

    fn render_compact(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("search-bar-compact")
            .h(px(36.0))
            .w(px(240.0))
            .px(px(12.0))
            .bg(rgba(0x27272AFF))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .hover(|d| d.bg(rgba(0x3F3F46FF)))
            .child(div().text_sm().text_color(rgba(0x52525BFF)).child("search"))
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(rgba(0x52525BFF))
                    .child("Search emails..."),
            )
            .child(
                div()
                    .px(px(6.0))
                    .py(px(2.0))
                    .bg(rgba(0x3F3F46FF))
                    .rounded(px(4.0))
                    .text_xs()
                    .text_color(rgba(0x71717AFF))
                    .child("/"),
            )
    }

    fn render_expanded(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("search-bar-expanded")
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_start()
            .justify_center()
            .pt(px(80.0))
            .child(
                div()
                    .w(px(600.0))
                    .bg(rgba(0x18181BFF))
                    .rounded(px(12.0))
                    .shadow_lg()
                    .border_1()
                    .border_color(rgba(0x27272AFF))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Search input
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .border_b_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div()
                                    .text_lg()
                                    .when(self.searching, |d| d.text_color(rgba(0x3B82F6FF)))
                                    .when(!self.searching, |d| d.text_color(rgba(0x71717AFF)))
                                    .child(if self.searching { "loading" } else { "search" }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_base()
                                    .text_color(rgba(0xE4E4E7FF))
                                    .child(if self.query.is_empty() {
                                        div().text_color(rgba(0x52525BFF)).child("Search emails...")
                                    } else {
                                        div().child(self.query.clone())
                                    }),
                            )
                            .child(
                                div()
                                    .id("operators-help")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .when(self.show_operators, |d| d.bg(rgba(0x3B82F620)))
                                    .when(!self.show_operators, |d| d.bg(rgba(0x27272AFF)))
                                    .hover(|d| d.bg(rgba(0x3F3F46FF)))
                                    .child(div().text_xs().text_color(rgba(0x71717AFF)).child("?")),
                            )
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .bg(rgba(0x27272AFF))
                                    .rounded(px(4.0))
                                    .text_xs()
                                    .text_color(rgba(0x71717AFF))
                                    .child("Esc"),
                            ),
                    )
                    // Mode selector
                    .child(
                        div()
                            .h(px(44.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgba(0x71717AFF))
                                    .child("Search mode:"),
                            )
                            .child(self.render_mode_selector(cx)),
                    )
                    // Operators help (if shown)
                    .when(self.show_operators, |d| {
                        d.child(
                            div()
                                .max_h(px(200.0))
                                .overflow_hidden()
                                .border_b_1()
                                .border_color(rgba(0x27272AFF))
                                .child(self.render_operators_help(cx)),
                        )
                    })
                    // Suggestions
                    .when(!self.show_operators, |d| {
                        d.child(
                            div()
                                .max_h(px(240.0))
                                .overflow_hidden()
                                .when(self.suggestions.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .p(px(16.0))
                                            .text_sm()
                                            .text_color(rgba(0x52525BFF))
                                            .text_center()
                                            .child("Start typing to search"),
                                    )
                                })
                                .when(!self.suggestions.is_empty(), |d| {
                                    d.py(px(4.0)).children(
                                        self.suggestions
                                            .iter()
                                            .enumerate()
                                            .map(|(i, s)| self.render_suggestion(s, i)),
                                    )
                                }),
                        )
                    })
                    // Footer
                    .child(
                        div()
                            .h(px(40.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .gap(px(16.0))
                            .border_t_1()
                            .border_color(rgba(0x27272AFF))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(2.0))
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("up/dn"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x52525BFF))
                                            .child("navigate"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(2.0))
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("Enter"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x52525BFF))
                                            .child("search"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .px(px(4.0))
                                            .bg(rgba(0x27272AFF))
                                            .rounded(px(2.0))
                                            .text_xs()
                                            .text_color(rgba(0x71717AFF))
                                            .child("Tab"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgba(0x52525BFF))
                                            .child("complete"),
                                    ),
                            ),
                    ),
            )
    }
}

impl Render for SearchBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.expanded {
            self.render_expanded(cx).into_any_element()
        } else {
            self.render_compact(cx).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_operators() {
        let ops = SearchOperator::all();
        assert!(!ops.is_empty());
        assert!(ops.iter().any(|o| o.name == "from"));
        assert!(ops.iter().any(|o| o.name == "has"));
    }

    #[test]
    fn suggestion_kind_icons() {
        assert_eq!(SuggestionKind::Recent.icon(), "history");
        assert_eq!(SuggestionKind::Operator.icon(), "code");
    }

    #[test]
    fn search_bar_mode() {
        // Mode tests require ViewContext
    }
}
