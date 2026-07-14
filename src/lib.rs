use std::collections::HashMap;

use ortalib::{
    Card, Chips, Edition, Enhancement, Joker, JokerCard, Mult, PokerHand, Rank, Round, Suit,
};

pub fn score(round: Round) -> (Chips, Mult) {
    let played_cards = round.cards_played.clone();

    let best_hand = determine_poker_hand(&played_cards, &round.jokers);
    let (mut total_chips, mut total_mult) = best_hand.hand_value();

    let mut scored_indices = get_scored_indices(&played_cards, best_hand, &round.jokers);
    let splash_active = round.jokers.iter().any(|j| j.joker == Joker::Splash);
    for (index, card) in played_cards.iter().enumerate() {
        if (splash_active || card_is_stone(card)) && !scored_indices.contains(&index) {
            scored_indices.push(index);
        }
    }
    scored_indices.sort_unstable();

    let first_face_index = scored_indices
        .iter()
        .copied()
        .find(|&index| is_card_face(played_cards[index], &round));
    for (position, &index) in scored_indices.iter().enumerate() {
        apply_card_scoring(
            &round,
            played_cards[index],
            &mut total_chips,
            &mut total_mult,
            position == 0,
            Some(index) == first_face_index,
        );
    }

    let mime_count = effective_jokers(&round.jokers)
        .filter(|&joker| joker == Joker::Mime)
        .count();
    for (held_index, held_card) in round.cards_held_in_hand.iter().enumerate() {
        for _ in 0..=mime_count {
            if held_card.enhancement == Some(Enhancement::Steel) {
                total_mult *= 1.5;
            }
            for joker in effective_jokers(&round.jokers) {
                apply_on_held_joker(
                    joker,
                    held_index,
                    *held_card,
                    &round.cards_held_in_hand,
                    &mut total_mult,
                );
            }
        }
    }

    let scored_cards: Vec<Card> = scored_indices
        .iter()
        .map(|&index| played_cards[index])
        .collect();
    for (index, joker) in round.jokers.iter().enumerate() {
        if let Some(edition) = joker.edition {
            match edition {
                Edition::Foil => total_chips += 50.0,
                Edition::Holographic => total_mult += 10.0,
                Edition::Polychrome => {}
            }
        }
        if let Some(ability) = effective_joker_at(&round.jokers, index) {
            apply_independent_joker(
                ability,
                &played_cards,
                &scored_cards,
                &round.cards_held_in_hand,
                &round.jokers,
                &mut total_chips,
                &mut total_mult,
            );
        }
        if joker.edition == Some(Edition::Polychrome) {
            total_mult *= 1.5;
        }
    }

    (total_chips, total_mult)
}

/// 应用卡牌计分逻辑（包括 On Scored Jokers 和 Retriggers）
fn apply_card_scoring(
    round: &Round,
    card: Card,
    total_chips: &mut Chips,
    total_mult: &mut Mult,
    is_first_card: bool,
    is_first_face: bool,
) {
    trigger_scored_card(round, card, total_chips, total_mult, is_first_face);

    let retrigger_count: usize = effective_jokers(&round.jokers)
        .map(|joker| match joker {
            Joker::SockAndBuskin if is_card_face(card, round) => 1,
            Joker::Hack
                if !card_is_stone(&card)
                    && matches!(card.rank, Rank::Two | Rank::Three | Rank::Four | Rank::Five) =>
            {
                1
            }
            Joker::HangingChad if is_first_card => 2,
            _ => 0,
        })
        .sum();
    for _ in 0..retrigger_count {
        trigger_scored_card(round, card, total_chips, total_mult, is_first_face);
    }
}

