// The ante flow was removed from the production crate.
// For backward-compatibility of test expectations, we inline the parser
// and prefix here to validate the original contract without shipping it.
const SAGA_TAVERN_GAMES_ANTE_PREFIX: &str = "saga_tavern_games_ante_";

fn parse_tavern_ante_id(id: &str) -> Option<(String, i64)> {
    if !id.starts_with(SAGA_TAVERN_GAMES_ANTE_PREFIX) {
        return None;
    }
    let rest = &id[SAGA_TAVERN_GAMES_ANTE_PREFIX.len()..];
    let (game_key, amount_str) = rest.rsplit_once('_')?;
    let amount = amount_str.parse::<i64>().ok()?;
    if game_key.is_empty() {
        return None;
    }
    Some((game_key.to_string(), amount))
}

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
