//stage3
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

// ==================== Stage 3 核心计分逻辑 ====================

fn score(round: Round) -> (Chips, Mult) {
    let played_cards = round.cards_played;

    // 1. 判定最优牌型并获取其基础 Chips 和 Mult
    let best_hand = determine_poker_hand(&played_cards);
    let (mut total_chips, mut total_mult) = best_hand.hand_value();

    // 2. 找出在这个牌型中被成功"计分"的牌
    let mut scored_cards = get_scored_cards(&played_cards, best_hand);
    
    // 【阶段二新增】石头牌（Stone）不属于任何牌型，但只要被打出就必须强制计分
    for card in &played_cards {
        if card.enhancement == Some(Enhancement::Stone) && !scored_cards.contains(card) {
            scored_cards.push(*card);
        }
    }

    // 3. 按照顺序应用打出牌的各种强化和版本（步骤 2.2、2.3）
    for card in scored_cards {
        // 步骤 2.1：基础筹码 (石头牌固定给 +50 Chips，不按点数算)
        if card.enhancement == Some(Enhancement::Stone) {
            total_chips += 50.0;
        } else {
            total_chips += card.rank.rank_value();
        }

        // 步骤 2.2：卡牌增强（Enhancements）调整
        if let Some(enhancement) = card.enhancement {
            match enhancement {
                Enhancement::Bonus => total_chips += 30.0,
                Enhancement::Mult => total_mult += 4.0,
                Enhancement::Glass => total_mult *= 2.0,
                _ => {} // Wild 在判定阶段生效；Steel 在手牌中生效
            }
        }

        // 步骤 2.3：卡牌版本（Editions）调整
        if let Some(edition) = card.edition {
            match edition {
                Edition::Foil => total_chips += 50.0,
                Edition::Holographic => total_mult += 10.0,
                Edition::Polychrome => total_mult *= 1.5,
            }
        }
    }

    // 4. 【阶段二新增】步骤 3.1：遍历留在手里的牌（Held in Hand）计算手牌钢铁卡能力
    for card in round.cards_held_in_hand {
        if card.enhancement == Some(Enhancement::Steel) {
            total_mult *= 1.5;
        }
    }

    // 【阶段三新增】步骤 4：处理 Joker 卡牌效果
    // 这里实现的是 "Independent" 类型的 Joker，它们在所有卡牌计分后激活
    for joker in &round.jokers {
        apply_independent_joker(joker, &played_cards, best_hand, &mut total_chips, &mut total_mult);
    }

    (total_chips, total_mult)
}

/// 【阶段三新增】应用 Independent 类型的 Joker 效果
fn apply_independent_joker(
    joker: &JokerCard,
    played_cards: &[Card],
    best_hand: PokerHand,
    total_chips: &mut Chips,
    total_mult: &mut Mult,
) {
    match joker.joker {
        // 1. Joker: +4 Mult 无条件
        Joker::Joker => {
            *total_mult += 4.0;
        }

        // 2. Jolly Joker: +8 Mult if Pair 存在
        Joker::JollyJoker => {
            if contains_pair(played_cards) {
                *total_mult += 8.0;
            }
        }

        // 3. Zany Joker: +12 Mult if Three of a Kind 存在
        Joker::ZanyJoker => {
            if contains_three_of_a_kind(played_cards) {
                *total_mult += 12.0;
            }
        }

        // 4. Mad Joker: +10 Mult if Two Pair 存在
        Joker::MadJoker => {
            if contains_two_pair(played_cards) {
                *total_mult += 10.0;
            }
        }

        // 5. The Order: ×3 Mult if Straight 存在
        Joker::TheOrder => {
            if contains_straight(played_cards) {
                *total_mult *= 3.0;
            }
        }

        // 6. The Tribe: ×2 Mult if Flush 存在
        Joker::TheTribe => {
            if contains_flush(played_cards) {
                *total_mult *= 2.0;
            }
        }

        // 7. Sly Joker: +50 Chips if Pair 存在
        Joker::SlyJoker => {
            if contains_pair(played_cards) {
                *total_chips += 50.0;
            }
        }

        // 8. Wily Joker: +100 Chips if Three of a Kind 存在
        Joker::WilyJoker => {
            if contains_three_of_a_kind(played_cards) {
                *total_chips += 100.0;
            }
        }

        // 9. Clever Joker: +80 Chips if Two Pair 存在
        Joker::CleverJoker => {
            if contains_two_pair(played_cards) {
                *total_chips += 80.0;
            }
        }

        // 10. Devious Joker: +100 Chips if Straight 存在
        Joker::DeviousJoker => {
            if contains_straight(played_cards) {
                *total_chips += 100.0;
            }
        }

        // 11. Crafty Joker: +80 Chips if Flush 存在
        Joker::CraftyJoker => {
            if contains_flush(played_cards) {
                *total_chips += 80.0;
            }
        }

        // 12. Abstract Joker: +3 Mult for each Joker card
        Joker::AbstractJoker => {
            // 这个会在后面处理（需要知道 Joker 总数）
        }

        // 其他 Joker 暂不处理（Stage 4+）
        _ => {}
    }
}