fn trigger_scored_card(
    round: &Round,
    card: Card,
    total_chips: &mut Chips,
    total_mult: &mut Mult,
    is_first_face: bool,
) {
    // 步骤 2.1：基础筹码
    if card.enhancement == Some(Enhancement::Stone) {
        *total_chips += 50.0;
    } else {
        *total_chips += card.rank.rank_value();
    }

    // 步骤 2.2：卡牌增强
    if let Some(enhancement) = card.enhancement {
        match enhancement {
            Enhancement::Bonus => *total_chips += 30.0,
            Enhancement::Mult => *total_mult += 4.0,
            Enhancement::Glass => *total_mult *= 2.0,
            _ => {}
        }
    }

    // 步骤 2.3：卡牌版本
    if let Some(edition) = card.edition {
        match edition {
            Edition::Foil => *total_chips += 50.0,
            Edition::Holographic => *total_mult += 10.0,
            Edition::Polychrome => *total_mult *= 1.5,
        }
    }

    for joker in effective_jokers(&round.jokers) {
        apply_on_scored_joker(joker, card, round, total_chips, total_mult, is_first_face);
    }
}

/// 检查卡是否是面卡（考虑 Pareidolia）
fn is_card_face(card: Card, round: &Round) -> bool {
    if card.enhancement == Some(Enhancement::Stone) {
        return false;
    }
    card.rank.is_face() || round.jokers.iter().any(|j| j.joker == Joker::Pareidolia)
}

/// 应用 On Scored 类型的 Joker 效果
fn apply_on_scored_joker(
    joker: Joker,
    card: Card,
    round: &Round,
    total_chips: &mut Chips,
    total_mult: &mut Mult,
    is_first_face: bool,
) {
    if card_is_stone(&card) {
        return;
    }
    let smeared = round
        .jokers
        .iter()
        .any(|joker| joker.joker == Joker::SmearedJoker);
    match joker {
        Joker::GreedyJoker => {
            if card_matches_suit(card, Suit::Diamonds, smeared) {
                *total_mult += 3.0;
            }
        }

        Joker::LustyJoker => {
            if card_matches_suit(card, Suit::Hearts, smeared) {
                *total_mult += 3.0;
            }
        }

        Joker::Arrowhead => {
            if card_matches_suit(card, Suit::Spades, smeared) {
                *total_chips += 50.0;
            }
        }

        Joker::OnyxAgate => {
            if card_matches_suit(card, Suit::Clubs, smeared) {
                *total_mult += 7.0;
            }
        }

        Joker::Fibonacci => match card.rank {
            Rank::Ace | Rank::Two | Rank::Three | Rank::Five | Rank::Eight => {
                *total_mult += 8.0;
            }
            _ => {}
        },

        Joker::ScaryFace => {
            if is_card_face(card, round) {
                *total_chips += 30.0;
            }
        }

        Joker::EvenSteven => match card.rank {
            Rank::Two | Rank::Four | Rank::Six | Rank::Eight | Rank::Ten => {
                *total_mult += 4.0;
            }
            _ => {}
        },

        Joker::OddTodd => match card.rank {
            Rank::Ace | Rank::Three | Rank::Five | Rank::Seven | Rank::Nine => {
                *total_chips += 31.0;
            }
            _ => {}
        },

        Joker::Scholar => {
            if card.rank == Rank::Ace {
                *total_chips += 20.0;
                *total_mult += 4.0;
            }
        }

        Joker::WalkieTalkie => match card.rank {
            Rank::Ten | Rank::Four => {
                *total_chips += 10.0;
                *total_mult += 4.0;
            }
            _ => {}
        },

        Joker::Photograph => {
            if is_first_face {
                *total_mult *= 2.0;
            }
        }

        Joker::SmileyFace if is_card_face(card, round) => {
            *total_mult += 5.0;
        }

        _ => {}
    }
}

/// 应用 On Held 类型的 Joker 效果
fn apply_on_held_joker(
    joker: Joker,
    held_index: usize,
    held_card: Card,
    held_cards: &[Card],
    total_mult: &mut Mult,
) {
    if card_is_stone(&held_card) {
        return;
    }
    match joker {
        Joker::RaisedFist => {
            if Some(held_index) == get_lowest_rank_card_index(held_cards) {
                *total_mult += 2.0 * held_card.rank.rank_value();
            }
        }

        Joker::Baron => {
            if held_card.rank == Rank::King {
                *total_mult *= 1.5;
            }
        }

        Joker::ShootTheMoon if held_card.rank == Rank::Queen => {
            *total_mult += 13.0;
        }

        _ => {}
    }
}

