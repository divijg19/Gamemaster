//! Ensures help embed lists all command names by category formatting.
use gamemaster_bot::commands::help::all_command_names;

#[test]
fn help_command_names_unique_and_present() {
    let names = all_command_names();
    // Ensure uniqueness
    let mut sorted = names.clone();
    sorted.sort();
    for w in sorted.windows(2) {
        assert_ne!(w[0], w[1], "Duplicate help command name: {}", w[0]);
    }
    // Basic expected core commands (spot check; list can evolve)
    let expected = [
        "help",
        "ping",
        "saga",
        "quests",
        "questlog",
        "party",
        "bond",
        "train",
        "contracts",
        "research",
        "bestiary",
        "progress",
        "open",
        "profile",
        "work",
        "inventory",
        "sell",
        "shop",
        "give",
        "craft",
        "tasks",
        "leaderboard",
        "rps",
        "blackjack",
        "poker",
        "prefix",
        "adminutil",
        "config",
    ];
    for e in expected {
        assert!(sorted.contains(&e), "Missing help entry for `{}`", e);
    }
}
