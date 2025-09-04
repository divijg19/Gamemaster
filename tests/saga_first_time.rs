use gamemaster_bot::commands::saga::ui::{create_first_time_tutorial, create_saga_menu};
use gamemaster_bot::database::models::SagaProfile;

fn dummy_profile(sp: i32, has_party: bool) -> (SagaProfile, bool) {
    (
        SagaProfile {
            current_ap: 5,
            max_ap: 5,
            current_tp: 3,
            max_tp: 3,
            last_tp_update: chrono::Utc::now(),
            story_progress: sp,
        },
        has_party,
    )
}

#[test]
fn first_time_tutorial_has_expected_elements() {
    let (_embed, components) = create_first_time_tutorial();
    // Can't introspect title directly (private); rely on component presence as proxy.
    // Expect at least one button row and global nav row (we appended it at end)
    assert!(
        components.len() >= 2,
        "Expected at least two action rows (tutorial + nav)"
    );
}

#[test]
fn saga_menu_for_returning_player() {
    let (profile, has_party) = dummy_profile(2, true); // story progress > 0 and has party
    let (_embed, components) = create_saga_menu(&profile, has_party);
    // Title inaccessible; ensure we got at least one component row.
    // Expect action rows including nav row appended at end
    assert!(!components.is_empty(), "Saga menu should have components");
}

#[tokio::test]
async fn cache_stats_hit_miss_counts() {
    use gamemaster_bot::services::cache::{cache_stats, get_with_ttl, insert};
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio::sync::RwLock;
    let map: RwLock<HashMap<i32, (std::time::Instant, i32)>> = RwLock::new(HashMap::new());
    insert(&map, 1, 42).await;
    // First fetch should be hit
    let v = get_with_ttl(&map, &1, Duration::from_secs(60)).await;
    assert_eq!(v, Some(42));
    // Miss: key absent
    let v2 = get_with_ttl(&map, &2, Duration::from_secs(60)).await;
    assert!(v2.is_none());
    let (hits, misses) = cache_stats().await;
    assert!(hits >= 1, "Expected at least one cache hit");
    assert!(misses >= 1, "Expected at least one cache miss");
}
