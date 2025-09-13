use gamemaster_bot::interactions::ids::{SAGA_TAVERN_GAMES_ANTE_PREFIX, parse_tavern_ante_id};

#[test]
fn parse_ante_ok_blackjack() {
    let id = format!("{}blackjack_100", SAGA_TAVERN_GAMES_ANTE_PREFIX);
    let (game, amt) = parse_tavern_ante_id(&id).expect("should parse");
    assert_eq!(game, "blackjack");
    assert_eq!(amt, 100);
}

#[test]
fn parse_ante_ok_poker_all_in() {
    let id = format!("{}poker_0", SAGA_TAVERN_GAMES_ANTE_PREFIX);
    let (game, amt) = parse_tavern_ante_id(&id).expect("should parse");
    assert_eq!(game, "poker");
    assert_eq!(amt, 0);
}

#[test]
fn parse_ante_bad() {
    assert!(parse_tavern_ante_id("saga_tavern_games_ante_").is_none());
    assert!(parse_tavern_ante_id("saga_tavern_games_ante_blackjack_").is_none());
    assert!(parse_tavern_ante_id("saga_tavern_games_ante__100").is_none());
    assert!(parse_tavern_ante_id("saga_tavern_games_ante_blackjack_x").is_none());
}