/// 应用 Independent 类型的 Joker 效果
fn apply_independent_joker(
    joker: Joker,
    played_cards: &[Card],
    scored_cards: &[Card],
    held_cards: &[Card],
    jokers: &[JokerCard],
    total_chips: &mut Chips,
    total_mult: &mut Mult,
) {
    let four_fingers = jokers.iter().any(|joker| joker.joker == Joker::FourFingers);
    let shortcut = jokers.iter().any(|joker| joker.joker == Joker::Shortcut);
    let smeared = jokers
        .iter()
        .any(|joker| joker.joker == Joker::SmearedJoker);
    match joker {
        Joker::Joker => {
            *total_mult += 4.0;
        }

        Joker::JollyJoker => {
            if contains_pair(played_cards) {
                *total_mult += 8.0;
            }
        }

        Joker::ZanyJoker => {
            if contains_three_of_a_kind(played_cards) {
                *total_mult += 12.0;
            }
        }

        Joker::MadJoker => {
            if contains_two_pair(played_cards) {
                *total_mult += 10.0;
            }
        }

        Joker::TheOrder => {
            if find_straight_indices(played_cards, four_fingers, shortcut).is_some() {
                *total_mult *= 3.0;
            }
        }

        Joker::TheTribe => {
            if find_flush_indices(played_cards, four_fingers, smeared).is_some() {
                *total_mult *= 2.0;
            }
        }

        Joker::SlyJoker => {
            if contains_pair(played_cards) {
                *total_chips += 50.0;
            }
        }

        Joker::WilyJoker => {
            if contains_three_of_a_kind(played_cards) {
                *total_chips += 100.0;
            }
        }

        Joker::CleverJoker => {
            if contains_two_pair(played_cards) {
                *total_chips += 80.0;
            }
        }

        Joker::DeviousJoker => {
            if find_straight_indices(played_cards, four_fingers, shortcut).is_some() {
                *total_chips += 100.0;
            }
        }

        Joker::CraftyJoker => {
            if find_flush_indices(played_cards, four_fingers, smeared).is_some() {
                *total_chips += 80.0;
            }
        }

        Joker::AbstractJoker => {
            *total_mult += 3.0 * jokers.len() as f64;
        }

        Joker::Blackboard => {
            if held_cards.iter().all(|c| {
                if c.enhancement == Some(Enhancement::Wild) {
                    true
                } else {
                    c.suit == Suit::Spades || c.suit == Suit::Clubs
                }
            }) {
                *total_mult *= 3.0;
            }
        }

        Joker::FlowerPot => {
            if flower_pot_matches(scored_cards, smeared) {
                *total_mult *= 3.0;
            }
        }

        Joker::FourFingers
        | Joker::Shortcut
        | Joker::Mime
        | Joker::Pareidolia
        | Joker::Splash
        | Joker::SockAndBuskin
        | Joker::Hack
        | Joker::HangingChad
        | Joker::SmearedJoker
        | Joker::Blueprint => {
            // 这些是效果类 Joker，不在这里给出奖励
        }

        _ => {}
    }
}

fn card_matches_suit(card: Card, suit: Suit, smeared: bool) -> bool {
    if card_is_stone(&card) {
        false
    } else if card.enhancement == Some(Enhancement::Wild) {
        true
    } else if smeared {
        matches!(
            (card.suit, suit),
            (Suit::Hearts | Suit::Diamonds, Suit::Hearts | Suit::Diamonds)
                | (Suit::Spades | Suit::Clubs, Suit::Spades | Suit::Clubs)
        )
    } else {
        card.suit == suit
    }
}

fn flower_pot_matches(cards: &[Card], smeared: bool) -> bool {
    const SUITS: [Suit; 4] = [Suit::Diamonds, Suit::Clubs, Suit::Hearts, Suit::Spades];

    fn assign(suit_index: usize, cards: &[Card], smeared: bool, used: &mut [bool]) -> bool {
        if suit_index == SUITS.len() {
            return true;
        }
        for (card_index, card) in cards.iter().enumerate() {
            if !used[card_index] && card_matches_suit(*card, SUITS[suit_index], smeared) {
                used[card_index] = true;
                if assign(suit_index + 1, cards, smeared, used) {
                    return true;
                }
                used[card_index] = false;
            }
        }
        false
    }

    cards.len() >= 4 && assign(0, cards, smeared, &mut vec![false; cards.len()])
}

