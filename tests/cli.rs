use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const ROUND: &str = r#"
cards_played: [A♥]
cards_held_in_hand: []
jokers: []
"#;

#[test]
fn scores_round_from_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should follow the Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ortalab-{unique}.yml"));
    fs::write(&path, ROUND).expect("temporary round should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_ortalab"))
        .arg(&path)
        .output()
        .expect("Ortalab should start");
    fs::remove_file(path).expect("temporary round should be removable");

    assert!(output.status.success());
    assert_eq!(output.stdout, b"16\n");
}

#[test]
fn scores_round_from_standard_input() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ortalab"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Ortalab should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(ROUND.as_bytes())
        .expect("round should be written to stdin");

    let output = child
        .wait_with_output()
        .expect("Ortalab should finish successfully");

    assert!(output.status.success());
    assert_eq!(output.stdout, b"16\n");
}

use ortalab::score;
use ortalib::Round;

fn round(yaml: &str) -> Round {
    serde_yaml::from_str(yaml).expect("test round should be valid YAML")
}

fn total(yaml: &str) -> f64 {
    let (chips, mult) = score(round(yaml));
    (chips * mult).floor()
}

#[test]
fn scores_all_card_modifiers_in_order() {
    assert_eq!(
        total(
            r#"
cards_played:
  - A♥ Bonus Foil
  - K♠ Mult Holographic
  - Q♦ Glass Polychrome
  - J♣ Wild
  - 10♥ Steel
cards_held_in_hand:
  - K♠ Steel Foil
  - 7♠ Steel Holographic
  - 3♠ Steel Polychrome
jokers: []
"#
        ),
        29342.0
    );
}

#[test]
fn stone_always_scores_but_has_no_rank_or_suit() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥, 3♥, 4♥, 5♥, A♠ Stone Foil]
cards_held_in_hand: []
jokers: [Hack, Greedy Joker, Scary Face]
"#
        ),
        115.0
    );
}

#[test]
fn easy_pair_jokers_activate_in_order() {
    assert_eq!(
        total(
            r#"
cards_played: [9♥, 9♠]
cards_held_in_hand: []
jokers: [Joker, Jolly Joker, Sly Joker, Abstract Joker, Blackboard]
"#
        ),
        6786.0
    );
}

#[test]
fn easy_three_kind_jokers_activate() {
    assert_eq!(
        total(
            r#"
cards_played: [10♥, 10♠, 10♦]
cards_held_in_hand: []
jokers: [Zany Joker, Wily Joker]
"#
        ),
        2400.0
    );
}

#[test]
fn easy_two_pair_jokers_activate() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥, 2♠, 3♦, 3♣]
cards_held_in_hand: []
jokers: [Mad Joker, Clever Joker]
"#
        ),
        1320.0
    );
}

#[test]
fn easy_straight_jokers_activate() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥, 3♠, 4♦, 5♣, 6♥]
cards_held_in_hand: []
jokers: [The Order, Devious Joker]
"#
        ),
        1800.0
    );
}

#[test]
fn easy_flush_jokers_activate() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥, 4♥, 6♥, 8♥, 10♥]
cards_held_in_hand: []
jokers: [The Tribe, Crafty Joker]
"#
        ),
        1160.0
    );
}

#[test]
fn diamond_ace_triggers_rank_and_suit_jokers() {
    assert_eq!(
        total(
            r#"
cards_played: [A♦]
cards_held_in_hand: []
jokers: [Greedy Joker, Fibonacci, Odd Todd, Scholar]
"#
        ),
        1072.0
    );
}

#[test]
fn heart_face_triggers_face_jokers() {
    assert_eq!(
        total(
            r#"
cards_played: [K♥]
cards_held_in_hand: []
jokers: [Lusty Joker, Scary Face, Photograph, Smiley Face]
"#
        ),
        585.0
    );
}

#[test]
fn spade_ten_triggers_scored_jokers() {
    assert_eq!(
        total(
            r#"
cards_played: [10♠]
cards_held_in_hand: []
jokers: [Arrowhead, Even Steven, Walkie Talkie]
"#
        ),
        675.0
    );
}

#[test]
fn club_eight_triggers_scored_jokers() {
    assert_eq!(
        total(
            r#"
cards_played: [8♣]
cards_held_in_hand: []
jokers: [Onyx Agate, Fibonacci, Even Steven]
"#
        ),
        260.0
    );
}

#[test]
fn held_jokers_and_mime_retrigger_per_card() {
    assert_eq!(
        total(
            r#"
cards_played: [A♥]
cards_held_in_hand: [2♣, K♣, Q♣, 7♣ Steel]
jokers: [Raised Fist, Baron, Shoot The Moon, Mime]
"#
        ),
        1665.0
    );
}

#[test]
fn four_fingers_supports_four_card_hands() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥, 3♠, 4♦, 5♣, 10♥]
cards_held_in_hand: []
jokers: [Four Fingers]
"#
        ),
        176.0
    );
    assert_eq!(
        total(
            r#"
cards_played: [2♥, 4♥, 6♥, 8♥, K♠]
cards_held_in_hand: []
jokers: [Four Fingers]
"#
        ),
        220.0
    );
}

#[test]
fn four_finger_straight_flush_can_use_different_cards() {
    assert_eq!(
        total(
            r#"
cards_played: [2♠, 3♠, 4♦, 5♠, 10♠]
cards_held_in_hand: []
jokers: [Four Fingers]
"#
        ),
        992.0
    );
}

