use snipe_core::{
    Action, Animal, AnimalDrop, AnimalStep, Card, Player, Rank, SnipeStep, State, StepDirection,
};
use std::{cmp::Reverse, collections::HashMap};

pub(crate) const ANIMALS: [Animal; 16] = [
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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct Cell {
    alpha: u16,
    beta: u16,
    twins: u16,
    snipes: u8,
}

impl Cell {
    pub(crate) fn animal_count(self, animal: usize, player: Player) -> u8 {
        let mask = 1_u16 << animal;
        let owned = match player {
            Player::Alpha => self.alpha,
            Player::Beta => self.beta,
        };
        if owned & mask == 0 {
            0
        } else if self.twins & mask != 0 {
            2
        } else {
            1
        }
    }

    pub(crate) fn owned_presence(self, player: Player) -> u16 {
        match player {
            Player::Alpha => self.alpha,
            Player::Beta => self.beta,
        }
    }

    pub(crate) fn presence(self) -> u16 {
        self.alpha | self.beta
    }

    pub(crate) fn has_snipe(self, player: Player) -> bool {
        self.snipes & player_mask(player) != 0
    }

    pub(crate) fn card_count(self) -> u32 {
        let mut count = self.snipes.count_ones();
        for animal in 0..16 {
            count += u32::from(self.animal_count(animal, Player::Alpha));
            count += u32::from(self.animal_count(animal, Player::Beta));
        }
        count
    }

    pub(crate) fn owned_animal_count(self, player: Player) -> u32 {
        (0..16)
            .map(|animal| u32::from(self.animal_count(animal, player)))
            .sum()
    }

    fn add_animal(&mut self, animal: usize, player: Player) {
        let mask = 1_u16 << animal;
        let owned = match player {
            Player::Alpha => &mut self.alpha,
            Player::Beta => &mut self.beta,
        };
        if *owned & mask != 0 {
            debug_assert!(self.twins & mask == 0);
            self.twins |= mask;
        } else {
            *owned |= mask;
        }
    }

    fn remove_animal(&mut self, animal: usize, player: Player) {
        let mask = 1_u16 << animal;
        let owned = match player {
            Player::Alpha => &mut self.alpha,
            Player::Beta => &mut self.beta,
        };
        debug_assert!(*owned & mask != 0);
        if self.twins & mask != 0 {
            self.twins &= !mask;
        } else {
            *owned &= !mask;
        }
    }

    fn add_snipe(&mut self, player: Player) {
        self.snipes |= player_mask(player);
    }

    fn remove_snipe(&mut self, player: Player) {
        debug_assert!(self.has_snipe(player));
        self.snipes &= !player_mask(player);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Position {
    pub(crate) cells: [Cell; 7],
    pub(crate) active: Player,
    pub(crate) leading: Option<AnimalStep>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Turn {
    pub(crate) first: Action,
    pub(crate) second: Option<Action>,
    pub(crate) next: Position,
    pub(crate) capture_count: u8,
    pub(crate) order_score: i32,
}

#[derive(Clone, Copy)]
struct GeneratedAction {
    action: Action,
    capture_count: u8,
    order_score: i32,
}

impl Position {
    pub(crate) fn from_core(state: &State) -> Self {
        let core_cells = [
            state.reserves,
            state.r1,
            state.r2,
            state.r3,
            state.r4,
            state.r5,
            state.r6,
        ];
        let mut cells = [Cell::default(); 7];
        for (location, cards) in core_cells.into_iter().enumerate() {
            for (animal, kind) in ANIMALS.into_iter().enumerate() {
                for player in [Player::Alpha, Player::Beta] {
                    for _ in 0..cards.count(Card::Animal(kind), player) {
                        cells[location].add_animal(animal, player);
                    }
                }
            }
            for player in [Player::Alpha, Player::Beta] {
                if cards.count(Card::Snipe, player) != 0 {
                    cells[location].add_snipe(player);
                }
            }
        }
        Self {
            cells,
            active: state.active_player,
            leading: state.leading_action,
        }
    }

    pub(crate) fn captured_winner(self) -> Option<Player> {
        if self.cells[0].has_snipe(Player::Beta) {
            Some(Player::Alpha)
        } else if self.cells[0].has_snipe(Player::Alpha) {
            Some(Player::Beta)
        } else {
            None
        }
    }

    pub(crate) fn snipe_location(self, player: Player) -> Option<usize> {
        (1..=6).find(|&rank| self.cells[rank].has_snipe(player))
    }

    #[cfg(test)]
    pub(crate) fn legal_actions(self) -> Vec<Action> {
        self.generated_actions()
            .into_iter()
            .map(|generated| generated.action)
            .collect()
    }

    pub(crate) fn has_legal_action(self) -> bool {
        !self.generated_actions().is_empty()
    }

    pub(crate) fn winner(self) -> Option<Player> {
        self.captured_winner()
            .or_else(|| (!self.has_legal_action()).then(|| self.active.opponent()))
    }

    pub(crate) fn turns(self) -> Vec<Turn> {
        if self.captured_winner().is_some() {
            return Vec::new();
        }
        let first_actions = self.generated_actions();
        let mut turns = Vec::with_capacity(first_actions.len().saturating_mul(4));
        let mut by_position = HashMap::<Position, usize>::new();
        for first in first_actions {
            let after_first = self.apply_generated(first);
            if after_first.captured_winner().is_some() || after_first.active != self.active {
                insert_turn(
                    &mut turns,
                    &mut by_position,
                    Turn {
                        first: first.action,
                        second: None,
                        next: after_first,
                        capture_count: first.capture_count,
                        order_score: first.order_score,
                    },
                );
                continue;
            }
            let second_actions = after_first.generated_actions();
            if second_actions.is_empty() {
                insert_turn(
                    &mut turns,
                    &mut by_position,
                    Turn {
                        first: first.action,
                        second: None,
                        next: after_first,
                        capture_count: first.capture_count,
                        order_score: first.order_score,
                    },
                );
                continue;
            }
            for second in second_actions {
                insert_turn(
                    &mut turns,
                    &mut by_position,
                    Turn {
                        first: first.action,
                        second: Some(second.action),
                        next: after_first.apply_generated(second),
                        capture_count: first.capture_count.saturating_add(second.capture_count),
                        order_score: first.order_score.saturating_add(second.order_score),
                    },
                );
            }
        }
        turns.sort_unstable_by_key(|turn| Reverse(turn.order_score));
        turns
    }

    pub(crate) fn apply(self, action: Action) -> Option<Self> {
        let generated = self
            .generated_actions()
            .into_iter()
            .find(|candidate| candidate.action == action)?;
        Some(self.apply_generated(generated))
    }

    fn generated_actions(self) -> Vec<GeneratedAction> {
        if self.captured_winner().is_some() {
            return Vec::new();
        }
        let mut actions = Vec::with_capacity(96);
        if self.leading.is_none() {
            if let Some(source) = self.snipe_location(self.active)
                && self.cells[source].card_count() > 1
            {
                for destination in [advance(self.active, source), retreat(self.active, source)]
                    .into_iter()
                    .flatten()
                {
                    let action = Action::SnipeStep(SnipeStep {
                        destination: rank(destination),
                    });
                    actions.push(GeneratedAction {
                        action,
                        capture_count: 0,
                        order_score: self.action_score(action, 0),
                    });
                }
            }
            if self.cells[0].owned_animal_count(self.active) > 1 {
                let mut animals = self.cells[0].owned_presence(self.active);
                while animals != 0 {
                    let animal = animals.trailing_zeros() as usize;
                    animals &= animals - 1;
                    for destination in 1..=6 {
                        if !ANIMALS[animal].is_retreater()
                            || legal_retreater_drop(self.active, destination)
                        {
                            let action = Action::Drop(AnimalDrop {
                                actor: ANIMALS[animal],
                                destination: rank(destination),
                            });
                            actions.push(GeneratedAction {
                                action,
                                capture_count: 0,
                                order_score: self.action_score(action, 0),
                            });
                        }
                    }
                }
            }
        }

        for source in 1..=6 {
            let mut animals = self.cells[source].owned_presence(self.active);
            while animals != 0 {
                let animal = animals.trailing_zeros() as usize;
                animals &= animals - 1;
                for direction in [StepDirection::Advance, StepDirection::Retreat] {
                    if direction == StepDirection::Retreat && !ANIMALS[animal].is_retreater() {
                        continue;
                    }
                    let destination = match direction {
                        StepDirection::Advance => advance(self.active, source),
                        StepDirection::Retreat => retreat(self.active, source),
                    };
                    let Some(destination) = destination else {
                        continue;
                    };
                    if let Some(leading) = self.leading
                        && leading.actor == ANIMALS[animal]
                        && number(leading.destination) == source
                        && self.cells[source].animal_count(animal, self.active) < 2
                    {
                        continue;
                    }
                    let capture = activates(animal, self.cells[destination].presence());
                    let enemy_snipe = self.cells[destination].has_snipe(self.active.opponent());
                    let friendly_snipe = self.cells[destination].has_snipe(self.active);
                    if self.cells[source].card_count() <= 1 {
                        if !capture || !enemy_snipe {
                            continue;
                        }
                    } else if capture && friendly_snipe && !enemy_snipe {
                        continue;
                    }
                    let action = Action::AnimalStep(AnimalStep {
                        actor: ANIMALS[animal],
                        direction,
                        destination: rank(destination),
                    });
                    let capture_count = if capture {
                        self.cells[destination].card_count().min(u32::from(u8::MAX)) as u8
                    } else {
                        0
                    };
                    actions.push(GeneratedAction {
                        action,
                        capture_count,
                        order_score: self.action_score(action, capture_count),
                    });
                }
            }
        }
        actions.sort_unstable_by_key(|action| Reverse(action.order_score));
        actions
    }

    fn apply_generated(self, generated: GeneratedAction) -> Self {
        let mut next = self;
        match generated.action {
            Action::SnipeStep(step) => {
                let source = self
                    .snipe_location(self.active)
                    .expect("generated snipe has source");
                let destination = number(step.destination);
                next.cells[source].remove_snipe(self.active);
                next.cells[destination].add_snipe(self.active);
                next.active = self.active.opponent();
            }
            Action::Drop(drop) => {
                let animal = animal_index(drop.actor);
                next.cells[0].remove_animal(animal, self.active);
                next.cells[number(drop.destination)].add_animal(animal, self.active);
                next.active = self.active.opponent();
            }
            Action::AnimalStep(step) => {
                let animal = animal_index(step.actor);
                let destination = number(step.destination);
                let source = match step.direction {
                    StepDirection::Advance => retreat(self.active, destination),
                    StepDirection::Retreat => advance(self.active, destination),
                }
                .expect("generated animal step has source");
                next.cells[source].remove_animal(animal, self.active);
                if generated.capture_count != 0 {
                    let captured = next.cells[destination];
                    next.capture_into_reserve(captured);
                    next.cells[destination] = Cell::default();
                }
                next.cells[destination].add_animal(animal, self.active);
                if self.leading.is_none() {
                    next.leading = Some(step);
                } else {
                    next.leading = None;
                    next.active = self.active.opponent();
                }
            }
        }
        next
    }

    fn capture_into_reserve(&mut self, captured: Cell) {
        for animal in 0..16 {
            let count = captured.animal_count(animal, Player::Alpha)
                + captured.animal_count(animal, Player::Beta);
            for _ in 0..count {
                self.cells[0].add_animal(animal, self.active);
            }
        }
        for player in [Player::Alpha, Player::Beta] {
            if captured.has_snipe(player) {
                self.cells[0].add_snipe(player);
            }
        }
    }

    fn action_score(self, action: Action, capture_count: u8) -> i32 {
        let enemy_snipe = self.snipe_location(self.active.opponent()).unwrap_or(3) as i32;
        match action {
            Action::AnimalStep(step) => {
                let destination = number(step.destination) as i32;
                let progress = match self.active {
                    Player::Alpha => destination,
                    Player::Beta => 7 - destination,
                };
                i32::from(capture_count) * 20_000
                    + i32::from(self.cells[destination as usize].has_snipe(self.active.opponent()))
                        * 1_000_000
                    + 2_000
                    + progress * 30
                    - (destination - enemy_snipe).abs() * 25
            }
            Action::Drop(drop) => {
                let destination = number(drop.destination) as i32;
                800 - (destination - enemy_snipe).abs() * 20
            }
            Action::SnipeStep(step) => {
                let destination = number(step.destination) as i32;
                300 + (destination - enemy_snipe).abs() * 10
            }
        }
    }
}

fn insert_turn(turns: &mut Vec<Turn>, by_position: &mut HashMap<Position, usize>, candidate: Turn) {
    if let Some(&index) = by_position.get(&candidate.next) {
        if candidate.order_score > turns[index].order_score {
            turns[index] = candidate;
        }
    } else {
        by_position.insert(candidate.next, turns.len());
        turns.push(candidate);
    }
}

pub(crate) const fn animal_index(animal: Animal) -> usize {
    animal as usize
}

pub(crate) const fn number(rank: Rank) -> usize {
    rank as usize + 1
}

pub(crate) const fn rank(number: usize) -> Rank {
    match number {
        1 => Rank::R1,
        2 => Rank::R2,
        3 => Rank::R3,
        4 => Rank::R4,
        5 => Rank::R5,
        6 => Rank::R6,
        _ => panic!("rank out of range"),
    }
}

pub(crate) const fn advance(player: Player, source: usize) -> Option<usize> {
    match player {
        Player::Alpha if source < 6 => Some(source + 1),
        Player::Beta if source > 1 => Some(source - 1),
        _ => None,
    }
}

pub(crate) const fn retreat(player: Player, source: usize) -> Option<usize> {
    match player {
        Player::Alpha if source > 1 => Some(source - 1),
        Player::Beta if source < 6 => Some(source + 1),
        _ => None,
    }
}

const fn legal_retreater_drop(player: Player, destination: usize) -> bool {
    match player {
        Player::Alpha => destination <= 4,
        Player::Beta => destination >= 3,
    }
}

const fn player_mask(player: Player) -> u8 {
    match player {
        Player::Alpha => 1,
        Player::Beta => 2,
    }
}

// Three bits encode the unary, binary and ternary roles for one element.
const ROLE_MASKS: [[u16; 3]; 4] = [
    [
        (1 << 9) | (1 << 11) | (1 << 14),
        (1 << 0) | (1 << 6) | (1 << 10),
        1 << 2,
    ],
    [
        (1 << 1) | (1 << 3) | (1 << 10),
        (1 << 5) | (1 << 14) | (1 << 15),
        1 << 12,
    ],
    [
        (1 << 0) | (1 << 5) | (1 << 8),
        (1 << 1) | (1 << 7) | (1 << 11),
        1 << 13,
    ],
    [
        (1 << 6) | (1 << 7) | (1 << 15),
        (1 << 3) | (1 << 8) | (1 << 9),
        1 << 4,
    ],
];

pub(crate) fn activates(actor: usize, destination: u16) -> bool {
    let actor_mask = 1_u16 << actor;
    let animals = destination | actor_mask;
    ROLE_MASKS.iter().any(|roles| {
        roles.iter().any(|role| role & actor_mask != 0)
            && roles.iter().all(|role| role & animals != 0)
    })
}
