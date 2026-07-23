//! Authoritative, dependency-free rules for Snipe Hunt.
//!
//! A [`State`] is small and `Copy`, so search code can apply moves by value.
//! The board representation mirrors the original TypeScript implementation:
//! each location contains two 32-bit animal sets and a two-bit snipe set.

use core::fmt;
use core::hash::{Hash, Hasher};

pub const STATE_VERSION: u8 = 1;
pub const LOCATION_COUNT: usize = 8;
pub const ANIMAL_COUNT: usize = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Player {
    Alpha = 0,
    Beta = 1,
}

impl Player {
    #[inline]
    pub const fn opponent(self) -> Self {
        match self {
            Self::Alpha => Self::Beta,
            Self::Beta => Self::Alpha,
        }
    }

    #[inline]
    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Row {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

impl Row {
    pub const ALL: [Self; 6] = [
        Self::One,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
        Self::Six,
    ];

    pub const fn new(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            3 => Some(Self::Three),
            4 => Some(Self::Four),
            5 => Some(Self::Five),
            6 => Some(Self::Six),
            _ => None,
        }
    }

    #[inline]
    pub const fn number(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn location(self) -> Location {
        Location::from_index(self as usize)
    }

    #[inline]
    pub const fn reflected(self) -> Self {
        match self {
            Self::One => Self::Six,
            Self::Two => Self::Five,
            Self::Three => Self::Four,
            Self::Four => Self::Three,
            Self::Five => Self::Two,
            Self::Six => Self::One,
        }
    }

    pub const fn forward(self, player: Player) -> Option<Self> {
        let n = self as u8;
        match player {
            Player::Alpha => Self::new(n + 1),
            Player::Beta => Self::new(n.saturating_sub(1)),
        }
    }

    pub const fn backward(self, player: Player) -> Option<Self> {
        self.forward(player.opponent())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Location {
    AlphaReserve = 0,
    Row1 = 1,
    Row2 = 2,
    Row3 = 3,
    Row4 = 4,
    Row5 = 5,
    Row6 = 6,
    BetaReserve = 7,
}

impl Location {
    pub const ALL: [Self; 8] = [
        Self::AlphaReserve,
        Self::Row1,
        Self::Row2,
        Self::Row3,
        Self::Row4,
        Self::Row5,
        Self::Row6,
        Self::BetaReserve,
    ];

    pub const fn from_index(index: usize) -> Self {
        match index {
            0 => Self::AlphaReserve,
            1 => Self::Row1,
            2 => Self::Row2,
            3 => Self::Row3,
            4 => Self::Row4,
            5 => Self::Row5,
            6 => Self::Row6,
            7 => Self::BetaReserve,
            _ => panic!("location index out of range"),
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[inline]
    pub const fn row(self) -> Option<Row> {
        Row::new(self as u8)
    }

    #[inline]
    pub const fn reserve_of(player: Player) -> Self {
        match player {
            Player::Alpha => Self::AlphaReserve,
            Player::Beta => Self::BetaReserve,
        }
    }

    #[inline]
    pub const fn reflected(self) -> Self {
        Self::from_index(7 - self.index())
    }
}

/// The numeric values intentionally match the TypeScript `CardType` values.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Animal {
    Mouse1 = 0,
    Ox1 = 1,
    Tiger1 = 2,
    Rabbit1 = 3,
    Dragon1 = 4,
    Snake1 = 5,
    Horse1 = 6,
    Ram1 = 7,
    Monkey1 = 8,
    Rooster1 = 9,
    Dog1 = 10,
    Boar1 = 11,
    Fish1 = 12,
    Elephant1 = 13,
    Squid1 = 14,
    Frog1 = 15,
    Mouse2 = 16,
    Ox2 = 17,
    Tiger2 = 18,
    Rabbit2 = 19,
    Dragon2 = 20,
    Snake2 = 21,
    Horse2 = 22,
    Ram2 = 23,
    Monkey2 = 24,
    Rooster2 = 25,
    Dog2 = 26,
    Boar2 = 27,
    Fish2 = 28,
    Elephant2 = 29,
    Squid2 = 30,
    Frog2 = 31,
}

impl Animal {
    pub const ALL: [Self; ANIMAL_COUNT] = [
        Self::Mouse1,
        Self::Ox1,
        Self::Tiger1,
        Self::Rabbit1,
        Self::Dragon1,
        Self::Snake1,
        Self::Horse1,
        Self::Ram1,
        Self::Monkey1,
        Self::Rooster1,
        Self::Dog1,
        Self::Boar1,
        Self::Fish1,
        Self::Elephant1,
        Self::Squid1,
        Self::Frog1,
        Self::Mouse2,
        Self::Ox2,
        Self::Tiger2,
        Self::Rabbit2,
        Self::Dragon2,
        Self::Snake2,
        Self::Horse2,
        Self::Ram2,
        Self::Monkey2,
        Self::Rooster2,
        Self::Dog2,
        Self::Boar2,
        Self::Fish2,
        Self::Elephant2,
        Self::Squid2,
        Self::Frog2,
    ];

    pub const fn from_index(index: u8) -> Option<Self> {
        if index < ANIMAL_COUNT as u8 {
            // SAFETY: repr(u8) is dense from 0 through 31.
            Some(unsafe { core::mem::transmute::<u8, Self>(index) })
        } else {
            None
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[inline]
    pub const fn bit(self) -> u32 {
        1_u32 << self as u8
    }

    #[inline]
    pub const fn can_retreat(self) -> bool {
        matches!((self as u8) & 15, 0 | 3 | 5 | 7 | 11 | 14)
    }

    #[inline]
    const fn element_counts(self) -> u16 {
        const F1: u16 = 1 << 0;
        const F2: u16 = 1 << 1;
        const F3: u16 = 1 << 2;
        const W1: u16 = 1 << 3;
        const W2: u16 = 1 << 4;
        const W3: u16 = 1 << 5;
        const E1: u16 = 1 << 6;
        const E2: u16 = 1 << 7;
        const E3: u16 = 1 << 8;
        const A1: u16 = 1 << 9;
        const A2: u16 = 1 << 10;
        const A3: u16 = 1 << 11;
        match (self as u8) & 15 {
            0 => F2 | E1,
            1 => E2 | W1,
            2 => F3,
            3 => A2 | W1,
            4 => A3,
            5 => W2 | E1,
            6 => F2 | A1,
            7 => E2 | A1,
            8 => A2 | E1,
            9 => A2 | F1,
            10 => F2 | W1,
            11 => E2 | F1,
            12 => W3,
            13 => E3,
            14 => W2 | F1,
            15 => W2 | A1,
            _ => unreachable!(),
        }
    }

    #[inline]
    const fn triplet_shifts(self) -> (u8, u8) {
        match (self as u8) & 15 {
            0 => (0, 6),   // fire, earth
            1 => (6, 3),   // earth, water
            2 => (0, 12),  // fire
            3 => (9, 3),   // air, water
            4 => (9, 12),  // air
            5 => (3, 6),   // water, earth
            6 => (0, 9),   // fire, air
            7 => (6, 9),   // earth, air
            8 => (9, 6),   // air, earth
            9 => (9, 0),   // air, fire
            10 => (0, 3),  // fire, water
            11 => (6, 0),  // earth, fire
            12 => (3, 12), // water
            13 => (6, 12), // earth
            14 => (3, 0),  // water, fire
            15 => (3, 9),  // water, air
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Card {
    Animal(Animal),
    Snipe(Player),
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Cell {
    animals: [u32; 2],
    snipes: u8,
}

/// Primitive-only board data suitable for JSON/WASM conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateData {
    pub alpha_animals: [u32; LOCATION_COUNT],
    pub beta_animals: [u32; LOCATION_COUNT],
    pub snipes: [u8; LOCATION_COUNT],
    pub side_to_move: u8,
    /// `u8::MAX` means there is no pending first animal step.
    pub pending_animal: u8,
    /// `0` means there is no pending first animal step; rows are 1 through 6.
    pub pending_destination: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidState {
    InvalidPlayer,
    InvalidPendingAnimal,
    InvalidPendingDestination,
    PartialPendingStep,
    DuplicateAnimal,
    InvalidSnipeBits,
}

impl Cell {
    #[inline]
    pub const fn animals(self, owner: Player) -> u32 {
        self.animals[owner.index()]
    }

    #[inline]
    pub const fn all_animals(self) -> u32 {
        self.animals[0] | self.animals[1]
    }

    #[inline]
    pub const fn snipes(self) -> u8 {
        self.snipes
    }

    #[inline]
    pub const fn has_snipe(self, player: Player) -> bool {
        self.snipes & (1 << player as u8) != 0
    }

    #[inline]
    pub const fn card_count(self) -> u32 {
        self.all_animals().count_ones() + self.snipes.count_ones()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AnimalStep {
    pub moved: Animal,
    pub destination: Row,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AtomicMove {
    Snipe { destination: Row },
    Drop { animal: Animal, destination: Row },
    Animal(AnimalStep),
}

/// A search move represents one whole turn.
///
/// `second` is `None` only when applying `first` immediately ends the game
/// (normally by capturing a snipe).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Move {
    Snipe {
        destination: Row,
    },
    Drop {
        animal: Animal,
        destination: Row,
    },
    Animals {
        first: AnimalStep,
        second: Option<AnimalStep>,
    },
}

impl Move {
    pub const fn reflected(self) -> Self {
        match self {
            Self::Snipe { destination } => Self::Snipe {
                destination: destination.reflected(),
            },
            Self::Drop {
                animal,
                destination,
            } => Self::Drop {
                animal,
                destination: destination.reflected(),
            },
            Self::Animals { first, second } => Self::Animals {
                first: AnimalStep {
                    moved: first.moved,
                    destination: first.destination.reflected(),
                },
                second: match second {
                    Some(step) => Some(AnimalStep {
                        moved: step.moved,
                        destination: step.destination.reflected(),
                    }),
                    None => None,
                },
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IllegalMove {
    SnipeAlreadyCaptured,
    AlreadyMovedAnimal,
    StepDestinationOutOfRange,
    CannotEmptyRowWithoutImmediatelyWinning,
    DroppedAnimalNotInReserve,
    CannotEmptyReserve,
    CannotDropRetreaterOnEnemyBackTwoRows,
    MovedCardInReserve,
    CardNotFound,
    NotYourAnimal,
    CannotMoveSameAnimalTwice,
    CannotCaptureOwnSnipeWithoutAlsoCapturingOpponent,
    IncompleteAnimalTurn,
    UnexpectedSecondAnimalStep,
}

impl fmt::Display for IllegalMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Undo {
    previous: State,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveOutcome {
    pub state: State,
    /// Animals swept from the activating row into the mover's reserve.
    pub captured_animals: u32,
    /// Bit zero is Alpha's snipe and bit one is Beta's.
    pub captured_snipes: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct State {
    cells: [Cell; LOCATION_COUNT],
    side_to_move: Player,
    pending: Option<AnimalStep>,
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cells.hash(state);
        self.side_to_move.hash(state);
        self.pending.hash(state);
    }
}

impl Default for State {
    fn default() -> Self {
        Self::empty(Player::Beta)
    }
}

impl State {
    pub const fn empty(side_to_move: Player) -> Self {
        Self {
            cells: [Cell {
                animals: [0; 2],
                snipes: 0,
            }; LOCATION_COUNT],
            side_to_move,
            pending: None,
        }
    }

    /// Deterministically deals a legal initial position from `seed`.
    pub fn initial(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let mut minors = [
            Animal::Mouse1,
            Animal::Ox1,
            Animal::Rabbit1,
            Animal::Snake1,
            Animal::Horse1,
            Animal::Ram1,
            Animal::Monkey1,
            Animal::Rooster1,
            Animal::Dog1,
            Animal::Boar1,
            Animal::Squid1,
            Animal::Frog1,
            Animal::Mouse2,
            Animal::Ox2,
            Animal::Rabbit2,
            Animal::Snake2,
            Animal::Horse2,
            Animal::Ram2,
            Animal::Monkey2,
            Animal::Rooster2,
            Animal::Dog2,
            Animal::Boar2,
            Animal::Squid2,
            Animal::Frog2,
        ];
        let mut majors = [
            Animal::Tiger1,
            Animal::Dragon1,
            Animal::Fish1,
            Animal::Elephant1,
            Animal::Tiger2,
            Animal::Dragon2,
            Animal::Fish2,
            Animal::Elephant2,
        ];
        shuffle(&mut minors, &mut rng);
        shuffle(&mut majors, &mut rng);

        let mut alpha = [Animal::Mouse1; 16];
        let mut beta = [Animal::Mouse1; 16];
        alpha[..12].copy_from_slice(&minors[..12]);
        alpha[12..].copy_from_slice(&majors[..4]);
        beta[..12].copy_from_slice(&minors[12..]);
        beta[12..].copy_from_slice(&majors[4..]);
        shuffle(&mut alpha, &mut rng);
        shuffle(&mut beta, &mut rng);

        let mut state = Self::empty(Player::Beta);
        // Match the original pop-based layout exactly.
        state.put_animal(Location::AlphaReserve, alpha[15], Player::Alpha);
        state.put_animal(Location::Row1, alpha[14], Player::Alpha);
        state.put_snipe(Location::Row1, Player::Alpha);
        state.put_animal(Location::Row1, alpha[13], Player::Alpha);
        for &animal in alpha[1..13].iter().rev() {
            state.put_animal(Location::Row2, animal, Player::Alpha);
        }
        state.put_animal(Location::Row3, alpha[0], Player::Alpha);

        state.put_animal(Location::Row4, beta[15], Player::Beta);
        for &animal in beta[3..15].iter().rev() {
            state.put_animal(Location::Row5, animal, Player::Beta);
        }
        state.put_animal(Location::Row6, beta[2], Player::Beta);
        state.put_snipe(Location::Row6, Player::Beta);
        state.put_animal(Location::Row6, beta[1], Player::Beta);
        state.put_animal(Location::BetaReserve, beta[0], Player::Beta);
        state
    }

    pub fn from_data(data: StateData) -> Result<Self, InvalidState> {
        let side_to_move = match data.side_to_move {
            0 => Player::Alpha,
            1 => Player::Beta,
            _ => return Err(InvalidState::InvalidPlayer),
        };
        let pending = match (data.pending_animal, data.pending_destination) {
            (u8::MAX, 0) => None,
            (u8::MAX, _) | (_, 0) => return Err(InvalidState::PartialPendingStep),
            (animal, destination) => Some(AnimalStep {
                moved: Animal::from_index(animal).ok_or(InvalidState::InvalidPendingAnimal)?,
                destination: Row::new(destination)
                    .ok_or(InvalidState::InvalidPendingDestination)?,
            }),
        };
        let mut cells = [Cell::default(); LOCATION_COUNT];
        let mut seen = 0_u32;
        for (index, cell) in cells.iter_mut().enumerate() {
            let alpha = data.alpha_animals[index];
            let beta = data.beta_animals[index];
            if seen & (alpha | beta) != 0 || alpha & beta != 0 {
                return Err(InvalidState::DuplicateAnimal);
            }
            if data.snipes[index] & !0b11 != 0 {
                return Err(InvalidState::InvalidSnipeBits);
            }
            seen |= alpha | beta;
            *cell = Cell {
                animals: [alpha, beta],
                snipes: data.snipes[index],
            };
        }
        Ok(Self {
            cells,
            side_to_move,
            pending,
        })
    }

    pub fn to_data(self) -> StateData {
        let mut data = StateData {
            alpha_animals: [0; LOCATION_COUNT],
            beta_animals: [0; LOCATION_COUNT],
            snipes: [0; LOCATION_COUNT],
            side_to_move: self.side_to_move as u8,
            pending_animal: u8::MAX,
            pending_destination: 0,
        };
        for index in 0..LOCATION_COUNT {
            data.alpha_animals[index] = self.cells[index].animals[0];
            data.beta_animals[index] = self.cells[index].animals[1];
            data.snipes[index] = self.cells[index].snipes;
        }
        if let Some(step) = self.pending {
            data.pending_animal = step.moved as u8;
            data.pending_destination = step.destination as u8;
        }
        data
    }

    /// Converts to the original TypeScript `Int32Array(24)` board layout.
    pub fn to_legacy_board(self) -> [i32; 24] {
        let mut board = [0_i32; 24];
        for index in 0..LOCATION_COUNT {
            board[index * 3] = self.cells[index].animals[0] as i32;
            board[index * 3 + 1] = self.cells[index].animals[1] as i32;
            board[index * 3 + 2] = self.cells[index].snipes as i32;
        }
        board
    }

    pub fn from_legacy_board(
        board: [i32; 24],
        side_to_move: Player,
        pending: Option<AnimalStep>,
    ) -> Result<Self, InvalidState> {
        let mut data = StateData {
            alpha_animals: [0; LOCATION_COUNT],
            beta_animals: [0; LOCATION_COUNT],
            snipes: [0; LOCATION_COUNT],
            side_to_move: side_to_move as u8,
            pending_animal: u8::MAX,
            pending_destination: 0,
        };
        for index in 0..LOCATION_COUNT {
            data.alpha_animals[index] = board[index * 3] as u32;
            data.beta_animals[index] = board[index * 3 + 1] as u32;
            if !(0..=3).contains(&board[index * 3 + 2]) {
                return Err(InvalidState::InvalidSnipeBits);
            }
            data.snipes[index] = board[index * 3 + 2] as u8;
        }
        if let Some(step) = pending {
            data.pending_animal = step.moved as u8;
            data.pending_destination = step.destination as u8;
        }
        Self::from_data(data)
    }

    #[inline]
    pub const fn side_to_move(self) -> Player {
        self.side_to_move
    }

    #[inline]
    pub const fn pending_animal_step(self) -> Option<AnimalStep> {
        self.pending
    }

    #[inline]
    pub const fn cell(self, location: Location) -> Cell {
        self.cells[location.index()]
    }

    #[inline]
    pub const fn animal_bits(self, location: Location, owner: Player) -> u32 {
        self.cell(location).animals(owner)
    }

    #[inline]
    pub const fn animal_count(self, location: Location, owner: Player) -> u32 {
        self.animal_bits(location, owner).count_ones()
    }

    #[inline]
    pub const fn reserve_count(self, player: Player) -> u32 {
        self.animal_count(Location::reserve_of(player), player)
    }

    pub fn location_of_animal(self, animal: Animal) -> Option<Location> {
        let bit = animal.bit();
        Location::ALL
            .into_iter()
            .find(|&location| self.cell(location).all_animals() & bit != 0)
    }

    pub fn owner_of_animal(self, animal: Animal) -> Option<Player> {
        let location = self.location_of_animal(animal)?;
        if self.animal_bits(location, Player::Alpha) & animal.bit() != 0 {
            Some(Player::Alpha)
        } else {
            Some(Player::Beta)
        }
    }

    pub fn snipe_location(self, player: Player) -> Option<Location> {
        Location::ALL
            .into_iter()
            .find(|&location| self.cell(location).has_snipe(player))
    }

    /// A stable cross-process hash (FNV-1a), independent of Rust's `Hasher`.
    pub fn position_hash(self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        let mut add = |byte: u8| {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        };
        for cell in self.cells {
            for bits in cell.animals {
                for byte in bits.to_le_bytes() {
                    add(byte);
                }
            }
            add(cell.snipes);
        }
        add(self.side_to_move as u8);
        match self.pending {
            Some(step) => {
                add(1);
                add(step.moved as u8);
                add(step.destination as u8);
            }
            None => add(0),
        }
        hash
    }

    pub fn captured_snipe_winner(self) -> Option<Player> {
        if self.cell(Location::BetaReserve).has_snipe(Player::Alpha) {
            Some(Player::Beta)
        } else if self.cell(Location::AlphaReserve).has_snipe(Player::Beta) {
            Some(Player::Alpha)
        } else {
            None
        }
    }

    pub fn winner(self) -> Option<Player> {
        if let Some(winner) = self.captured_snipe_winner() {
            return Some(winner);
        }
        if !self.has_legal_move() {
            Some(self.side_to_move.opponent())
        } else {
            None
        }
    }

    /// Checks move availability without materializing every complete two-step
    /// turn. This is on the search hot path: an opening position can have
    /// hundreds of full turns even though finding the first one is trivial.
    pub fn has_legal_move(self) -> bool {
        if self.captured_snipe_winner().is_some() {
            return false;
        }

        let mut atomics = Vec::new();
        self.legal_atomics_into(&mut atomics);
        if self.pending.is_some() {
            return !atomics.is_empty();
        }

        for atomic in atomics {
            match atomic {
                AtomicMove::Snipe { .. } | AtomicMove::Drop { .. } => return true,
                AtomicMove::Animal(first) => {
                    let after_first = self
                        .apply_atomic(AtomicMove::Animal(first))
                        .expect("generated atomic must apply");
                    if after_first.captured_snipe_winner().is_some() {
                        return true;
                    }
                    let mut seconds = Vec::new();
                    after_first.legal_atomics_into(&mut seconds);
                    if !seconds.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[inline]
    pub fn is_terminal(self) -> bool {
        self.winner().is_some()
    }

    pub fn legal_atomics(self) -> Vec<AtomicMove> {
        let mut moves = Vec::new();
        self.legal_atomics_into(&mut moves);
        moves
    }

    pub fn legal_atomics_into(self, moves: &mut Vec<AtomicMove>) {
        moves.clear();
        if self.captured_snipe_winner().is_some() {
            return;
        }

        if self.pending.is_none() {
            self.generate_snipe_steps(moves);
            self.generate_drops(moves);
        }
        self.generate_animal_steps(moves);
    }

    pub fn legal_moves(self) -> Vec<Move> {
        let mut moves = Vec::new();
        self.legal_moves_into(&mut moves);
        moves
    }

    /// Generates full-turn moves and reuses the caller's allocation.
    pub fn legal_moves_into(self, moves: &mut Vec<Move>) {
        moves.clear();
        if self.pending.is_some() || self.captured_snipe_winner().is_some() {
            return;
        }

        let mut firsts = Vec::new();
        self.legal_atomics_into(&mut firsts);
        for atomic in firsts {
            match atomic {
                AtomicMove::Snipe { destination } => {
                    moves.push(Move::Snipe { destination });
                }
                AtomicMove::Drop {
                    animal,
                    destination,
                } => {
                    moves.push(Move::Drop {
                        animal,
                        destination,
                    });
                }
                AtomicMove::Animal(first) => {
                    let after_first = self
                        .apply_atomic(AtomicMove::Animal(first))
                        .expect("generated atomic must apply");
                    let mut seconds = Vec::new();
                    after_first.legal_atomics_into(&mut seconds);
                    if after_first.captured_snipe_winner().is_some() {
                        moves.push(Move::Animals {
                            first,
                            second: None,
                        });
                    } else {
                        for second in seconds {
                            if let AtomicMove::Animal(second) = second {
                                moves.push(Move::Animals {
                                    first,
                                    second: Some(second),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn apply_move(self, mv: Move) -> Result<Self, IllegalMove> {
        match mv {
            Move::Snipe { destination } => self.apply_atomic(AtomicMove::Snipe { destination }),
            Move::Drop {
                animal,
                destination,
            } => self.apply_atomic(AtomicMove::Drop {
                animal,
                destination,
            }),
            Move::Animals { first, second } => {
                let after_first = self.apply_atomic(AtomicMove::Animal(first))?;
                match second {
                    Some(second) => {
                        if after_first.captured_snipe_winner().is_some() {
                            Err(IllegalMove::UnexpectedSecondAnimalStep)
                        } else {
                            after_first.apply_atomic(AtomicMove::Animal(second))
                        }
                    }
                    None => {
                        if after_first.captured_snipe_winner().is_some() {
                            Ok(after_first)
                        } else {
                            Err(IllegalMove::IncompleteAnimalTurn)
                        }
                    }
                }
            }
        }
    }

    pub fn apply_move_with_outcome(self, mv: Move) -> Result<MoveOutcome, IllegalMove> {
        let mover = self.side_to_move;
        let reserve = Location::reserve_of(mover);
        let before = self.cell(reserve);
        let state = self.apply_move(mv)?;
        let after = state.cell(reserve);
        Ok(MoveOutcome {
            state,
            captured_animals: after.animals(mover) & !before.animals(mover),
            captured_snipes: after.snipes() & !before.snipes(),
        })
    }

    pub fn apply_with_undo(&mut self, mv: Move) -> Result<Undo, IllegalMove> {
        let previous = *self;
        *self = self.apply_move(mv)?;
        Ok(Undo { previous })
    }

    #[inline]
    pub fn unapply(&mut self, undo: Undo) {
        *self = undo.previous;
    }

    pub fn apply_atomic(self, atomic: AtomicMove) -> Result<Self, IllegalMove> {
        self.validate_atomic(atomic)?;
        Ok(self.force_atomic(atomic))
    }

    pub fn reflected(self) -> Self {
        let mut reflected = Self::empty(self.side_to_move.opponent());
        for location in Location::ALL {
            let src = self.cell(location);
            let dst = location.reflected().index();
            reflected.cells[dst].animals[Player::Alpha.index()] = src.animals(Player::Beta);
            reflected.cells[dst].animals[Player::Beta.index()] = src.animals(Player::Alpha);
            reflected.cells[dst].snipes = ((src.snipes & 1) << 1) | ((src.snipes & 2) >> 1);
        }
        reflected.pending = self.pending.map(|step| AnimalStep {
            moved: step.moved,
            destination: step.destination.reflected(),
        });
        reflected
    }

    /// Builder helper used by tests, importers, and position editors.
    pub fn with_card(mut self, location: Location, card: Card, allegiance: Player) -> Self {
        match card {
            Card::Animal(animal) => self.put_animal(location, animal, allegiance),
            Card::Snipe(player) => self.put_snipe(location, player),
        }
        self
    }

    fn put_animal(&mut self, location: Location, animal: Animal, owner: Player) {
        self.cells[location.index()].animals[owner.index()] |= animal.bit();
    }

    fn put_snipe(&mut self, location: Location, player: Player) {
        self.cells[location.index()].snipes |= 1 << player as u8;
    }

    fn generate_snipe_steps(self, moves: &mut Vec<AtomicMove>) {
        let Some(location) = self.snipe_location(self.side_to_move) else {
            return;
        };
        let Some(row) = location.row() else {
            return;
        };
        let cell = self.cell(location);
        let has_other_card =
            cell.all_animals() != 0 || cell.has_snipe(self.side_to_move.opponent());
        if !has_other_card {
            return;
        }
        if let Some(destination) = row.forward(self.side_to_move) {
            moves.push(AtomicMove::Snipe { destination });
        }
        if let Some(destination) = row.backward(self.side_to_move) {
            moves.push(AtomicMove::Snipe { destination });
        }
    }

    fn generate_drops(self, moves: &mut Vec<AtomicMove>) {
        let reserve = Location::reserve_of(self.side_to_move);
        let animals = self.animal_bits(reserve, self.side_to_move);
        if animals.count_ones() <= 1 {
            return;
        }
        for animal in Animal::ALL {
            if animals & animal.bit() == 0 {
                continue;
            }
            for row in Row::ALL {
                if !animal.can_retreat() || legal_retreater_drop(self.side_to_move, row) {
                    moves.push(AtomicMove::Drop {
                        animal,
                        destination: row,
                    });
                }
            }
        }
    }

    fn generate_animal_steps(self, moves: &mut Vec<AtomicMove>) {
        for row in Row::ALL {
            let location = row.location();
            let cell = self.cell(location);
            let friendly = cell.animals(self.side_to_move);
            if friendly == 0 {
                continue;
            }
            for animal in Animal::ALL {
                if friendly & animal.bit() == 0
                    || self.pending.is_some_and(|step| step.moved == animal)
                {
                    continue;
                }
                if let Some(destination) = row.forward(self.side_to_move) {
                    let step = AnimalStep {
                        moved: animal,
                        destination,
                    };
                    if self.validate_animal_step(step).is_ok() {
                        moves.push(AtomicMove::Animal(step));
                    }
                }
                if animal.can_retreat() {
                    if let Some(destination) = row.backward(self.side_to_move) {
                        let step = AnimalStep {
                            moved: animal,
                            destination,
                        };
                        if self.validate_animal_step(step).is_ok() {
                            moves.push(AtomicMove::Animal(step));
                        }
                    }
                }
            }
        }
    }

    fn validate_atomic(self, atomic: AtomicMove) -> Result<(), IllegalMove> {
        if self.captured_snipe_winner().is_some() {
            return Err(IllegalMove::SnipeAlreadyCaptured);
        }
        match atomic {
            AtomicMove::Snipe { destination } => {
                if self.pending.is_some() {
                    return Err(IllegalMove::AlreadyMovedAnimal);
                }
                let location = self
                    .snipe_location(self.side_to_move)
                    .ok_or(IllegalMove::CardNotFound)?;
                let row = location
                    .row()
                    .ok_or(IllegalMove::StepDestinationOutOfRange)?;
                if row.forward(self.side_to_move) != Some(destination)
                    && row.backward(self.side_to_move) != Some(destination)
                {
                    return Err(IllegalMove::StepDestinationOutOfRange);
                }
                let cell = self.cell(location);
                if cell.all_animals() == 0 && !cell.has_snipe(self.side_to_move.opponent()) {
                    return Err(IllegalMove::CannotEmptyRowWithoutImmediatelyWinning);
                }
                Ok(())
            }
            AtomicMove::Drop {
                animal,
                destination,
            } => {
                if self.pending.is_some() {
                    return Err(IllegalMove::AlreadyMovedAnimal);
                }
                let reserve = Location::reserve_of(self.side_to_move);
                let animals = self.animal_bits(reserve, self.side_to_move);
                if animals & animal.bit() == 0 {
                    return Err(IllegalMove::DroppedAnimalNotInReserve);
                }
                if animals & !animal.bit() == 0 {
                    return Err(IllegalMove::CannotEmptyReserve);
                }
                if animal.can_retreat() && !legal_retreater_drop(self.side_to_move, destination) {
                    return Err(IllegalMove::CannotDropRetreaterOnEnemyBackTwoRows);
                }
                Ok(())
            }
            AtomicMove::Animal(step) => self.validate_animal_step(step),
        }
    }

    fn validate_animal_step(self, step: AnimalStep) -> Result<(), IllegalMove> {
        let location = self
            .location_of_animal(step.moved)
            .ok_or(IllegalMove::CardNotFound)?;
        let Some(source) = location.row() else {
            return Err(IllegalMove::MovedCardInReserve);
        };
        if self.animal_bits(location, self.side_to_move) & step.moved.bit() == 0 {
            return Err(IllegalMove::NotYourAnimal);
        }
        if source.forward(self.side_to_move) != Some(step.destination)
            && !(step.moved.can_retreat()
                && source.backward(self.side_to_move) == Some(step.destination))
        {
            return Err(IllegalMove::StepDestinationOutOfRange);
        }
        if self
            .pending
            .is_some_and(|pending| pending.moved == step.moved)
        {
            return Err(IllegalMove::CannotMoveSameAnimalTwice);
        }

        let destination = self.cell(step.destination.location());
        let activates = activates_triplet(destination.all_animals(), step.moved);
        let enemy_snipe = destination.has_snipe(self.side_to_move.opponent());
        let friendly_snipe = destination.has_snipe(self.side_to_move);
        let source_cell = self.cell(location);
        let source_has_another_card =
            source_cell.all_animals() & !step.moved.bit() != 0 || source_cell.snipes != 0;

        if source_has_another_card {
            if activates && friendly_snipe && !enemy_snipe {
                Err(IllegalMove::CannotCaptureOwnSnipeWithoutAlsoCapturingOpponent)
            } else {
                Ok(())
            }
        } else if activates && enemy_snipe {
            Ok(())
        } else {
            Err(IllegalMove::CannotEmptyRowWithoutImmediatelyWinning)
        }
    }

    fn force_atomic(mut self, atomic: AtomicMove) -> Self {
        match atomic {
            AtomicMove::Snipe { destination } => {
                let source = self
                    .snipe_location(self.side_to_move)
                    .expect("validated snipe exists");
                let bit = 1 << self.side_to_move as u8;
                self.cells[source.index()].snipes &= !bit;
                self.cells[destination.location().index()].snipes |= bit;
                self.side_to_move = self.side_to_move.opponent();
            }
            AtomicMove::Drop {
                animal,
                destination,
            } => {
                let reserve = Location::reserve_of(self.side_to_move);
                self.cells[reserve.index()].animals[self.side_to_move.index()] &= !animal.bit();
                self.cells[destination.location().index()].animals[self.side_to_move.index()] |=
                    animal.bit();
                self.side_to_move = self.side_to_move.opponent();
            }
            AtomicMove::Animal(step) => {
                let source = self
                    .location_of_animal(step.moved)
                    .expect("validated animal exists");
                let destination_index = step.destination.location().index();
                let destination_before = self.cells[destination_index];
                self.cells[source.index()].animals[self.side_to_move.index()] &= !step.moved.bit();

                if activates_triplet(destination_before.all_animals(), step.moved) {
                    let reserve = Location::reserve_of(self.side_to_move).index();
                    self.cells[reserve].animals[self.side_to_move.index()] |=
                        destination_before.all_animals();
                    self.cells[reserve].snipes |= destination_before.snipes;
                    self.cells[destination_index].animals = [0; 2];
                    self.cells[destination_index].animals[self.side_to_move.index()] =
                        step.moved.bit();
                    self.cells[destination_index].snipes = 0;
                } else {
                    self.cells[destination_index].animals[self.side_to_move.index()] |=
                        step.moved.bit();
                }

                if self.pending.is_some() {
                    self.pending = None;
                    self.side_to_move = self.side_to_move.opponent();
                } else {
                    self.pending = Some(step);
                }
            }
        }
        self
    }
}

#[inline]
pub const fn legal_retreater_drop(player: Player, row: Row) -> bool {
    match player {
        Player::Alpha => (row as u8) <= 4,
        Player::Beta => (row as u8) >= 3,
    }
}

pub fn activates_triplet(old_animals: u32, new_animal: Animal) -> bool {
    let mut counts = new_animal.element_counts();
    for animal in Animal::ALL {
        if old_animals & animal.bit() != 0 {
            counts |= animal.element_counts();
        }
    }
    let (first, second) = new_animal.triplet_shifts();
    ((counts >> first) & 0b111) == 0b111 || ((counts >> second) & 0b111) == 0b111
}

#[derive(Clone, Copy)]
struct SplitMix64(u64);

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

fn shuffle<T>(values: &mut [T], rng: &mut SplitMix64) {
    for i in (1..values.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        values.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_state(turn: Player) -> State {
        State::empty(turn)
            .with_card(Location::Row1, Card::Snipe(Player::Alpha), Player::Alpha)
            .with_card(Location::Row6, Card::Snipe(Player::Beta), Player::Beta)
            .with_card(Location::Row1, Card::Animal(Animal::Mouse1), Player::Alpha)
            .with_card(Location::Row6, Card::Animal(Animal::Mouse2), Player::Beta)
            .with_card(
                Location::AlphaReserve,
                Card::Animal(Animal::Ox1),
                Player::Alpha,
            )
            .with_card(
                Location::AlphaReserve,
                Card::Animal(Animal::Tiger1),
                Player::Alpha,
            )
            .with_card(
                Location::BetaReserve,
                Card::Animal(Animal::Ox2),
                Player::Beta,
            )
            .with_card(
                Location::BetaReserve,
                Card::Animal(Animal::Tiger2),
                Player::Beta,
            )
    }

    #[test]
    fn initial_deal_has_every_animal_once_and_correct_shape() {
        let state = State::initial(42);
        let mut seen = 0_u32;
        for location in Location::ALL {
            let animals = state.cell(location).all_animals();
            assert_eq!(seen & animals, 0, "duplicate animal at {location:?}");
            seen |= animals;
        }
        assert_eq!(seen, u32::MAX);
        assert_eq!(state.cell(Location::Row1).card_count(), 3);
        assert_eq!(state.cell(Location::Row2).card_count(), 12);
        assert_eq!(state.cell(Location::Row3).card_count(), 1);
        assert_eq!(state.cell(Location::Row4).card_count(), 1);
        assert_eq!(state.cell(Location::Row5).card_count(), 12);
        assert_eq!(state.cell(Location::Row6).card_count(), 3);
        assert_eq!(state.cell(Location::AlphaReserve).card_count(), 1);
        assert_eq!(state.cell(Location::BetaReserve).card_count(), 1);
        assert_eq!(state.side_to_move(), Player::Beta);
    }

    #[test]
    fn seeded_deals_are_reproducible_and_vary() {
        assert_eq!(State::initial(99), State::initial(99));
        assert_ne!(State::initial(99), State::initial(100));
        assert_eq!(
            State::initial(99).position_hash(),
            State::initial(99).position_hash()
        );
    }

    #[test]
    fn state_data_and_legacy_board_round_trip() {
        for seed in 0..16 {
            let state = State::initial(seed);
            assert_eq!(State::from_data(state.to_data()), Ok(state));
            assert_eq!(
                State::from_legacy_board(state.to_legacy_board(), state.side_to_move(), None),
                Ok(state)
            );
        }
    }

    #[test]
    fn state_data_rejects_duplicate_animals() {
        let mut data = State::initial(0).to_data();
        data.alpha_animals[0] |= Animal::Mouse1.bit();
        data.beta_animals[7] |= Animal::Mouse1.bit();
        assert_eq!(State::from_data(data), Err(InvalidState::DuplicateAnimal));
    }

    #[test]
    fn initial_state_has_full_turns_and_every_one_applies() {
        for seed in 0..16 {
            let state = State::initial(seed);
            let moves = state.legal_moves();
            assert!(!moves.is_empty());
            for mv in moves {
                state.apply_move(mv).unwrap();
            }
        }
    }

    #[test]
    fn drops_cannot_empty_reserve_and_retreaters_are_restricted() {
        let one = basic_state(Player::Beta);
        let drop = AtomicMove::Drop {
            animal: Animal::Ox2,
            destination: Row::One,
        };
        assert!(one.apply_atomic(drop).is_ok());

        let restricted = AtomicMove::Drop {
            animal: Animal::Mouse2,
            destination: Row::One,
        };
        assert_eq!(
            one.apply_atomic(restricted),
            Err(IllegalMove::DroppedAnimalNotInReserve)
        );

        let single = State::empty(Player::Alpha).with_card(
            Location::AlphaReserve,
            Card::Animal(Animal::Mouse1),
            Player::Alpha,
        );
        assert_eq!(
            single.apply_atomic(AtomicMove::Drop {
                animal: Animal::Mouse1,
                destination: Row::One,
            }),
            Err(IllegalMove::CannotEmptyReserve)
        );
    }

    #[test]
    fn animal_turn_requires_distinct_cards() {
        let state = basic_state(Player::Alpha);
        let first = AnimalStep {
            moved: Animal::Mouse1,
            destination: Row::Two,
        };
        let after = state.apply_atomic(AtomicMove::Animal(first)).unwrap();
        assert_eq!(after.side_to_move(), Player::Alpha);
        assert_eq!(after.pending_animal_step(), Some(first));
        assert_eq!(
            after.apply_atomic(AtomicMove::Animal(AnimalStep {
                moved: Animal::Mouse1,
                destination: Row::Three,
            })),
            Err(IllegalMove::CannotMoveSameAnimalTwice)
        );
    }

    #[test]
    fn second_animal_step_finishes_turn() {
        let state = basic_state(Player::Alpha).with_card(
            Location::Row1,
            Card::Animal(Animal::Rabbit1),
            Player::Alpha,
        );
        let mv = Move::Animals {
            first: AnimalStep {
                moved: Animal::Mouse1,
                destination: Row::Two,
            },
            second: Some(AnimalStep {
                moved: Animal::Rabbit1,
                destination: Row::Two,
            }),
        };
        let next = state.apply_move(mv).unwrap();
        assert_eq!(next.side_to_move(), Player::Beta);
        assert_eq!(next.pending_animal_step(), None);
    }

    #[test]
    fn cannot_empty_a_row_except_by_immediately_capturing_enemy_snipe() {
        let state = State::empty(Player::Alpha)
            .with_card(Location::Row1, Card::Snipe(Player::Alpha), Player::Alpha)
            .with_card(Location::Row6, Card::Snipe(Player::Beta), Player::Beta)
            .with_card(
                Location::Row3,
                Card::Animal(Animal::Rooster1),
                Player::Alpha,
            );
        let step = AtomicMove::Animal(AnimalStep {
            moved: Animal::Rooster1,
            destination: Row::Four,
        });
        assert_eq!(
            state.apply_atomic(step),
            Err(IllegalMove::CannotEmptyRowWithoutImmediatelyWinning)
        );

        // F1 (Rooster) + F2 (Mouse) + F3 (Tiger) activates a fire triplet.
        let winning = state
            .with_card(Location::Row4, Card::Animal(Animal::Mouse2), Player::Beta)
            .with_card(Location::Row4, Card::Animal(Animal::Tiger2), Player::Beta)
            .with_card(Location::Row4, Card::Snipe(Player::Beta), Player::Beta);
        let won = winning.apply_atomic(step).unwrap();
        assert_eq!(won.captured_snipe_winner(), Some(Player::Alpha));
        assert_eq!(
            won.snipe_location(Player::Beta),
            Some(Location::AlphaReserve)
        );
    }

    #[test]
    fn triplet_captures_everything_except_activator_and_changes_allegiance() {
        let state = State::empty(Player::Alpha)
            .with_card(Location::Row1, Card::Snipe(Player::Alpha), Player::Alpha)
            .with_card(Location::Row6, Card::Snipe(Player::Beta), Player::Beta)
            .with_card(
                Location::Row3,
                Card::Animal(Animal::Rooster1),
                Player::Alpha,
            )
            .with_card(Location::Row3, Card::Animal(Animal::Dog1), Player::Alpha)
            .with_card(Location::Row4, Card::Animal(Animal::Mouse2), Player::Beta)
            .with_card(Location::Row4, Card::Animal(Animal::Tiger2), Player::Beta)
            .with_card(Location::Row4, Card::Animal(Animal::Ox1), Player::Alpha);
        let next = state
            .apply_atomic(AtomicMove::Animal(AnimalStep {
                moved: Animal::Rooster1,
                destination: Row::Four,
            }))
            .unwrap();
        assert_eq!(
            next.cell(Location::Row4).all_animals(),
            Animal::Rooster1.bit()
        );
        for animal in [Animal::Mouse2, Animal::Tiger2, Animal::Ox1] {
            assert_eq!(next.owner_of_animal(animal), Some(Player::Alpha));
            assert_eq!(
                next.location_of_animal(animal),
                Some(Location::AlphaReserve)
            );
        }
    }

    #[test]
    fn cannot_triplet_capture_only_own_snipe() {
        let state = State::empty(Player::Alpha)
            .with_card(Location::Row1, Card::Snipe(Player::Beta), Player::Beta)
            .with_card(
                Location::Row3,
                Card::Animal(Animal::Rooster1),
                Player::Alpha,
            )
            .with_card(Location::Row3, Card::Animal(Animal::Dog1), Player::Alpha)
            .with_card(Location::Row4, Card::Animal(Animal::Mouse2), Player::Beta)
            .with_card(Location::Row4, Card::Animal(Animal::Tiger2), Player::Beta)
            .with_card(Location::Row4, Card::Snipe(Player::Alpha), Player::Alpha);
        assert_eq!(
            state.apply_atomic(AtomicMove::Animal(AnimalStep {
                moved: Animal::Rooster1,
                destination: Row::Four,
            })),
            Err(IllegalMove::CannotCaptureOwnSnipeWithoutAlsoCapturingOpponent)
        );
    }

    #[test]
    fn snipe_cannot_leave_a_row_empty() {
        let state = State::empty(Player::Alpha)
            .with_card(Location::Row2, Card::Snipe(Player::Alpha), Player::Alpha)
            .with_card(Location::Row6, Card::Snipe(Player::Beta), Player::Beta);
        assert_eq!(
            state.apply_atomic(AtomicMove::Snipe {
                destination: Row::Three,
            }),
            Err(IllegalMove::CannotEmptyRowWithoutImmediatelyWinning)
        );
    }

    #[test]
    fn terminal_first_animal_step_is_a_complete_move() {
        let state = State::empty(Player::Alpha)
            .with_card(Location::Row1, Card::Snipe(Player::Alpha), Player::Alpha)
            .with_card(
                Location::Row3,
                Card::Animal(Animal::Rooster1),
                Player::Alpha,
            )
            .with_card(Location::Row4, Card::Animal(Animal::Mouse2), Player::Beta)
            .with_card(Location::Row4, Card::Animal(Animal::Tiger2), Player::Beta)
            .with_card(Location::Row4, Card::Snipe(Player::Beta), Player::Beta);
        let winning = Move::Animals {
            first: AnimalStep {
                moved: Animal::Rooster1,
                destination: Row::Four,
            },
            second: None,
        };
        assert!(state.legal_moves().contains(&winning));
        assert_eq!(
            state.apply_move(winning).unwrap().winner(),
            Some(Player::Alpha)
        );
    }

    #[test]
    fn nonterminal_first_step_without_a_second_is_not_a_full_move() {
        let state = State::empty(Player::Alpha)
            .with_card(Location::Row1, Card::Snipe(Player::Alpha), Player::Alpha)
            .with_card(Location::Row1, Card::Animal(Animal::Mouse1), Player::Alpha)
            .with_card(Location::Row6, Card::Snipe(Player::Beta), Player::Beta);
        let incomplete = Move::Animals {
            first: AnimalStep {
                moved: Animal::Mouse1,
                destination: Row::Two,
            },
            second: None,
        };
        assert!(!state.legal_moves().contains(&incomplete));
        assert_eq!(
            state.apply_move(incomplete),
            Err(IllegalMove::IncompleteAnimalTurn)
        );
    }

    #[test]
    fn reflection_is_an_involution_and_maps_legal_moves() {
        for seed in 0..12 {
            let state = State::initial(seed);
            let reflected = state.reflected();
            assert_eq!(reflected.reflected(), state);
            let reflected_moves = reflected.legal_moves();
            for mv in state.legal_moves() {
                assert!(reflected_moves.contains(&mv.reflected()), "{mv:?}");
                assert_eq!(
                    state.apply_move(mv).unwrap().reflected(),
                    reflected.apply_move(mv.reflected()).unwrap()
                );
            }
        }
    }

    #[test]
    fn undo_restores_state_exactly() {
        let mut state = State::initial(123);
        let original = state;
        let mv = state.legal_moves()[0];
        let undo = state.apply_with_undo(mv).unwrap();
        assert_ne!(state, original);
        state.unapply(undo);
        assert_eq!(state, original);
    }

    fn perft(state: State, depth: u8) -> u64 {
        if depth == 0 || state.is_terminal() {
            return 1;
        }
        state
            .legal_moves()
            .into_iter()
            .map(|mv| perft(state.apply_move(mv).unwrap(), depth - 1))
            .sum()
    }

    #[test]
    fn reference_perft_seed_zero() {
        let state = State::initial(0);
        assert_eq!(perft(state, 1), 279);
        assert_eq!(perft(state, 2), 81_525);
    }
}
