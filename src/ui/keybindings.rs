//! Keyboard binding management.
//!
//! Provides configurable keyboard shortcuts with:
//! - Single-key and modifier combinations
//! - Multi-key sequences (Gmail-style `g i`)
//! - Context-aware bindings
//! - Conflict detection
//! - User customization

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// A keyboard key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Numbers
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Navigation
    Up, Down, Left, Right, Home, End, PageUp, PageDown,
    // Editing
    Backspace, Delete, Tab, Enter, Escape, Space,
    // Punctuation
    Comma, Period, Slash, Backslash, Semicolon, Quote,
    BracketLeft, BracketRight, Minus, Equals, Grave,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Key::A => "A", Key::B => "B", Key::C => "C", Key::D => "D",
            Key::E => "E", Key::F => "F", Key::G => "G", Key::H => "H",
            Key::I => "I", Key::J => "J", Key::K => "K", Key::L => "L",
            Key::M => "M", Key::N => "N", Key::O => "O", Key::P => "P",
            Key::Q => "Q", Key::R => "R", Key::S => "S", Key::T => "T",
            Key::U => "U", Key::V => "V", Key::W => "W", Key::X => "X",
            Key::Y => "Y", Key::Z => "Z",
            Key::Num0 => "0", Key::Num1 => "1", Key::Num2 => "2",
            Key::Num3 => "3", Key::Num4 => "4", Key::Num5 => "5",
            Key::Num6 => "6", Key::Num7 => "7", Key::Num8 => "8",
            Key::Num9 => "9",
            Key::F1 => "F1", Key::F2 => "F2", Key::F3 => "F3", Key::F4 => "F4",
            Key::F5 => "F5", Key::F6 => "F6", Key::F7 => "F7", Key::F8 => "F8",
            Key::F9 => "F9", Key::F10 => "F10", Key::F11 => "F11", Key::F12 => "F12",
            Key::Up => "Up", Key::Down => "Down", Key::Left => "Left", Key::Right => "Right",
            Key::Home => "Home", Key::End => "End", Key::PageUp => "PageUp", Key::PageDown => "PageDown",
            Key::Backspace => "Backspace", Key::Delete => "Delete", Key::Tab => "Tab",
            Key::Enter => "Enter", Key::Escape => "Esc", Key::Space => "Space",
            Key::Comma => ",", Key::Period => ".", Key::Slash => "/",
            Key::Backslash => "\\", Key::Semicolon => ";", Key::Quote => "'",
            Key::BracketLeft => "[", Key::BracketRight => "]",
            Key::Minus => "-", Key::Equals => "=", Key::Grave => "`",
        };
        write!(f, "{}", s)
    }
}

/// Modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifiers {
    /// Command/Super key (Cmd on macOS, Win on Windows).
    pub cmd: bool,
    /// Control key.
    pub ctrl: bool,
    /// Alt/Option key.
    pub alt: bool,
    /// Shift key.
    pub shift: bool,
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            cmd: false,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }
}

impl Modifiers {
    /// No modifiers.
    pub fn none() -> Self {
        Self::default()
    }

    /// Command modifier only.
    pub fn cmd() -> Self {
        Self { cmd: true, ..Default::default() }
    }

    /// Control modifier only.
    pub fn ctrl() -> Self {
        Self { ctrl: true, ..Default::default() }
    }

    /// Shift modifier only.
    pub fn shift() -> Self {
        Self { shift: true, ..Default::default() }
    }

    /// Alt modifier only.
    pub fn alt() -> Self {
        Self { alt: true, ..Default::default() }
    }

    /// Returns true if any modifier is pressed.
    pub fn any(&self) -> bool {
        self.cmd || self.ctrl || self.alt || self.shift
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.cmd { parts.push("Cmd"); }
        if self.ctrl { parts.push("Ctrl"); }
        if self.alt { parts.push("Alt"); }
        if self.shift { parts.push("Shift"); }
        write!(f, "{}", parts.join("+"))
    }
}

