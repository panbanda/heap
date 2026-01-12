//! Integration tests for core services.
//!
//! These tests verify that domain types and service utilities work correctly
//! across module boundaries. Each service module contains its own unit tests
//! for detailed logic testing.

use heap::domain::{system_labels, Contact, LabelId, ThreadId};
use heap::services::{ContactFilter, SnoozeDuration, ViewType};

// ============================================================================
// Domain Type Tests
// ============================================================================

#[test]
fn contact_creation_and_properties() {
    let contact = Contact::with_name("alice@example.com", "Alice Smith");

    assert_eq!(contact.email, "alice@example.com");
    assert_eq!(contact.name, Some("Alice Smith".to_string()));
    assert!(!contact.is_vip);
    assert_eq!(contact.frequency, 1); // New contacts start with frequency 1
}

#[test]
fn contact_vip_toggle() {
    let mut contact = Contact::new("vip@example.com");
    assert!(!contact.is_vip);

    contact.is_vip = true;
    assert!(contact.is_vip);
}

#[test]
fn label_id_equality() {
    let id1 = LabelId("inbox".to_string());
    let id2 = LabelId("inbox".to_string());
    let id3 = LabelId("sent".to_string());

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn thread_id_equality() {
    let id1 = ThreadId("thread-123".to_string());
    let id2 = ThreadId("thread-123".to_string());
    let id3 = ThreadId("thread-456".to_string());

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn system_labels_are_defined() {
    let inbox = system_labels::inbox();
    let sent = system_labels::sent();
    let drafts = system_labels::drafts();
    let trash = system_labels::trash();
    let spam = system_labels::spam();
    let starred = system_labels::starred();
    let archive = system_labels::archive();

    // Verify they are distinct
    assert_ne!(inbox, sent);
    assert_ne!(sent, drafts);
    assert_ne!(drafts, trash);
    assert_ne!(trash, spam);
    assert_ne!(spam, starred);
    assert_ne!(starred, archive);
}

// ============================================================================
// Snooze Duration Tests
// ============================================================================

#[test]
fn snooze_duration_later_today() {
    let now = chrono::Utc::now();
    let wake_time = SnoozeDuration::LaterToday.wake_time();

    assert!(wake_time > now);
}

#[test]
fn snooze_duration_tomorrow_is_future() {
    let now = chrono::Utc::now();
    let wake_time = SnoozeDuration::Tomorrow.wake_time();

    assert!(wake_time > now);
}

#[test]
fn snooze_duration_next_week_is_future() {
    let now = chrono::Utc::now();
    let wake_time = SnoozeDuration::NextWeek.wake_time();

    assert!(wake_time > now);
    let days_diff = (wake_time.date_naive() - now.date_naive()).num_days();
    assert!(days_diff >= 1);
    assert!(days_diff <= 8);
}

#[test]
fn snooze_duration_weekend_is_future() {
    let now = chrono::Utc::now();
    let wake_time = SnoozeDuration::ThisWeekend.wake_time();

    assert!(wake_time > now);
}

#[test]
fn snooze_duration_custom() {
    let future = chrono::Utc::now() + chrono::Duration::hours(5);
    let wake_time = SnoozeDuration::Custom(future).wake_time();

    assert_eq!(wake_time, future);
}

// ============================================================================
// View Type Tests
// ============================================================================

#[test]
fn view_type_folder_names() {
    assert_eq!(ViewType::Inbox.folder_name(), "INBOX");
    assert_eq!(ViewType::Sent.folder_name(), "[Gmail]/Sent Mail");
    assert_eq!(ViewType::Drafts.folder_name(), "[Gmail]/Drafts");
    assert_eq!(ViewType::Trash.folder_name(), "[Gmail]/Trash");
}

// ============================================================================
// Service Type Tests
// ============================================================================

#[test]
fn contact_filter_builder_pattern() {
    let filter = ContactFilter::new()
        .vip()
        .search("alice")
        .min_frequency(5)
        .limit(10);

    assert!(filter.vip_only);
    assert_eq!(filter.search, Some("alice".to_string()));
    assert_eq!(filter.min_frequency, Some(5));
    assert_eq!(filter.limit, Some(10));
}

#[test]
fn contact_filter_matches_vip() {
    let mut vip_contact = Contact::new("vip@example.com");
    vip_contact.is_vip = true;

    let normal_contact = Contact::new("normal@example.com");

    let filter = ContactFilter::new().vip();

    assert!(filter.matches(&vip_contact));
    assert!(!filter.matches(&normal_contact));
}

#[test]
fn contact_filter_matches_search() {
    let contact = Contact::with_name("alice@example.com", "Alice Smith");
    let other = Contact::with_name("bob@example.com", "Bob Jones");

    let filter = ContactFilter::new().search("alice");

    assert!(filter.matches(&contact));
    assert!(!filter.matches(&other));
}

#[test]
fn contact_filter_matches_frequency() {
    let mut frequent = Contact::new("frequent@example.com");
    frequent.frequency = 10;

    let mut rare = Contact::new("rare@example.com");
    rare.frequency = 2;

    let filter = ContactFilter::new().min_frequency(5);

    assert!(filter.matches(&frequent));
    assert!(!filter.matches(&rare));
}
