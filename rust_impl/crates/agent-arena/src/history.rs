use snipe_core::{Action, Animal, Card, CardMultiset, Player, Rank, State, StepDirection};
use std::time::Duration;

const ANIMALS: [Animal; 16] = [
    Animal::Mouse,
    Animal::Ox,
    Animal::Tiger,
    Animal::Rabbit,
    Animal::Dragon,
    Animal::Snake,
    Animal::Horse,
    Animal::Ram,
    Animal::Monkey,
    Animal::Rooster,
    Animal::Dog,
    Animal::Boar,
    Animal::Fish,
    Animal::Elephant,
    Animal::Squid,
    Animal::Frog,
];
const RANKS: [Rank; 6] = [Rank::R1, Rank::R2, Rank::R3, Rank::R4, Rank::R5, Rank::R6];
const MAJOR_ANIMALS: [Animal; 4] = [
    Animal::Tiger,
    Animal::Dragon,
    Animal::Fish,
    Animal::Elephant,
];

pub(crate) struct HistoryRecorder {
    lines: Vec<String>,
}

impl HistoryRecorder {
    pub(crate) fn new(
        state: &State,
        seed: u64,
        alpha: &str,
        beta: &str,
        alpha_time: Duration,
        beta_time: Duration,
    ) -> Self {
        for player in [Player::Alpha, Player::Beta] {
            assert_eq!(
                initial_major_animal_count(state, player),
                4,
                "the native arena only records standard Major-balanced deals"
            );
        }
        let mut lines = vec![
            format!(
                "// Beta: {beta} ({} seconds per ply)",
                beta_time.as_secs_f64()
            ),
            format!(
                "// Alpha: {alpha} ({} seconds per ply)",
                alpha_time.as_secs_f64()
            ),
            format!("// Seed: {seed}"),
            String::new(),
            format!(
                "0b. ={}; {}; {}; {}",
                format_cards(state.reserves, Player::Beta),
                format_cards(state.r6, Player::Beta),
                format_cards(state.r5, Player::Beta),
                format_cards(state.r4, Player::Beta),
            ),
            format!(
                "0a. ={}; {}; {}; {}",
                format_cards(state.reserves, Player::Alpha),
                format_cards(state.r1, Player::Alpha),
                format_cards(state.r2, Player::Alpha),
                format_cards(state.r3, Player::Alpha),
            ),
        ];
        lines.reserve(258);
        Self { lines }
    }

    pub(crate) fn record_turn(
        &mut self,
        timeline_index: u32,
        state: &State,
        actions: &[Action],
    ) -> Result<(), String> {
        let player = state.active_player;
        let mut position = state.clone();
        let mut steps = Vec::with_capacity(actions.len());
        for &action in actions {
            steps.push(format_action(&position, action));
            position = position
                .apply(action)
                .map_err(|error| format!("cannot record illegal action {action:?}: {error:?}"))?;
        }
        let suffix = match position.winner() {
            Some(Player::Alpha) => "+#0",
            Some(Player::Beta) => "-#0",
            None => "",
        };
        if !suffix.is_empty()
            && let Some(last) = steps.last_mut()
        {
            last.push_str(suffix);
        }
        let side = match player {
            Player::Alpha => 'a',
            Player::Beta => 'b',
        };
        self.lines
            .push(format!("{timeline_index}{side}. {}", steps.join(", ")));
        Ok(())
    }

    pub(crate) fn render(&self, incomplete: bool) -> String {
        let marker = if incomplete { "// INCOMPLETE\n" } else { "" };
        format!("{marker}{}\n", self.lines.join("\n"))
    }
}

fn format_action(state: &State, action: Action) -> String {
    match action {
        Action::AnimalStep(step) => {
            let destination = rank_cards(state, step.destination);
            let captures = step.actor.would_activate_triplet_by_entering(destination);
            let capture_marker = if captures && animal_count(destination) != 0 {
                "x"
            } else {
                ""
            };
            format!(
                "{} {}{}{capture_marker}",
                animal_name(step.actor),
                if step.direction == StepDirection::Retreat {
                    "*"
                } else {
                    ""
                },
                rank_number(step.destination),
            )
        }
        Action::SnipeStep(step) => {
            let source = RANKS
                .into_iter()
                .find(|&rank| rank_cards(state, rank).count(Card::Snipe, state.active_player) != 0)
                .expect("live player has a snipe");
            let retreating = match state.active_player {
                Player::Alpha => rank_number(step.destination) < rank_number(source),
                Player::Beta => rank_number(step.destination) > rank_number(source),
            };
            format!(
                "{} {}{}",
                player_name(state.active_player),
                if retreating { "*" } else { "" },
                rank_number(step.destination),
            )
        }
        Action::Drop(drop) => format!(
            "{} &{}",
            animal_name(drop.actor),
            rank_number(drop.destination)
        ),
    }
}