fn effective_jokers(jokers: &[JokerCard]) -> impl Iterator<Item = Joker> + '_ {
    (0..jokers.len()).filter_map(|index| effective_joker_at(jokers, index))
}

fn effective_joker_at(jokers: &[JokerCard], source: usize) -> Option<Joker> {
    let mut index = source;
    while jokers.get(index)?.joker == Joker::Blueprint {
        index += 1;
    }
    let copied = index != source;
    let ability = jokers.get(index)?.joker;
    if copied
        && matches!(
            ability,
            Joker::FourFingers
                | Joker::Shortcut
                | Joker::Pareidolia
                | Joker::Splash
                | Joker::SmearedJoker
        )
    {
        None
    } else {
        Some(ability)
    }
}

/// 获取手中最低点数的卡
fn get_lowest_rank_card_index(cards: &[Card]) -> Option<usize> {
    cards
        .iter()
        .enumerate()
        .filter(|(_, card)| !card_is_stone(card))
        .min_by_key(|(index, card)| (rank_order(card.rank), std::cmp::Reverse(*index)))
        .map(|(index, _)| index)
}

fn rank_order(rank: Rank) -> u8 {
    match rank {
        Rank::Ace => 14,
        Rank::Two => 2,
        Rank::Three => 3,
        Rank::Four => 4,
        Rank::Five => 5,
        Rank::Six => 6,
        Rank::Seven => 7,
        Rank::Eight => 8,
        Rank::Nine => 9,
        Rank::Ten => 10,
        Rank::Jack => 11,
        Rank::Queen => 12,
        Rank::King => 13,
    }
}

/// 检查是否存在 Pair
fn contains_pair(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    counts.values().any(|&count| count >= 2)
}

/// 检查是否存在 Three of a Kind
fn contains_three_of_a_kind(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    counts.values().any(|&count| count >= 3)
}

/// 检查是否存在 Two Pair
fn contains_two_pair(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    let pair_count = counts.values().filter(|&&count| count >= 2).count();
    pair_count >= 2
}

/// 从高到低判定当前打出的牌属于哪种最佳牌型
fn determine_poker_hand(cards: &[Card], jokers: &[JokerCard]) -> PokerHand {
    let four_fingers = jokers.iter().any(|j| j.joker == Joker::FourFingers);
    let shortcut = jokers.iter().any(|j| j.joker == Joker::Shortcut);
    let smeared = jokers.iter().any(|j| j.joker == Joker::SmearedJoker);

    if is_flush_five(cards, smeared) {
        return PokerHand::FlushFive;
    }
    if is_flush_house(cards, smeared) {
        return PokerHand::FlushHouse;
    }
    if is_five_of_a_kind(cards) {
        return PokerHand::FiveOfAKind;
    }
    if is_straight_flush(cards, four_fingers, shortcut, smeared) {
        return PokerHand::StraightFlush;
    }
    if is_four_of_a_kind(cards) {
        return PokerHand::FourOfAKind;
    }
    if is_full_house(cards) {
        return PokerHand::FullHouse;
    }
    if is_flush(cards, four_fingers, smeared) {
        return PokerHand::Flush;
    }
    if is_straight(cards, four_fingers, shortcut) {
        return PokerHand::Straight;
    }
    if is_three_of_a_kind(cards) {
        return PokerHand::ThreeOfAKind;
    }
    if is_two_pair(cards) {
        return PokerHand::TwoPair;
    }
    if is_pair(cards) {
        return PokerHand::Pair;
    }

    PokerHand::HighCard
}

// ==================== 牌型辅助判定函数 ====================

fn count_ranks(cards: &[Card]) -> HashMap<Rank, usize> {
    let mut counts = HashMap::new();
    for card in cards {
        if card.enhancement != Some(Enhancement::Stone) {
            *counts.entry(card.rank).or_insert(0) += 1;
        }
    }
    counts
}

