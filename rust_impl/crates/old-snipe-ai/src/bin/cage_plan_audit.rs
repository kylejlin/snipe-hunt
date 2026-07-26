//! Three-ply support-cage oracle for the late seed-2 fortress.

use snipe_ai::evaluate_state;
use snipe_core::{Animal, Location, Move, Player, Row, State, StateData};

fn main() {
    let beta_to_move = late_seed_two();
    let alpha_to_move = beta_to_move
        .apply_move(Move::Snipe {
            destination: Row::Four,
        })
        .unwrap();
    print_rows(alpha_to_move);

    let mut plans = alpha_to_move
        .legal_moves()
        .into_iter()
        .filter_map(|first| plan(alpha_to_move, first))
        .collect::<Vec<_>>();
    plans.sort_unstable_by_key(|plan| {
        (
            plan.worst_support,
            std::cmp::Reverse(plan.static_score),
            plan.first,
        )
    });
    println!(
        "ROOT hash={:016x} legal={} beta_support={} plans={}",
        alpha_to_move.position_hash(),
        alpha_to_move.legal_moves().len(),
        beta_support_region(alpha_to_move),
        plans.len()
    );
    for (rank, plan) in plans.iter().take(20).enumerate() {
        println!(
            "PLAN {} first={:?} beta={:?} second={:?} support={} score={}",
            rank + 1,
            plan.first,
            plan.safe_beta_reply,
            plan.second,
            plan.worst_support,
            plan.static_score
        );
    }
}

struct Plan {
    first: Move,
    safe_beta_reply: Move,
    second: Move,
    worst_support: i32,
    static_score: i32,
}

fn plan(state: State, first: Move) -> Option<Plan> {
    let beta_to_move = state.apply_move(first).ok()?;
    let safe_replies = safe_beta_replies(beta_to_move);
    let mut worst = None::<(i32, Move, Move, i32)>;
    for reply in safe_replies {
        let alpha_to_move = beta_to_move.apply_move(reply).unwrap();
        let best = alpha_to_move
            .legal_moves()
            .into_iter()
            .map(|second| {
                let child = alpha_to_move.apply_move(second).unwrap();
                let support = if child.winner() == Some(Player::Alpha) {
                    -1
                } else {
                    beta_support_region(child)
                };
                (support, second, -evaluate_state(child))
            })
            .min_by_key(|&(support, mv, score)| (support, std::cmp::Reverse(score), mv))?;
        if worst
            .as_ref()
            .is_none_or(|current| (best.0, reply) > (current.0, current.1))
        {
            worst = Some((best.0, reply, best.1, best.2));
        }
    }
    let (worst_support, safe_beta_reply, second, static_score) = worst?;
    Some(Plan {
        first,
        safe_beta_reply,
        second,
        worst_support,
        static_score,
    })
}

fn safe_beta_replies(state: State) -> Vec<Move> {
    state
        .legal_moves()
        .into_iter()
        .filter(|&reply| {
            let alpha_to_move = state.apply_move(reply).unwrap();
            alpha_to_move.winner() != Some(Player::Alpha)
                && !alpha_to_move.has_winning_snipe_capture()
        })
        .collect()
}

fn beta_support_region(state: State) -> i32 {
    let row = state
        .snipe_location(Player::Beta)
        .and_then(Location::row)
        .unwrap();
    [
        row.backward(Player::Beta),
        Some(row),
        row.forward(Player::Beta),
    ]
    .into_iter()
    .flatten()
    .map(|row| state.cell(row.location()).all_animals().count_ones() as i32)
    .sum()
}

fn print_rows(state: State) {
    for row in [Row::Three, Row::Four, Row::Five] {
        let cell = state.cell(row.location());
        let animals = Animal::ALL
            .into_iter()
            .filter(|animal| cell.all_animals() & animal.bit() != 0)
            .map(|animal| (animal, state.owner_of_animal(animal).unwrap()))
            .collect::<Vec<_>>();
        println!(
            "ROW {} animals={animals:?} snipes={:02b}",
            row.number(),
            cell.snipes()
        );
    }
}

fn late_seed_two() -> State {
    State::from_data(StateData {
        alpha_animals: [402_047_060, 536_877_344, 1_074_339_849, 128, 0, 0, 0, 0],
        beta_animals: [0, 2, 0, 0, 1_024, 512, 134_217_728, 2_147_483_648],
        snipes: [0, 1, 0, 0, 0, 2, 0, 0],
        side_to_move: 1,
        pending_animal: u8::MAX,
        pending_destination: 0,
    })
    .unwrap()
}
