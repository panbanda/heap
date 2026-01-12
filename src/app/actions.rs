//! Application actions for keyboard shortcuts and commands.
//!
//! Actions are the primary way to trigger functionality in margin.
//! They can be bound to keyboard shortcuts or invoked programmatically.

use gpui::actions;

// Email operations
actions!(
    margin,
    [
        // Composition
        Compose,
        Reply,
        ReplyAll,
        Forward,
        SendEmail,
        SaveDraft,
        DiscardDraft,
        // Organization
        Archive,
        Trash,
        Star,
        Unstar,
        Snooze,
        Unsnooze,
        MarkRead,
        MarkUnread,
        ApplyLabel,
        RemoveLabel,
        MoveToLabel,
        // Navigation
        NextMessage,
        PreviousMessage,
        NextThread,
        PreviousThread,
        OpenThread,
        CloseThread,
        ExpandMessage,
        CollapseMessage,
        ExpandAllMessages,
        CollapseAllMessages,
        // Go to views
        GoToInbox,
        GoToStarred,
        GoToDrafts,
        GoToSent,
        GoToArchive,
        GoToTrash,
        GoToLabel,
        GoToStats,
        GoToScreener,
        // Search and command palette
        Search,
        SemanticSearch,
        OpenCommandPalette,
        CloseCommandPalette,
        // Selection
        SelectMessage,
        SelectAll,
        DeselectAll,
        SelectRange,
        // AI features
        SummarizeThread,
        GenerateReply,
        CategorizeEmail,
        // Account management
        SwitchAccount,
        AddAccount,
        RemoveAccount,
        // Settings
        OpenSettings,
        ToggleTheme,
        IncreaseFontSize,
        DecreaseFontSize,
        ResetFontSize,
        ToggleDensity,
        // Sync
        SyncNow,
        SyncAccount,
        // Application
        Quit,
        Undo,
        Redo,
        ShowHelp,
        ShowKeyboardShortcuts,
    ]
);

#[cfg(test)]
mod tests {
    #[test]
    fn actions_are_defined() {
        // Actions are compile-time verified by the macro
        // This test exists to ensure the module compiles
    }
}