fn is_flush(cards: &[Card], four_fingers: bool, smeared: bool) -> bool {
    find_flush_indices(cards, four_fingers, smeared).is_some()
}

fn is_straight(cards: &[Card], four_fingers: bool, shortcut: bool) -> bool {
    find_straight_indices(cards, four_fingers, shortcut).is_some()
}

fn is_flush_five(cards: &[Card], smeared: bool) -> bool {
    cards.len() == 5
        && flush_candidate(cards, &[0, 1, 2, 3, 4], smeared)
        && count_ranks(cards).values().any(|&count| count == 5)
}

fn is_flush_house(cards: &[Card], smeared: bool) -> bool {
    if cards.len() != 5 || !flush_candidate(cards, &[0, 1, 2, 3, 4], smeared) {
        return false;
    }
    let counts = count_ranks(cards);
    let values: Vec<&usize> = counts.values().collect();
    values.contains(&&3) && values.contains(&&2)
}

fn is_five_of_a_kind(cards: &[Card]) -> bool {
    cards.len() == 5 && count_ranks(cards).values().any(|&count| count == 5)
}

fn is_straight_flush(cards: &[Card], four_fingers: bool, shortcut: bool, smeared: bool) -> bool {
    if four_fingers {
        find_straight_indices(cards, true, shortcut).is_some()
            && find_flush_indices(cards, true, smeared).is_some()
    } else {
        combinations(cards, 5).into_iter().any(|indices| {
            straight_candidate(cards, &indices, shortcut)
                && flush_candidate(cards, &indices, smeared)
        })
    }
}

fn combinations(cards: &[Card], size: usize) -> Vec<Vec<usize>> {
    fn build(
        candidates: &[usize],
        size: usize,
        start: usize,
        current: &mut Vec<usize>,
        result: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == size {
            result.push(current.clone());
            return;
        }
        for position in start..candidates.len() {
            current.push(candidates[position]);
            build(candidates, size, position + 1, current, result);
            current.pop();
        }
    }

    let candidates: Vec<usize> = cards
        .iter()
        .enumerate()
        .filter_map(|(index, card)| (!card_is_stone(card)).then_some(index))
        .collect();
    let mut result = Vec::new();
    build(&candidates, size, 0, &mut Vec::new(), &mut result);
    result
}

fn straight_candidate(cards: &[Card], indices: &[usize], shortcut: bool) -> bool {
    let mut ranks: Vec<u8> = indices
        .iter()
        .map(|&index| rank_order(cards[index].rank))
        .collect();
    ranks.sort_unstable();
    ranks.dedup();
    if ranks.len() != indices.len() {
        return false;
    }

    fn valid(ranks: &[u8], shortcut: bool) -> bool {
        ranks.windows(2).all(|pair| {
            let difference = pair[1] - pair[0];
            difference == 1 || (shortcut && difference == 2)
        })
    }

    if valid(&ranks, shortcut) {
        return true;
    }
    if let Some(ace) = ranks.iter().position(|&rank| rank == 14) {
        ranks[ace] = 1;
        ranks.sort_unstable();
        return valid(&ranks, shortcut);
    }
    false
}

fn flush_candidate(cards: &[Card], indices: &[usize], smeared: bool) -> bool {
    const SUITS: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Clubs, Suit::Diamonds];
    SUITS.iter().any(|&suit| {
        indices
            .iter()
            .all(|&index| card_matches_suit(cards[index], suit, smeared))
    })
}

fn find_straight_indices(cards: &[Card], four_fingers: bool, shortcut: bool) -> Option<Vec<usize>> {
    let minimum = if four_fingers { 4 } else { 5 };
    (minimum..=5).rev().find_map(|size| {
        combinations(cards, size)
            .into_iter()
            .find(|indices| straight_candidate(cards, indices, shortcut))
    })
}

fn find_flush_indices(cards: &[Card], four_fingers: bool, smeared: bool) -> Option<Vec<usize>> {
    let minimum = if four_fingers { 4 } else { 5 };
    (minimum..=5).rev().find_map(|size| {
        combinations(cards, size)
            .into_iter()
            .find(|indices| flush_candidate(cards, indices, smeared))
    })
}