fn format_cards(cards: CardMultiset, owner: Player) -> String {
    let mut names = Vec::new();
    for animal in ANIMALS {
        for _ in 0..cards.count(Card::Animal(animal), owner) {
            names.push(animal_name(animal));
        }
    }
    if cards.count(Card::Snipe, owner) != 0 {
        names.push(player_name(owner));
    }
    names.join(" ")
}

fn animal_count(cards: CardMultiset) -> u32 {
    ANIMALS
        .into_iter()
        .map(|animal| {
            u32::from(cards.count(Card::Animal(animal), Player::Alpha))
                + u32::from(cards.count(Card::Animal(animal), Player::Beta))
        })
        .sum()
}

fn initial_major_animal_count(state: &State, player: Player) -> u32 {
    let locations = match player {
        Player::Alpha => [state.reserves, state.r1, state.r2, state.r3],
        Player::Beta => [state.reserves, state.r4, state.r5, state.r6],
    };
    locations
        .into_iter()
        .map(|cards| {
            MAJOR_ANIMALS
                .into_iter()
                .map(|animal| u32::from(cards.count(Card::Animal(animal), player)))
                .sum::<u32>()
        })
        .sum()
}

fn animal_name(animal: Animal) -> &'static str {
    match animal {
        Animal::Mouse => "Rat",
        Animal::Ox => "Ox",
        Animal::Tiger => "Tiger",
        Animal::Rabbit => "Rabbit",
        Animal::Dragon => "Dragon",
        Animal::Snake => "Snake",
        Animal::Horse => "Horse",
        Animal::Ram => "Ram",
        Animal::Monkey => "Monkey",
        Animal::Rooster => "Rooster",
        Animal::Dog => "Dog",
        Animal::Boar => "Boar",
        Animal::Fish => "Fish",
        Animal::Elephant => "Elephant",
        Animal::Squid => "Squid",
        Animal::Frog => "Frog",
    }
}

fn player_name(player: Player) -> &'static str {
    match player {
        Player::Alpha => "Alpha",
        Player::Beta => "Beta",
    }
}

fn rank_number(rank: Rank) -> u8 {
    match rank {
        Rank::R1 => 1,
        Rank::R2 => 2,
        Rank::R3 => 3,
        Rank::R4 => 4,
        Rank::R5 => 5,
        Rank::R6 => 6,
    }
}

fn rank_cards(state: &State, rank: Rank) -> CardMultiset {
    match rank {
        Rank::R1 => state.r1,
        Rank::R2 => state.r2,
        Rank::R3 => state.r3,
        Rank::R4 => state.r4,
        Rank::R5 => state.r5,
        Rank::R6 => state.r6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_prng::initial_state;

    #[test]
    fn incomplete_marker_wraps_an_initial_position_without_plies() {
        let history = HistoryRecorder::new(
            &initial_state(7),
            7,
            "Avocado",
            "Cherry",
            Duration::from_secs(10),
            Duration::from_secs(10),
        );

        let incomplete = history.render(true);
        assert!(incomplete.starts_with("// INCOMPLETE\n// Beta: Cherry"));
        assert!(incomplete.contains("\n// Alpha: Avocado (10 seconds per ply)\n"));
        assert!(incomplete.contains("\n// Seed: 7\n\n0b. ="));
        assert!(incomplete.contains("\n0a. ="));
        assert!(!incomplete.contains("\n1a."));
        assert!(!incomplete.contains("\n1b."));

        let complete = history.render(false);
        assert!(!complete.contains("// INCOMPLETE"));
        assert!(complete.starts_with("// Beta: Cherry"));
    }
}
