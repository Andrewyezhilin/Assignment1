//stage5
use std::{
    error::Error,
    fs::File,
    io::{Read, stdin},
    path::{Path, PathBuf},
};

use clap::Parser;
use ortalib::{Chips, Mult, Round, Card, PokerHand, Rank, Suit, Enhancement, Edition, Joker, JokerCard};

#[derive(Parser)]
struct Opts {
    file: PathBuf,

    #[arg(long)]
    explain: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::parse();
    let round = parse_round(&opts)?;

    let (chips, mult) = score(round);

    println!("{}", (chips * mult).floor());
    Ok(())
}

fn parse_round(opts: &Opts) -> Result<Round, Box<dyn Error>> {
    let mut input = String::new();
    if opts.file == Path::new("-") {
        stdin().read_to_string(&mut input)?;
    } else {
        File::open(&opts.file)?.read_to_string(&mut input)?;
    }

    let round = serde_yaml::from_str(&input)?;
    Ok(round)
}

// ==================== Stage 5 核心计分逻辑 ====================

fn score(round: Round) -> (Chips, Mult) {
    let played_cards = round.cards_played.clone();

    // 1. 判定最优牌型并获取其基础 Chips 和 Mult
    let best_hand = determine_poker_hand(&played_cards, &round.jokers);
    let (mut total_chips, mut total_mult) = best_hand.hand_value();

    // 2. 找出在这个牌型中被成功"计分"的牌
    let mut scored_cards = get_scored_cards(&played_cards, best_hand);
    
    // 石头牌不属于任何牌型，但只要被打出就必须强制计分
    for card in &played_cards {
        if card.enhancement == Some(Enhancement::Stone) && !scored_cards.contains(card) {
            scored_cards.push(*card);
        }
    }

    // 【Stage 5】检查 Splash 是否激活（所有卡牌都计分）
    let splash_active = round.jokers.iter().any(|j| j.joker == Joker::Splash);
    if splash_active {
        for card in &played_cards {
            if !scored_cards.contains(card) && card.enhancement != Some(Enhancement::Stone) {
                scored_cards.push(*card);
            }
        }
    }

    // 追踪是否已经应用了 Photograph 的第一张面卡
    let mut photograph_triggered = false;

    // 3. 按照顺序应用打出牌的各种强化和版本（步骤 2.1-2.3）和 On Scored Jokers
    for (index, card) in scored_cards.iter().enumerate() {
        apply_card_scoring(&round, *card, &mut total_chips, &mut total_mult, &mut photograph_triggered, index == 0);
    }

    // 步骤 3.1：遍历留在手里的牌计算手牌钢铁卡能力和 On Held Jokers
    for held_card in &round.cards_held_in_hand {
        if held_card.enhancement == Some(Enhancement::Steel) {
            total_mult *= 1.5;
        }

        // On Held Jokers 激活
        for joker in &round.jokers {
            apply_on_held_joker(joker, *held_card, &round.cards_held_in_hand, &mut total_chips, &mut total_mult);
        }
    }

    // 【Stage 5】检查 Mime 是否激活（重新触发所有手牌能力）
    let mime_count = round.jokers.iter().filter(|j| j.joker == Joker::Mime).count();
    for _ in 0..mime_count {
        for held_card in &round.cards_held_in_hand {
            if held_card.enhancement == Some(Enhancement::Steel) {
                total_mult *= 1.5;
            }

            for joker in &round.jokers {
                apply_on_held_joker(joker, *held_card, &round.cards_held_in_hand, &mut total_chips, &mut total_mult);
            }
        }
    }

    // 步骤 4.1：处理 Joker 的版本效果（Foil, Holographic）
    for joker in &round.jokers {
        if let Some(edition) = joker.edition {
            match edition {
                Edition::Foil => total_chips += 50.0,
                Edition::Holographic => total_mult += 10.0,
                Edition::Polychrome => {}
            }
        }
    }

    // 步骤 4.2：处理 Independent 类型的 Joker 效果
    for joker in &round.jokers {
        apply_independent_joker(joker, &played_cards, &round.cards_held_in_hand, &mut total_chips, &mut total_mult, round.jokers.len());
    }

    // 步骤 4.3：处理 Joker 的 Polychrome 版本效果
    for joker in &round.jokers {
        if let Some(edition) = joker.edition {
            if edition == Edition::Polychrome {
                total_mult *= 1.5;
            }
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
    photograph_triggered: &mut bool,
    is_first_card: bool,
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

    // 步骤 2.4：On Scored Jokers 激活
    apply_on_scored_jokers(round, card, total_chips, total_mult, photograph_triggered, is_first_card);
}

/// 应用所有 On Scored Jokers
fn apply_on_scored_jokers(
    round: &Round,
    card: Card,
    total_chips: &mut Chips,
    total_mult: &mut Mult,
    photograph_triggered: &mut bool,
    is_first_card: bool,
) {
    for joker in &round.jokers {
        apply_on_scored_joker(joker, card, total_chips, total_mult, photograph_triggered, round.jokers.len(), &round.jokers);
    }

    // 【Stage 5】处理 Retriggers
    let mut retrigger_count = 0;

    // Sock and Buskin: Retrigger all scoring face cards
    let sock_buskin_count = round.jokers.iter().filter(|j| j.joker == Joker::SockAndBuskin).count();
    let is_face_card = is_card_face(card, round);
    if is_face_card {
        retrigger_count += sock_buskin_count;
    }

    // Hack: Retrigger each scored card that is a 2, 3, 4, or 5
    let hack_count = round.jokers.iter().filter(|j| j.joker == Joker::Hack).count();
    match card.rank {
        Rank::Two | Rank::Three | Rank::Four | Rank::Five => {
            retrigger_count += hack_count;
        }
        _ => {}
    }

    // Hanging Chad: Retrigger the first scored card 2 additional times
    if is_first_card && round.jokers.iter().any(|j| j.joker == Joker::HangingChad) {
        retrigger_count += 2;
    }

    // 执行 Retriggers（重复应用 On Scored Jokers）
    for _ in 0..retrigger_count {
        for joker in &round.jokers {
            apply_on_scored_joker(joker, card, total_chips, total_mult, photograph_triggered, round.jokers.len(), &round.jokers);
        }
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
    joker: &JokerCard,
    card: Card,
    total_chips: &mut Chips,
    total_mult: &mut Mult,
    photograph_triggered: &mut bool,
    _joker_count: usize,
    all_jokers: &[JokerCard],
) {
    match joker.joker {
        Joker::GreedyJoker => {
            if card_matches_suit(card, Suit::Diamonds) {
                *total_mult += 3.0;
            }
        }

        Joker::LustyJoker => {
            if card_matches_suit(card, Suit::Hearts) {
                *total_mult += 3.0;
            }
        }

        Joker::Arrowhead => {
            if card_matches_suit(card, Suit::Spades) {
                *total_chips += 50.0;
            }
        }

        Joker::OnyxAgate => {
            if card_matches_suit(card, Suit::Clubs) {
                *total_mult += 7.0;
            }
        }

        Joker::Fibonacci => {
            match card.rank {
                Rank::Ace | Rank::Two | Rank::Three | Rank::Five | Rank::Eight => {
                    *total_mult += 8.0;
                }
                _ => {}
            }
        }

        Joker::ScaryFace => {
            if card.rank.is_face() {
                *total_chips += 30.0;
            }
        }

        Joker::EvenSteven => {
            match card.rank {
                Rank::Two | Rank::Four | Rank::Six | Rank::Eight | Rank::Ten => {
                    *total_mult += 4.0;
                }
                _ => {}
            }
        }

        Joker::OddTodd => {
            match card.rank {
                Rank::Ace | Rank::Three | Rank::Five | Rank::Seven | Rank::Nine => {
                    *total_chips += 31.0;
                }
                _ => {}
            }
        }

        Joker::Scholar => {
            if card.rank == Rank::Ace {
                *total_chips += 20.0;
                *total_mult += 4.0;
            }
        }

        Joker::WalkieTalkie => {
            match card.rank {
                Rank::Ten | Rank::Four => {
                    *total_chips += 10.0;
                    *total_mult += 4.0;
                }
                _ => {}
            }
        }

        Joker::Photograph => {
            if card.rank.is_face() && !*photograph_triggered {
                *photograph_triggered = true;
                *total_mult *= 2.0;
            }
        }

        Joker::SmileyFace => {
            if card.rank.is_face() {
                *total_mult += 5.0;
            }
        }

        Joker::Blueprint => {
            // Blueprint 复制右边（后面）的 Joker
            // 这里简单处理：找到 Blueprint 的索引，然后复制下一个 Joker
            if let Some(pos) = all_jokers.iter().position(|j| std::ptr::eq(j, joker)) {
                if pos + 1 < all_jokers.len() {
                    let next_joker = &all_jokers[pos + 1];
                    // 递归调用以复制下一个 Joker 的效果
                    apply_on_scored_joker(next_joker, card, total_chips, total_mult, photograph_triggered, _joker_count, all_jokers);
                }
            }
        }

        _ => {}
    }
}

/// 应用 On Held 类型的 Joker 效果
fn apply_on_held_joker(
    joker: &JokerCard,
    held_card: Card,
    held_cards: &[Card],
    total_chips: &mut Chips,
    total_mult: &mut Mult,
) {
    match joker.joker {
        Joker::RaisedFist => {
            if let Some(lowest_card) = get_lowest_rank_card(held_cards) {
                if held_card.rank == lowest_card.rank {
                    let lowest_cards: Vec<Card> = held_cards.iter()
                        .filter(|c| c.rank == lowest_card.rank)
                        .copied()
                        .collect();
                    if held_card == *lowest_cards.last().unwrap() {
                        *total_mult += 2.0 * held_card.rank.rank_value();
                    }
                }
            }
        }

        Joker::Baron => {
            if held_card.rank == Rank::King {
                *total_mult *= 1.5;
            }
        }

        Joker::ShootTheMoon => {
            if held_card.rank == Rank::Queen {
                *total_mult += 13.0;
            }
        }

        _ => {}
    }
}

/// 应用 Independent 类型的 Joker 效果
fn apply_independent_joker(
    joker: &JokerCard,
    played_cards: &[Card],
    held_cards: &[Card],
    total_chips: &mut Chips,
    total_mult: &mut Mult,
    joker_count: usize,
) {
    match joker.joker {
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
            if contains_straight(played_cards) {
                *total_mult *= 3.0;
            }
        }

        Joker::TheTribe => {
            if contains_flush(played_cards) {
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
            if contains_straight(played_cards) {
                *total_chips += 100.0;
            }
        }

        Joker::CraftyJoker => {
            if contains_flush(played_cards) {
                *total_chips += 80.0;
            }
        }

        Joker::AbstractJoker => {
            *total_mult += 3.0 * joker_count as f64;
        }

        Joker::Blackboard => {
            if held_cards.is_empty() {
                *total_mult *= 3.0;
            } else if held_cards.iter().all(|c| {
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
            let has_diamonds = played_cards.iter().any(|c| card_matches_suit(*c, Suit::Diamonds));
            let has_clubs = played_cards.iter().any(|c| card_matches_suit(*c, Suit::Clubs));
            let has_hearts = played_cards.iter().any(|c| card_matches_suit(*c, Suit::Hearts));
            let has_spades = played_cards.iter().any(|c| card_matches_suit(*c, Suit::Spades));
            
            if has_diamonds && has_clubs && has_hearts && has_spades {
                *total_mult *= 3.0;
            }
        }

        Joker::FourFingers | Joker::Shortcut | Joker::Mime | Joker::Pareidolia | 
        Joker::Splash | Joker::SockAndBuskin | Joker::Hack | Joker::HangingChad | 
        Joker::SmearedJoker | Joker::Blueprint => {
            // 这些是效果类 Joker，不在这里给出奖励
        }

        _ => {}
    }
}

/// 检查卡是否匹配特定花色（考虑 Wild）
fn card_matches_suit(card: Card, suit: Suit) -> bool {
    if card.enhancement == Some(Enhancement::Wild) {
        true
    } else {
        card.suit == suit
    }
}

/// 获取手中最低点数的卡
fn get_lowest_rank_card(cards: &[Card]) -> Option<Card> {
    cards.iter()
        .min_by_key(|c| match c.rank {
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
        })
        .copied()
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

/// 检查是否存在 Straight
fn contains_straight(cards: &[Card]) -> bool {
    is_sequential(cards, false, false)
}

/// 检查是否存在 Flush
fn contains_flush(cards: &[Card]) -> bool {
    is_all_same_suit(cards, false)
}

/// 从高到低判定当前打出的牌属于哪种最佳牌型
fn determine_poker_hand(cards: &[Card], jokers: &[JokerCard]) -> PokerHand {
    let four_fingers = jokers.iter().any(|j| j.joker == Joker::FourFingers);
    let shortcut = jokers.iter().any(|j| j.joker == Joker::Shortcut);
    let smeared = jokers.iter().any(|j| j.joker == Joker::SmearedJoker);

    if is_flush_five(cards) { return PokerHand::FlushFive; }
    if is_flush_house(cards) { return PokerHand::FlushHouse; }
    if is_five_of_a_kind(cards) { return PokerHand::FiveOfAKind; }
    if is_straight_flush(cards, four_fingers, shortcut, smeared) { return PokerHand::StraightFlush; }
    if is_four_of_a_kind(cards) { return PokerHand::FourOfAKind; }
    if is_full_house(cards) { return PokerHand::FullHouse; }
    if is_flush(cards, four_fingers, smeared) { return PokerHand::Flush; }
    if is_straight(cards, shortcut) { return PokerHand::Straight; }
    if is_three_of_a_kind(cards) { return PokerHand::ThreeOfAKind; }
    if is_two_pair(cards) { return PokerHand::TwoPair; }
    if is_pair(cards) { return PokerHand::Pair; }
    
    PokerHand::HighCard
}

// ==================== 牌型辅助判定函数 ====================

fn count_ranks(cards: &[Card]) -> std::collections::HashMap<Rank, usize> {
    let mut counts = std::collections::HashMap::new();
    for card in cards {
        if card.enhancement != Some(Enhancement::Stone) {
            *counts.entry(card.rank).or_insert(0) += 1;
        }
    }
    counts
}

fn is_all_same_suit(cards: &[Card], smeared: bool) -> bool {
    if cards.len() < 5 { return false; }
    
    if cards.iter().any(|c| card_is_stone(c)) {
        return false;
    }

    let non_wild_suits: Vec<Suit> = cards.iter()
        .filter(|c| c.enhancement != Some(Enhancement::Wild))
        .map(|c| c.suit)
        .collect();

    if non_wild_suits.is_empty() {
        return true;
    }

    if smeared {
        // Smeared Joker 模式：红色和黑色分别作为同一花色
        let has_red = non_wild_suits.iter().any(|s| *s == Suit::Hearts || *s == Suit::Diamonds);
        let has_black = non_wild_suits.iter().any(|s| *s == Suit::Spades || *s == Suit::Clubs);
        let has_mixed = has_red && has_black;
        !has_mixed
    } else {
        let first_suit = non_wild_suits[0];
        non_wild_suits.iter().all(|&s| s == first_suit)
    }
}

fn is_sequential(cards: &[Card], shortcut: bool, _smeared: bool) -> bool {
    if cards.len() < 5 { return false; }
    
    if cards.iter().any(|c| card_is_stone(c)) {
        return false;
    }
    
    let mut rank_nums: Vec<u8> = cards.iter().map(|c| match c.rank {
        Rank::Two => 2, Rank::Three => 3, Rank::Four => 4, Rank::Five => 5,
        Rank::Six => 6, Rank::Seven => 7, Rank::Eight => 8, Rank::Nine => 9,
        Rank::Ten => 10, Rank::Jack => 11, Rank::Queen => 12, Rank::King => 13,
        Rank::Ace => 14,
    }).collect();
    
    rank_nums.sort_unstable();
    
    let mut unique_nums = rank_nums.clone();
    unique_nums.dedup();
    if unique_nums.len() < 5 { return false; }

    // 标准顺子：连续5个
    if rank_nums[4] - rank_nums[0] == 4 {
        return true;
    }

    // Ace 低顺
    if rank_nums == vec![2, 3, 4, 5, 14] {
        return true;
    }

    // Shortcut：允许有间隔
    if shortcut {
        if is_shortcut_straight(&unique_nums) {
            return true;
        }
    }

    false
}

fn is_shortcut_straight(unique_nums: &[u8]) -> bool {
    if unique_nums.len() < 5 { return false; }

    // 尝试找到5张卡的间隔顺子（每个间隔最多1）
    for i in 0..=unique_nums.len() - 5 {
        let subset = &unique_nums[i..i+5];
        if is_valid_shortcut(subset) {
            return true;
        }
    }

    false
}

fn is_valid_shortcut(five_ranks: &[u8]) -> bool {
    // 检查5张卡是否能形成间隔最多为1的顺子
    let mut gaps = 0;
    for i in 0..4 {
        let diff = five_ranks[i+1] - five_ranks[i];
        if diff == 1 {
            // 没有间隔
        } else if diff == 2 {
            // 有1个间隔
            gaps += 1;
        } else {
            // 间隔太大
            return false;
        }
    }
    gaps <= 4  // 允许最多4个间隔（每个位置一个）
}

fn is_flush(cards: &[Card], four_fingers: bool, smeared: bool) -> bool {
    let non_stone_cards: Vec<Card> = cards.iter()
        .filter(|c| c.enhancement != Some(Enhancement::Stone))
        .copied()
        .collect();
    
    let min_cards = if four_fingers { 4 } else { 5 };
    
    if non_stone_cards.len() < min_cards { return false; }

    if smeared {
        // Smeared 模式：红黑分开计数
        let red_cards = non_stone_cards.iter().filter(|c| {
            if c.enhancement == Some(Enhancement::Wild) { return true; }
            c.suit == Suit::Hearts || c.suit == Suit::Diamonds
        }).count();
        
        let black_cards = non_stone_cards.iter().filter(|c| {
            if c.enhancement == Some(Enhancement::Wild) { return true; }
            c.suit == Suit::Spades || c.suit == Suit::Clubs
        }).count();

        red_cards >= min_cards || black_cards >= min_cards
    } else {
        is_all_same_suit(&non_stone_cards, false) && non_stone_cards.len() >= min_cards
    }
}

fn is_straight(cards: &[Card], shortcut: bool) -> bool {
    if cards.len() < 5 { return false; }
    
    if cards.iter().any(|c| card_is_stone(c)) {
        return false;
    }
    
    is_sequential(cards, shortcut, false)
}

fn is_flush_five(cards: &[Card]) -> bool {
    cards.len() == 5 && is_all_same_suit(cards, false) && count_ranks(cards).values().any(|&count| count == 5)
}

fn is_flush_house(cards: &[Card]) -> bool {
    if cards.len() != 5 || !is_all_same_suit(cards, false) { return false; }
    let counts = count_ranks(cards);
    let values: Vec<&usize> = counts.values().collect();
    values.contains(&&3) && values.contains(&&2)
}

fn is_five_of_a_kind(cards: &[Card]) -> bool {
    cards.len() == 5 && count_ranks(cards).values().any(|&count| count == 5)
}

fn is_straight_flush(cards: &[Card], four_fingers: bool, shortcut: bool, smeared: bool) -> bool {
    let non_stone_cards: Vec<Card> = cards.iter()
        .filter(|c| c.enhancement != Some(Enhancement::Stone))
        .copied()
        .collect();
    
    let min_cards = if four_fingers { 4 } else { 5 };
    
    if non_stone_cards.len() < min_cards { return false; }

    // 检查是否是 Flush
    if !is_flush(cards, four_fingers, smeared) {
        return false;
    }

    // 检查是否是 Straight
    is_sequential(&non_stone_cards, shortcut, smeared)
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

fn get_scored_cards(cards: &[Card], hand: PokerHand) -> Vec<Card> {
    let cards_filtered: Vec<Card> = cards.iter().filter(|c| !card_is_stone(c)).copied().collect();
    let counts = count_ranks(&cards_filtered);
    
    match hand {
        PokerHand::FlushFive | PokerHand::FlushHouse | PokerHand::StraightFlush | 
        PokerHand::FullHouse | PokerHand::Flush | PokerHand::Straight => {
            cards_filtered
        }
        
        PokerHand::FiveOfAKind => {
            if counts.is_empty() { return vec![]; }
            let (target_rank, _) = counts.iter().find(|&(_, count)| *count == 5).unwrap();
            cards_filtered.iter().filter(|c| c.rank == *target_rank).copied().collect()
        }
        PokerHand::FourOfAKind => {
            if counts.is_empty() { return vec![]; }
            let (target_rank, _) = counts.iter().find(|&(_, count)| *count >= 4).unwrap();
            cards_filtered.iter().filter(|c| c.rank == *target_rank).copied().collect()
        }
        PokerHand::ThreeOfAKind => {
            if counts.is_empty() { return vec![]; }
            let (target_rank, _) = counts.iter().find(|&(_, count)| *count >= 3).unwrap();
            cards_filtered.iter().filter(|c| c.rank == *target_rank).copied().collect()
        }
        PokerHand::Pair => {
            if counts.is_empty() { return vec![]; }
            let (target_rank, _) = counts.iter().find(|&(_, count)| *count >= 2).unwrap();
            cards_filtered.iter().filter(|c| c.rank == *target_rank).copied().collect()
        }
        
        PokerHand::TwoPair => {
            let mut target_ranks: Vec<Rank> = counts.iter()
                .filter(|&(_, count)| *count >= 2)
                .map(|(&rank, _)| rank)
                .collect();
            
            target_ranks.sort_unstable_by_key(|r| match r {
                Rank::Two => 2, Rank::Three => 3, Rank::Four => 4, Rank::Five => 5,
                Rank::Six => 6, Rank::Seven => 7, Rank::Eight => 8, Rank::Nine => 9,
                Rank::Ten => 10, Rank::Jack => 11, Rank::Queen => 12, Rank::King => 13,
                Rank::Ace => 14,
            });
            target_ranks.reverse();
            target_ranks.truncate(2);

            cards_filtered.iter().filter(|c| target_ranks.contains(&c.rank)).copied().collect()
        }
        
        PokerHand::HighCard => {
            if cards_filtered.is_empty() { return vec![]; }
            let max_card = cards_filtered.iter().max_by_key(|c| match c.rank {
                Rank::Two => 2, Rank::Three => 3, Rank::Four => 4, Rank::Five => 5,
                Rank::Six => 6, Rank::Seven => 7, Rank::Eight => 8, Rank::Nine => 9,
                Rank::Ten => 10, Rank::Jack => 11, Rank::Queen => 12, Rank::King => 13,
                Rank::Ace => 14,
            }).unwrap();
            vec![*max_card]
        }
    }
}
