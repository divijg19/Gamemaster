use gamemaster_bot::database::models::PlayerUnit;
use gamemaster_bot::saga::{core::calculate_tp_recharge, leveling::handle_unit_leveling};

#[test]
fn tp_recharge_no_change_before_interval() {
    let profile = gamemaster_bot::database::models::SagaProfile {
        current_ap: 0,
        max_ap: 0,
        current_tp: 2,
        max_tp: 5,
        last_tp_update: chrono::Utc::now(),
        story_progress: 0,
    };
    let (tp, update) = calculate_tp_recharge(&profile);
    assert_eq!(tp, 2);
    assert!(!update);
}

#[test]
fn leveling_multi_level_gain() {
    let unit = PlayerUnit {
        player_unit_id: 1,
        user_id: 1,
        unit_id: 1,
        nickname: None,
        current_level: 1,
        current_xp: 0,
        current_attack: 10,
        current_defense: 5,
        current_health: 30,
        is_in_party: false,
        is_training: false,
        training_stat: None,
        training_ends_at: None,
        name: "Unit".into(),
        rarity: gamemaster_bot::database::models::UnitRarity::Common,
    };
    let res = handle_unit_leveling(&unit, 1000); // large xp for multiple ups
    assert!(res.did_level_up);
    assert!(res.new_level > 1);
    assert!(res.stat_gains.0 >= 2);
}