#[test]
fn splash_scores_every_played_card() {
    assert_eq!(
        total(
            r#"
cards_played: [A♥, 3♠, 2♦]
cards_held_in_hand: []
jokers: [Splash]
"#
        ),
        21.0
    );
}

#[test]
fn scored_retriggers_repeat_the_complete_card() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥]
cards_held_in_hand: []
jokers: [Hack, Hanging Chad]
"#
        ),
        13.0
    );
    assert_eq!(
        total(
            r#"
cards_played: [K♥]
cards_held_in_hand: []
jokers: [Sock And Buskin]
"#
        ),
        25.0
    );
}

#[test]
fn pareidolia_changes_face_checks_and_retriggers() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥]
cards_held_in_hand: []
jokers: [Pareidolia, Sock And Buskin]
"#
        ),
        9.0
    );
    assert_eq!(
        total(
            r#"
cards_played: [2♥]
cards_held_in_hand: []
jokers: [Pareidolia, Scary Face, Smiley Face]
"#
        ),
        222.0
    );
}

#[test]
fn joker_editions_apply_around_independent_ability() {
    assert_eq!(
        total(
            r#"
cards_played: [A♥]
cards_held_in_hand: []
jokers: [Joker Foil, Joker Holographic, Joker Polychrome]
"#
        ),
        2277.0
    );
}

#[test]
fn blackboard_accepts_wild_cards_and_flower_pot_scores() {
    assert_eq!(
        total(
            r#"
cards_played: [2♦, 3♣, 4♥, 5♠, 6♦]
cards_held_in_hand: [A♥ Wild]
jokers: [Blackboard, Flower Pot]
"#
        ),
        1800.0
    );
    assert_eq!(
        total(
            r#"
cards_played: [2♦, 3♣, 4♥, 5♠, 6♦]
cards_held_in_hand: [A♣]
jokers: [Blackboard]
"#
        ),
        600.0
    );
}

#[test]
fn scores_every_poker_hand() {
    let cases = [
        ("[A♠, Q♦, 9♦, 4♣, 3♦]", 16.0),
        ("[K♠, 9♠, 9♦, 6♥, 3♦]", 56.0),
        ("[A♥, A♦, Q♣, 4♥, 4♣]", 100.0),
        ("[10♠, 10♣, 10♦, 6♥, 5♦]", 180.0),
        ("[J♦, 10♣, 9♣, 8♠, 7♥]", 296.0),
        ("[A♥, K♥, 10♥, 5♥, 4♥]", 300.0),
        ("[K♥, K♣, K♦, 2♠, 2♦]", 296.0),
        ("[J♠, J♥, J♣, J♦, 3♣]", 700.0),
        ("[Q♠, J♠, 10♠, 9♠, 8♠]", 1176.0),
        ("[A♠, A♥, A♥, A♣, A♦]", 2100.0),
        ("[7♦, 7♦, 7♦, 4♦, 4♦]", 2366.0),
        ("[A♠, A♠, A♠, A♠, A♠]", 3440.0),
    ];
    for (cards, expected) in cases {
        let yaml = format!("cards_played: {cards}\ncards_held_in_hand: []\njokers: []\n");
        assert_eq!(total(&yaml), expected, "failed for {cards}");
    }
}

#[test]
fn handles_ace_and_shortcut_boundaries() {
    assert_eq!(
        total("cards_played: [A♠, 2♦, 3♦, 4♠, 5♠]\ncards_held_in_hand: []\njokers: []\n"),
        220.0
    );
    assert_eq!(
        total("cards_played: [Q♠, K♦, A♦, 2♦, 3♠]\ncards_held_in_hand: []\njokers: []\n"),
        16.0
    );
    assert_eq!(
        total("cards_played: [2♥, 4♠, 6♦, 8♣, 10♥]\ncards_held_in_hand: []\njokers: [Shortcut]\n"),
        240.0
    );
    assert_eq!(
        total("cards_played: [2♥, 5♠, 7♦, 9♣, J♥]\ncards_held_in_hand: []\njokers: [Shortcut]\n"),
        15.0
    );
}

#[test]
fn scores_smeared_flower_pot_and_wild_cards() {
    assert_eq!(
        total(
            "cards_played: [2♥, 4♦, 6♥, 8♦, 10♥]\ncards_held_in_hand: []\njokers: [Smeared Joker]\n"
        ),
        260.0
    );
    assert_eq!(
        total("cards_played: [2♦, 3♣, 4♥, 5♠, 6♦]\ncards_held_in_hand: []\njokers: [Flower Pot]\n"),
        600.0
    );
    assert_eq!(
        total(
            "cards_played: [Q♠ Wild, Q♣, 3♣, 3♣]\ncards_held_in_hand: []\njokers: [Smeared Joker, Flower Pot]\n"
        ),
        132.0
    );
    assert_eq!(
        total("cards_played: [2♥, 5♣ Wild, 5♥, 6♥, 6♠ Wild]\ncards_held_in_hand: []\njokers: []\n"),
        236.0
    );
}

#[test]
fn resolves_blueprint_chains_end_to_end() {
    assert_eq!(
        total("cards_played: [A♥]\ncards_held_in_hand: []\njokers: [Blueprint, Joker]\n"),
        144.0
    );
    assert_eq!(
        total(
            "cards_played: [A♥]\ncards_held_in_hand: []\njokers: [Blueprint, Blueprint, Blueprint, Joker]\n"
        ),
        272.0
    );
}