/// A single keystroke (key + modifiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Keystroke {
    /// The key pressed.
    pub key: Key,
    /// Active modifiers.
    pub modifiers: Modifiers,
}

impl Keystroke {
    /// Creates a new keystroke.
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Creates a keystroke with no modifiers.
    pub fn key(key: Key) -> Self {
        Self { key, modifiers: Modifiers::none() }
    }

    /// Creates a Cmd+key keystroke.
    pub fn cmd(key: Key) -> Self {
        Self { key, modifiers: Modifiers::cmd() }
    }

    /// Creates a Ctrl+key keystroke.
    pub fn ctrl(key: Key) -> Self {
        Self { key, modifiers: Modifiers::ctrl() }
    }

    /// Creates a Shift+key keystroke.
    pub fn shift(key: Key) -> Self {
        Self { key, modifiers: Modifiers::shift() }
    }
}

impl fmt::Display for Keystroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.any() {
            write!(f, "{}+{}", self.modifiers, self.key)
        } else {
            write!(f, "{}", self.key)
        }
    }
}

/// A key binding (one or more keystrokes in sequence).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    /// Sequence of keystrokes.
    pub sequence: Vec<Keystroke>,
}

impl KeyBinding {
    /// Creates a single-keystroke binding.
    pub fn single(keystroke: Keystroke) -> Self {
        Self { sequence: vec![keystroke] }
    }

    /// Creates a multi-keystroke sequence binding.
    pub fn sequence(sequence: Vec<Keystroke>) -> Self {
        Self { sequence }
    }

    /// Returns the display string for this binding.
    pub fn display(&self) -> String {
        self.sequence
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Returns true if this is a single keystroke binding.
    pub fn is_single(&self) -> bool {
        self.sequence.len() == 1
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Context in which keybindings are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyContext {
    /// Global context (always active unless overridden).
    Global,
    /// Message list view.
    MessageList,
    /// Reading pane view.
    ReadingPane,
    /// Composer/editor view.
    Composer,
    /// Command palette.
    CommandPalette,
    /// Settings panel.
    Settings,
    /// Search bar.
    Search,
}

impl Default for KeyContext {
    fn default() -> Self {
        Self::Global
    }
}

/// A command that can be triggered by a keybinding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Command {
    /// Unique command identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Category for grouping.
    pub category: String,
}

impl Command {
    /// Creates a new command.
    pub fn new(id: impl Into<String>, name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
        }
    }
}

/// A keybinding conflict.
#[derive(Debug, Clone)]
pub struct Conflict {
    /// The conflicting binding.
    pub binding: KeyBinding,
    /// Commands that share this binding.
    pub commands: Vec<String>,
    /// Context where conflict occurs.
    pub context: KeyContext,
}

/// Result of processing a keystroke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyResult {
    /// No binding matched, key was ignored.
    Ignored,
    /// Keystroke is part of a sequence, waiting for more keys.
    Pending,
    /// A command was matched.
    Matched(String),
}

/// Manages keyboard bindings and input processing.
pub struct KeybindingManager {
    /// Bindings organized by context.
    bindings: HashMap<KeyContext, HashMap<KeyBinding, String>>,
    /// Current key sequence being built.
    pending_sequence: Vec<Keystroke>,
    /// Time of last keystroke.
    last_keystroke: Option<Instant>,
    /// Timeout for key sequences.
    sequence_timeout: Duration,
    /// Current context.
    current_context: KeyContext,
}

impl Default for KeybindingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KeybindingManager {
    /// Creates a new keybinding manager with default bindings.
    pub fn new() -> Self {
        let mut manager = Self {
            bindings: HashMap::new(),
            pending_sequence: Vec::new(),
            last_keystroke: None,
            sequence_timeout: Duration::from_millis(1000),
            current_context: KeyContext::Global,
        };
        manager.register_defaults();
        manager
    }

