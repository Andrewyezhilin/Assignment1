use super::*;

fn round(yaml: &str) -> Round {
    serde_yaml::from_str(yaml).expect("test round should be valid YAML")
}

fn total(yaml: &str) -> f64 {
    let (chips, mult) = score(round(yaml));
    (chips * mult).floor()
}

fn hand(yaml: &str) -> PokerHand {
    let round = round(yaml);
    determine_poker_hand(&round.cards_played, &round.jokers)
}

macro_rules! assert_hand {
    ($yaml:literal, $expected:expr) => {
        assert_eq!(
            hand(concat!(
                "cards_played: ",
                $yaml,
                "\ncards_held_in_hand: []\njokers: []\n"
            )),
            $expected
        );
    };
}

#[test]
fn recognises_every_poker_hand() {
    assert_hand!("[A♠, Q♦, 9♦, 4♣, 3♦]", PokerHand::HighCard);
    assert_hand!("[K♠, 9♠, 9♦, 6♥, 3♦]", PokerHand::Pair);
    assert_hand!("[A♥, A♦, Q♣, 4♥, 4♣]", PokerHand::TwoPair);
    assert_hand!("[10♠, 10♣, 10♦, 6♥, 5♦]", PokerHand::ThreeOfAKind);
    assert_hand!("[J♦, 10♣, 9♣, 8♠, 7♥]", PokerHand::Straight);
    assert_hand!("[A♥, K♥, 10♥, 5♥, 4♥]", PokerHand::Flush);
    assert_hand!("[K♥, K♣, K♦, 2♠, 2♦]", PokerHand::FullHouse);
    assert_hand!("[J♠, J♥, J♣, J♦, 3♣]", PokerHand::FourOfAKind);
    assert_hand!("[Q♠, J♠, 10♠, 9♠, 8♠]", PokerHand::StraightFlush);
    assert_hand!("[A♠, A♥, A♥, A♣, A♦]", PokerHand::FiveOfAKind);
    assert_hand!("[7♦, 7♦, 7♦, 4♦, 4♦]", PokerHand::FlushHouse);
    assert_hand!("[A♠, A♠, A♠, A♠, A♠]", PokerHand::FlushFive);
}

#[test]
fn recognises_ace_low_but_not_wrapping_straights() {
    assert_hand!("[A♠, 2♦, 3♦, 4♠, 5♠]", PokerHand::Straight);
    assert_hand!("[Q♠, K♦, A♦, 2♦, 3♠]", PokerHand::HighCard);
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
fn raised_fist_uses_rightmost_lowest_card() {
    let held = round(
        r#"
cards_played: [A♥]
cards_held_in_hand: [2♣, 2♦, 3♠]
jokers: []
"#,
    )
    .cards_held_in_hand;
    assert_eq!(get_lowest_rank_card_index(&held), Some(1));
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
fn shortcut_allows_single_rank_gaps() {
    assert_eq!(
        total(
            r#"
cards_played: [2♥, 4♠, 6♦, 8♣, 10♥]
cards_held_in_hand: []
jokers: [Shortcut]
"#
        ),
        240.0
    );
    assert!(!straight_candidate(
        &round(
            r#"
cards_played: [2♥, 5♠, 7♦, 9♣, J♥]
cards_held_in_hand: []
jokers: []
"#
        )
        .cards_played,
        &[0, 1, 2, 3, 4],
        true
    ));
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
fn smeared_joker_changes_flush_and_suit_effects() {
    assert_eq!(
        hand(
            r#"
cards_played: [2♥, 4♦, 6♥, 8♦, 10♥]
cards_held_in_hand: []
jokers: [Smeared Joker]
"#
        ),
        PokerHand::Flush
    );
    assert_eq!(
        total(
            r#"
cards_played: [2♦]
cards_held_in_hand: []
jokers: [Smeared Joker, Lusty Joker]
"#
        ),
        28.0
    );
}

#[test]
fn flower_pot_requires_four_distinct_cards() {
    let success = round(
        r#"
cards_played: [2♦, 3♣, 4♥, 5♠]
cards_held_in_hand: []
jokers: []
"#,
    );
    assert!(flower_pot_matches(&success.cards_played, false));

    let failure = round(
        r#"
cards_played: [Q♠ Wild, Q♣, 3♣, 3♣]
cards_held_in_hand: []
jokers: []
"#,
    );
    assert!(!flower_pot_matches(&failure.cards_played, true));
}

#[test]
fn blueprint_resolves_chains_but_not_passive_jokers() {
    assert_eq!(
        total(
            r#"
cards_played: [A♥]
cards_held_in_hand: []
jokers: [Blueprint, Joker]
"#
        ),
        144.0
    );

    let copied = round(
        r#"
cards_played: [A♥]
cards_held_in_hand: []
jokers: [Blueprint, Blueprint, Joker]
"#,
    );
    assert_eq!(
        effective_jokers(&copied.jokers).collect::<Vec<_>>(),
        vec![Joker::Joker, Joker::Joker, Joker::Joker]
    );

    let passive = round(
        r#"
cards_played: [A♥]
cards_held_in_hand: []
jokers: [Blueprint, Splash]
"#,
    );
    assert_eq!(effective_joker_at(&passive.jokers, 0), None);
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
fn wild_cards_can_complete_flushes() {
    assert_eq!(
        hand(
            r#"
cards_played: [2♥, 5♣ Wild, 5♥, 6♥, 6♠ Wild]
cards_held_in_hand: []
jokers: []
"#
        ),
        PokerHand::Flush
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
fn scored_indices_cover_complete_and_rank_hands() {
    for yaml in [
        r#"
cards_played: [7♦, 7♦, 7♦, 4♦, 4♦]
cards_held_in_hand: []
jokers: []
"#,
        r#"
cards_played: [A♠, A♠, A♠, A♠, A♠]
cards_held_in_hand: []
jokers: []
"#,
        r#"
cards_played: [K♥, K♣, K♦, 2♠, 2♦]
cards_held_in_hand: []
jokers: []
"#,
        r#"
cards_played: [Q♠, J♠, 10♠, 9♠, 8♠]
cards_held_in_hand: []
jokers: []
"#,
        r#"
cards_played: [A♠, A♥, A♥, A♣, A♦]
cards_held_in_hand: []
jokers: []
"#,
        r#"
cards_played: [J♠, J♥, J♣, J♦, 3♣]
cards_held_in_hand: []
jokers: []
"#,
    ] {
        let round = round(yaml);
        let hand = determine_poker_hand(&round.cards_played, &round.jokers);
        assert!(!get_scored_indices(&round.cards_played, hand, &round.jokers).is_empty());
    }
}

#[test]
fn missing_rank_group_has_no_scored_indices() {
    let round = round(
        r#"
cards_played: [A♠]
cards_held_in_hand: []
jokers: []
"#,
    );
    let counts = count_ranks(&round.cards_played);
    assert!(rank_group_indices(&round.cards_played, &counts, 5).is_empty());
}