/// 【阶段三新增】检查是否存在 Pair（任意两张相同点数的牌）
fn contains_pair(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    counts.values().any(|&count| count >= 2)
}

/// 【阶段三新增】检查是否存在 Three of a Kind
fn contains_three_of_a_kind(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    counts.values().any(|&count| count >= 3)
}

/// 【阶段三新增】检查是否存在 Two Pair（两个不同的 Pair）
fn contains_two_pair(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    let pair_count = counts.values().filter(|&&count| count >= 2).count();
    pair_count >= 2
}

/// 【阶段三新增】检查是否存在 Straight
fn contains_straight(cards: &[Card]) -> bool {
    is_sequential(cards)
}

/// 【阶段三新增】检查是否存在 Flush
fn contains_flush(cards: &[Card]) -> bool {
    is_all_same_suit(cards)
}

/// 从高到低判定当前打出的牌属于哪种最佳牌型
fn determine_poker_hand(cards: &[Card]) -> PokerHand {
    if is_flush_five(cards) { return PokerHand::FlushFive; }
    if is_flush_house(cards) { return PokerHand::FlushHouse; }
    if is_five_of_a_kind(cards) { return PokerHand::FiveOfAKind; }
    if is_straight_flush(cards) { return PokerHand::StraightFlush; }
    if is_four_of_a_kind(cards) { return PokerHand::FourOfAKind; }
    if is_full_house(cards) { return PokerHand::FullHouse; }
    if is_flush(cards) { return PokerHand::Flush; }
    if is_straight(cards) { return PokerHand::Straight; }
    if is_three_of_a_kind(cards) { return PokerHand::ThreeOfAKind; }
    if is_two_pair(cards) { return PokerHand::TwoPair; }
    if is_pair(cards) { return PokerHand::Pair; }
    
    PokerHand::HighCard
}

// ==================== 牌型辅助判定函数 (升级版) ====================

// 统计各点数出现的次数（注意：石头牌没有点数，需剔除）
fn count_ranks(cards: &[Card]) -> std::collections::HashMap<Rank, usize> {
    let mut counts = std::collections::HashMap::new();
    for card in cards {
        if card.enhancement != Some(Enhancement::Stone) {
            *counts.entry(card.rank).or_insert(0) += 1;
        }
    }
    counts
}

// 【关键改动】升级同花花色判定：兼容万能牌 Wild，剔除石头牌 Stone
fn is_all_same_suit(cards: &[Card]) -> bool {
    if cards.len() < 5 { return false; }
    
    // 如果包含石头牌，则无法凑成同花
    if cards.iter().any(|c| card_is_stone(c)) {
        return false;
    }

    // 收集所有非万能牌的花色
    let non_wild_suits: Vec<Suit> = cards.iter()
        .filter(|c| c.enhancement != Some(Enhancement::Wild))
        .map(|c| c.suit)
        .collect();

    // 如果全部都是万能牌，自然满足同花
    if non_wild_suits.is_empty() {
        return true;
    }

    // 检查所有固定花色是否完全相同
    let first_suit = non_wild_suits[0];
    non_wild_suits.iter().all(|&s| s == first_suit)
}

fn is_sequential(cards: &[Card]) -> bool {
    if cards.len() < 5 { return false; }
    
    // 石头牌不参与顺子判定
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

    if rank_nums[4] - rank_nums[0] == 4 {
        return true;
    }

    if rank_nums == vec![2, 3, 4, 5, 14] {
        return true;
    }

    false
}

fn is_flush_five(cards: &[Card]) -> bool {
    cards.len() == 5 && is_all_same_suit(cards) && count_ranks(cards).values().any(|&count| count == 5)
}

fn is_flush_house(cards: &[Card]) -> bool {
    if cards.len() != 5 || !is_all_same_suit(cards) { return false; }
    let counts = count_ranks(cards);
    let values: Vec<&usize> = counts.values().collect();
    values.contains(&&3) && values.contains(&&2)
}

fn is_five_of_a_kind(cards: &[Card]) -> bool {
    cards.len() == 5 && count_ranks(cards).values().any(|&count| count == 5)
}

fn is_straight_flush(cards: &[Card]) -> bool {
    is_sequential(cards) && is_all_same_suit(cards)
}

fn is_four_of_a_kind(cards: &[Card]) -> bool {
    count_ranks(cards).values().any(|&count| count >= 4)
}

fn is_full_house(cards: &[Card]) -> bool {
    let counts = count_ranks(cards);
    let values: Vec<&usize> = counts.values().collect();
    values.contains(&&3) && values.contains(&&2)
}

fn is_flush(cards: &[Card]) -> bool {
    is_all_same_suit(cards)
}

fn is_straight(cards: &[Card]) -> bool {
    is_sequential(cards)
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
    // 计分筛选时同样要去除石头牌再进行组合点数切分
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