fn is_four_of_a_kind(cards: &[Card]) -> bool {
    count_ranks(cards).values().any(|&count| count >= 4)
}

fn is_full_house(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    let values: Vec<&usize> = counts.values().collect();
    values.contains(&&3) && values.contains(&&2)
}

fn is_three_of_a_kind(cards: &[Card]) -> bool {
    count_ranks(cards).values().any(|&count| count >= 3)
}

fn is_two_pair(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    let pair_count = counts.values().filter(|&&count| count >= 2).count();
    pair_count >= 2
}

fn is_pair(cards: &[Card]) -> bool {
    count_ranks(cards).values().any(|&count| count >= 2)
}

fn card_is_stone(card: &Card) -> bool {
    card.enhancement == Some(Enhancement::Stone)
}

// ==================== 提取计分卡牌函数 ====================

fn get_scored_indices(cards: &[Card], hand: PokerHand, jokers: &[JokerCard]) -> Vec<usize> {
    let counts = count_ranks(cards);
    let four_fingers = jokers.iter().any(|joker| joker.joker == Joker::FourFingers);
    let shortcut = jokers.iter().any(|joker| joker.joker == Joker::Shortcut);
    let smeared = jokers
        .iter()
        .any(|joker| joker.joker == Joker::SmearedJoker);

    match hand {
        PokerHand::FlushFive | PokerHand::FlushHouse | PokerHand::FullHouse => cards
            .iter()
            .enumerate()
            .filter_map(|(index, card)| (!card_is_stone(card)).then_some(index))
            .collect(),
        PokerHand::StraightFlush => {
            if four_fingers {
                let mut indices = find_straight_indices(cards, true, shortcut).unwrap_or_default();
                indices.extend(find_flush_indices(cards, true, smeared).unwrap_or_default());
                indices.sort_unstable();
                indices.dedup();
                indices
            } else {
                combinations(cards, 5)
                    .into_iter()
                    .find(|indices| {
                        straight_candidate(cards, indices, shortcut)
                            && flush_candidate(cards, indices, smeared)
                    })
                    .unwrap_or_default()
            }
        }
        PokerHand::Flush => find_flush_indices(cards, four_fingers, smeared).unwrap_or_default(),
        PokerHand::Straight => {
            find_straight_indices(cards, four_fingers, shortcut).unwrap_or_default()
        }
        PokerHand::FiveOfAKind => rank_group_indices(cards, &counts, 5),
        PokerHand::FourOfAKind => rank_group_indices(cards, &counts, 4),
        PokerHand::ThreeOfAKind => rank_group_indices(cards, &counts, 3),
        PokerHand::Pair => rank_group_indices(cards, &counts, 2),
        PokerHand::TwoPair => {
            let mut target_ranks: Vec<Rank> = counts
                .iter()
                .filter(|&(_, count)| *count >= 2)
                .map(|(&rank, _)| rank)
                .collect();
            target_ranks.sort_unstable_by_key(|rank| rank_order(*rank));
            target_ranks.reverse();
            target_ranks.truncate(2);
            cards
                .iter()
                .enumerate()
                .filter_map(|(index, card)| {
                    (!card_is_stone(card) && target_ranks.contains(&card.rank)).then_some(index)
                })
                .collect()
        }
        PokerHand::HighCard => cards
            .iter()
            .enumerate()
            .filter(|(_, card)| !card_is_stone(card))
            .max_by_key(|(_, card)| rank_order(card.rank))
            .map(|(index, _)| vec![index])
            .unwrap_or_default(),
    }
}

fn rank_group_indices(cards: &[Card], counts: &HashMap<Rank, usize>, minimum: usize) -> Vec<usize> {
    let Some((&target_rank, _)) = counts.iter().find(|(_, count)| **count >= minimum) else {
        return Vec::new();
    };
    cards
        .iter()
        .enumerate()
        .filter_map(|(index, card)| {
            (!card_is_stone(card) && card.rank == target_rank).then_some(index)
        })
        .collect()
}