    /// Registers the default keybindings.
    fn register_defaults(&mut self) {
        // Global bindings
        self.bind(KeyContext::Global, KeyBinding::single(Keystroke::key(Key::C)), "compose");
        self.bind(KeyContext::Global, KeyBinding::single(Keystroke::key(Key::Slash)), "search");
        self.bind(KeyContext::Global, KeyBinding::single(Keystroke::cmd(Key::K)), "command_palette");
        self.bind(KeyContext::Global, KeyBinding::single(Keystroke::cmd(Key::Comma)), "settings");
        self.bind(KeyContext::Global, KeyBinding::single(Keystroke::key(Key::Escape)), "cancel");

        // Gmail-style navigation sequences
        self.bind(KeyContext::Global, KeyBinding::sequence(vec![
            Keystroke::key(Key::G), Keystroke::key(Key::I)
        ]), "go_inbox");
        self.bind(KeyContext::Global, KeyBinding::sequence(vec![
            Keystroke::key(Key::G), Keystroke::key(Key::S)
        ]), "go_starred");
        self.bind(KeyContext::Global, KeyBinding::sequence(vec![
            Keystroke::key(Key::G), Keystroke::key(Key::D)
        ]), "go_drafts");
        self.bind(KeyContext::Global, KeyBinding::sequence(vec![
            Keystroke::key(Key::G), Keystroke::key(Key::T)
        ]), "go_sent");
        self.bind(KeyContext::Global, KeyBinding::sequence(vec![
            Keystroke::key(Key::G), Keystroke::key(Key::A)
        ]), "go_archive");

        // Message list bindings
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::key(Key::J)), "next_message");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::key(Key::K)), "prev_message");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::key(Key::Enter)), "open_message");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::key(Key::X)), "select_message");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::key(Key::E)), "archive");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::key(Key::S)), "star");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::shift(Key::Num3)), "trash");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::key(Key::U)), "mark_unread");
        self.bind(KeyContext::MessageList, KeyBinding::single(Keystroke::shift(Key::U)), "mark_read");

        // Reading pane bindings
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::key(Key::R)), "reply");
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::shift(Key::R)), "reply_all");
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::key(Key::F)), "forward");
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::key(Key::J)), "next_in_thread");
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::key(Key::K)), "prev_in_thread");
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::cmd(Key::S)), "summarize");
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::key(Key::N)), "expand_all");
        self.bind(KeyContext::ReadingPane, KeyBinding::single(Keystroke::shift(Key::N)), "collapse_all");

        // Composer bindings
        self.bind(KeyContext::Composer, KeyBinding::single(Keystroke::cmd(Key::Enter)), "send");
        self.bind(KeyContext::Composer, KeyBinding::single(Keystroke::cmd(Key::S)), "save_draft");
        self.bind(KeyContext::Composer, KeyBinding::single(Keystroke::cmd(Key::R)), "ai_suggest");
        self.bind(KeyContext::Composer, KeyBinding::single(Keystroke::cmd(Key::D)), "discard");
        self.bind(KeyContext::Composer, KeyBinding::single(Keystroke::cmd(Key::Slash)), "toggle_markdown");

        // Command palette bindings
        self.bind(KeyContext::CommandPalette, KeyBinding::single(Keystroke::key(Key::Up)), "prev_item");
        self.bind(KeyContext::CommandPalette, KeyBinding::single(Keystroke::key(Key::Down)), "next_item");
        self.bind(KeyContext::CommandPalette, KeyBinding::single(Keystroke::key(Key::Enter)), "execute");
        self.bind(KeyContext::CommandPalette, KeyBinding::single(Keystroke::key(Key::Escape)), "close");
    }

    /// Binds a key sequence to a command.
    pub fn bind(&mut self, context: KeyContext, binding: KeyBinding, command_id: &str) {
        let context_map = self.bindings.entry(context).or_default();
        context_map.insert(binding, command_id.to_string());
    }

    /// Unbinds a key sequence.
    pub fn unbind(&mut self, context: KeyContext, binding: &KeyBinding) {
        if let Some(context_map) = self.bindings.get_mut(&context) {
            context_map.remove(binding);
        }
    }

    /// Sets the current context.
    pub fn set_context(&mut self, context: KeyContext) {
        self.current_context = context;
        // Clear pending sequence when context changes
        self.pending_sequence.clear();
    }

    /// Returns the current context.
    pub fn context(&self) -> KeyContext {
        self.current_context
    }

    /// Processes a keystroke and returns the result.
    pub fn process(&mut self, keystroke: Keystroke) -> KeyResult {
        let now = Instant::now();

        // Check if sequence timed out
        if let Some(last) = self.last_keystroke {
            if now.duration_since(last) > self.sequence_timeout {
                self.pending_sequence.clear();
            }
        }

        self.last_keystroke = Some(now);
        self.pending_sequence.push(keystroke);

        // Try to match in current context first, then global
        let contexts = if self.current_context == KeyContext::Global {
            vec![KeyContext::Global]
        } else {
            vec![self.current_context, KeyContext::Global]
        };

        for context in contexts {
            if let Some(context_bindings) = self.bindings.get(&context) {
                // Check for exact match
                let current_binding = KeyBinding::sequence(self.pending_sequence.clone());
                if let Some(command) = context_bindings.get(&current_binding) {
                    self.pending_sequence.clear();
                    return KeyResult::Matched(command.clone());
                }

                // Check if current sequence is a prefix of any binding
                let is_prefix = context_bindings.keys().any(|b| {
                    b.sequence.len() > self.pending_sequence.len()
                        && b.sequence.starts_with(&self.pending_sequence)
                });

                if is_prefix {
                    return KeyResult::Pending;
                }
            }
        }

        // No match or prefix - reset and return ignored
        self.pending_sequence.clear();
        KeyResult::Ignored
    }

    /// Cancels any pending key sequence.
    pub fn cancel_sequence(&mut self) {
        self.pending_sequence.clear();
    }

    /// Returns all bindings for a context.
    pub fn bindings_for_context(&self, context: KeyContext) -> Vec<(KeyBinding, String)> {
        self.bindings
            .get(&context)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    /// Returns all bindings for a command.
    pub fn bindings_for_command(&self, command_id: &str) -> Vec<(KeyContext, KeyBinding)> {
        let mut results = Vec::new();
        for (context, bindings) in &self.bindings {
            for (binding, cmd) in bindings {
                if cmd == command_id {
                    results.push((*context, binding.clone()));
                }
            }
        }
        results
    }

    /// Detects conflicts between bindings.
    pub fn detect_conflicts(&self) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for (context, bindings) in &self.bindings {
            // Group commands by binding
            let mut by_binding: HashMap<&KeyBinding, Vec<&String>> = HashMap::new();
            for (binding, command) in bindings {
                by_binding.entry(binding).or_default().push(command);
            }

            // Report bindings with multiple commands
            for (binding, commands) in by_binding {
                if commands.len() > 1 {
                    conflicts.push(Conflict {
                        binding: binding.clone(),
                        commands: commands.into_iter().cloned().collect(),
                        context: *context,
                    });
                }
            }
        }

        conflicts
    }

    /// Returns all registered commands and their bindings.
    pub fn all_bindings(&self) -> HashMap<String, Vec<(KeyContext, String)>> {
        let mut result: HashMap<String, Vec<(KeyContext, String)>> = HashMap::new();

        for (context, bindings) in &self.bindings {
            for (binding, command) in bindings {
                result
                    .entry(command.clone())
                    .or_default()
                    .push((*context, binding.display()));
            }
        }

        result
    }

    /// Exports bindings as a serializable config.
    pub fn export_config(&self) -> KeybindingConfig {
        let mut bindings = Vec::new();

        for (context, context_bindings) in &self.bindings {
            for (binding, command) in context_bindings {
                bindings.push(KeybindingEntry {
                    context: *context,
                    binding: binding.clone(),
                    command: command.clone(),
                });
            }
        }

        KeybindingConfig { bindings }
    }

    /// Imports bindings from config.
    pub fn import_config(&mut self, config: &KeybindingConfig) {
        for entry in &config.bindings {
            self.bind(entry.context, entry.binding.clone(), &entry.command);
        }
    }
}

