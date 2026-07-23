//! Small native match harness used for regression matches and weight tuning.

use crate::{GamePosition, SearchEngine, MATE_SCORE};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArenaResult {
    FirstWins,
    SecondWins,
    Draw,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MatchSummary {
    pub first_wins: u32,
    pub second_wins: u32,
    pub draws: u32,
    pub total_turns: u64,
}

impl MatchSummary {
    pub fn games(&self) -> u32 {
        self.first_wins + self.second_wins + self.draws
    }

    pub fn first_score_percent(&self) -> f64 {
        if self.games() == 0 {
            return 0.0;
        }
        100.0 * (self.first_wins as f64 + self.draws as f64 * 0.5) / self.games() as f64
    }
}

/// Play a deterministic suite of initial states, swapping engines for every
/// second game to reduce first-player bias.
///
/// A game that reaches `max_turns` or returns no move in a nonterminal state is
/// adjudicated by static evaluation. In production, pass a limit comfortably
/// beyond ordinary game length so adjudication only catches repetitions.
pub fn play_match<P, I>(
    initial_states: I,
    first: &mut SearchEngine<P>,
    second: &mut SearchEngine<P>,
    max_turns: usize,
) -> MatchSummary
where
    P: GamePosition,
    I: IntoIterator<Item = P>,
{
    let mut summary = MatchSummary::default();
    for (game_index, initial) in initial_states.into_iter().enumerate() {
        let swapped = game_index % 2 == 1;
        let (result, turns) = play_one(initial, first, second, swapped, max_turns);
        let result = if swapped {
            match result {
                ArenaResult::FirstWins => ArenaResult::SecondWins,
                ArenaResult::SecondWins => ArenaResult::FirstWins,
                ArenaResult::Draw => ArenaResult::Draw,
            }
        } else {
            result
        };
        match result {
            ArenaResult::FirstWins => summary.first_wins += 1,
            ArenaResult::SecondWins => summary.second_wins += 1,
            ArenaResult::Draw => summary.draws += 1,
        }
        summary.total_turns += turns as u64;
    }
    summary
}

fn play_one<P: GamePosition>(
    mut position: P,
    first: &mut SearchEngine<P>,
    second: &mut SearchEngine<P>,
    swapped: bool,
    max_turns: usize,
) -> (ArenaResult, usize) {
    for turn in 0..max_turns {
        if let Some(score) = position.terminal_score() {
            return (result_from_side_to_move(score, turn), turn);
        }
        let use_first = (turn % 2 == 0) ^ swapped;
        let result = if use_first {
            first.search(&position)
        } else {
            second.search(&position)
        };
        let Some(mv) = result.best_move else {
            return (result_from_side_to_move(-MATE_SCORE, turn), turn);
        };
        position = position.apply_move(mv);
    }
    (
        result_from_side_to_move(position.evaluate(), max_turns),
        max_turns,
    )
}

fn result_from_side_to_move(score: i32, turn: usize) -> ArenaResult {
    if score == 0 {
        ArenaResult::Draw
    } else {
        let side_to_move_wins = score > 0;
        let first_to_move = turn & 1 == 0;
        if side_to_move_wins == first_to_move {
            ArenaResult::FirstWins
        } else {
            ArenaResult::SecondWins
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SearchConfig;
    use std::time::Duration;

    #[derive(Clone)]
    struct OneMove(bool);

    impl GamePosition for OneMove {
        type Move = u8;

        fn legal_moves(&self, moves: &mut Vec<Self::Move>) {
            if !self.0 {
                moves.push(0);
            }
        }
        fn apply_move(&self, _: Self::Move) -> Self {
            Self(true)
        }
        fn position_hash(&self) -> u64 {
            self.0 as u64
        }
        fn terminal_score(&self) -> Option<i32> {
            self.0.then_some(-MATE_SCORE)
        }
        fn evaluate(&self) -> i32 {
            0
        }
    }

    #[test]
    fn swapping_seats_attributes_wins_to_the_correct_engine() {
        let config = SearchConfig {
            time_limit: Duration::from_millis(10),
            max_depth: 2,
            ..SearchConfig::default()
        };
        let mut first = SearchEngine::new(config.clone());
        let mut second = SearchEngine::new(config);
        let summary = play_match([OneMove(false), OneMove(false)], &mut first, &mut second, 4);
        assert_eq!(summary.games(), 2);
        assert_eq!(summary.first_wins, 1);
        assert_eq!(summary.second_wins, 1);
    }
}