/// Serializable keybinding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    /// List of keybinding entries.
    pub bindings: Vec<KeybindingEntry>,
}

/// A single keybinding entry in config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingEntry {
    /// Context for this binding.
    pub context: KeyContext,
    /// The key binding.
    pub binding: KeyBinding,
    /// Command to execute.
    pub command: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_display() {
        let simple = Keystroke::key(Key::R);
        assert_eq!(simple.to_string(), "R");

        let with_cmd = Keystroke::cmd(Key::S);
        assert_eq!(with_cmd.to_string(), "Cmd+S");

        let with_shift = Keystroke::shift(Key::R);
        assert_eq!(with_shift.to_string(), "Shift+R");
    }

    #[test]
    fn keybinding_display() {
        let single = KeyBinding::single(Keystroke::key(Key::C));
        assert_eq!(single.display(), "C");

        let sequence = KeyBinding::sequence(vec![
            Keystroke::key(Key::G),
            Keystroke::key(Key::I),
        ]);
        assert_eq!(sequence.display(), "G I");
    }

    #[test]
    fn manager_single_keystroke() {
        let mut manager = KeybindingManager::new();
        manager.set_context(KeyContext::Global);

        let result = manager.process(Keystroke::key(Key::C));
        assert_eq!(result, KeyResult::Matched("compose".to_string()));
    }

    #[test]
    fn manager_sequence_keystroke() {
        let mut manager = KeybindingManager::new();
        manager.set_context(KeyContext::Global);

        // First key should be pending
        let result1 = manager.process(Keystroke::key(Key::G));
        assert_eq!(result1, KeyResult::Pending);

        // Second key should match
        let result2 = manager.process(Keystroke::key(Key::I));
        assert_eq!(result2, KeyResult::Matched("go_inbox".to_string()));
    }

    #[test]
    fn manager_unknown_keystroke() {
        let mut manager = KeybindingManager::new();
        manager.set_context(KeyContext::Global);

        let result = manager.process(Keystroke::key(Key::Z));
        assert_eq!(result, KeyResult::Ignored);
    }

    #[test]
    fn manager_context_specific() {
        let mut manager = KeybindingManager::new();

        // In MessageList context, J should navigate
        manager.set_context(KeyContext::MessageList);
        let result = manager.process(Keystroke::key(Key::J));
        assert_eq!(result, KeyResult::Matched("next_message".to_string()));

        // In ReadingPane context, J should navigate in thread
        manager.set_context(KeyContext::ReadingPane);
        let result = manager.process(Keystroke::key(Key::J));
        assert_eq!(result, KeyResult::Matched("next_in_thread".to_string()));
    }

    #[test]
    fn manager_detect_conflicts() {
        let mut manager = KeybindingManager::new();
        // This shouldn't have conflicts with defaults
        let conflicts = manager.detect_conflicts();
        assert!(conflicts.is_empty());

        // Add a conflicting binding
        manager.bind(KeyContext::Global, KeyBinding::single(Keystroke::key(Key::C)), "other_command");
        let conflicts = manager.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn manager_export_import() {
        let manager = KeybindingManager::new();
        let config = manager.export_config();
        assert!(!config.bindings.is_empty());

        let mut new_manager = KeybindingManager {
            bindings: HashMap::new(),
            pending_sequence: Vec::new(),
            last_keystroke: None,
            sequence_timeout: Duration::from_millis(1000),
            current_context: KeyContext::Global,
        };
        new_manager.import_config(&config);
        assert!(!new_manager.bindings.is_empty());
    }
}
