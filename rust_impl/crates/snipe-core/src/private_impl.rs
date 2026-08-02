//! This module contains the implementation.
//! It does NOT declare any `pub` items (only `pub(crate)`).

use super::*;

impl State {
    // We manually implement this because the public representation combines the
    // reserves, while Debug should display Alpha's above r1 and Beta's below r6.
    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("active_player", &self.active_player)
            .field("alpha_reserve", &self.reserves.for_reserve(Player::Alpha))
            .field("r1", &self.r1)
            .field("r2", &self.r2)
            .field("r3", &self.r3)
            .field("r4", &self.r4)
            .field("r5", &self.r5)
            .field("r6", &self.r6)
            .field("beta_reserve", &self.reserves.for_reserve(Player::Beta))
            .field("leading_action", &self.leading_action)
            .finish()
    }

    pub(crate) fn write_legal_actions_<W>(&self, w: &mut W)
    where
        W: ActionWriter,
    {
        if self.captured_snipe_winner().is_some() {
            return;
        }

        w.reserve(290);

        if self.leading_action.is_none() {
            self.write_legal_snipe_steps(w);
            self.write_legal_drops(w);
        }
        self.write_legal_animal_steps(w);
    }

    pub(crate) fn apply_(mut self, action: Action) -> Result<Self, IllegalActionError> {
        if self.captured_snipe_winner().is_some() {
            return Err(IllegalActionError::SnipeAlreadyCaptured);
        }
        let active_player = self.active_player;

        match action {
            Action::SnipeStep(step) => {
                self.validate_snipe_step(step)?;
                let source = self
                    .snipe_rank(self.active_player)
                    .expect("validated snipe step has a source");
                let source_cards = self.rank_mut(source);
                *source_cards = source_cards
                    .remove_one(Card::Snipe, active_player)
                    .expect("validated snipe is present");
                let destination = self.rank_mut(step.destination);
                *destination = destination
                    .checked_add(CardMultiset::singleton(Card::Snipe, active_player))
                    .expect("a snipe cannot be duplicated in a valid state");
                self.active_player = self.active_player.opponent();
            }
            Action::Drop(drop) => {
                self.validate_drop(drop)?;
                self.reserves = self
                    .reserves
                    .remove_one(Card::Animal(drop.actor), active_player)
                    .expect("validated dropped animal is present");
                let destination = self.rank_mut(drop.destination);
                *destination = destination
                    .checked_add(CardMultiset::singleton(
                        Card::Animal(drop.actor),
                        active_player,
                    ))
                    .expect("a dropped animal cannot be duplicated in a valid state");
                self.active_player = self.active_player.opponent();
            }
            Action::AnimalStep(step) => {
                let source = self.validate_animal_step(step)?;
                let destination_before = *self.rank(step.destination);
                let activates = step
                    .actor
                    .would_activate_triplet_by_entering(destination_before);

                let source_cards = self.rank_mut(source);
                *source_cards = source_cards
                    .remove_one(Card::Animal(step.actor), active_player)
                    .expect("validated moved animal is present");

                if activates {
                    self.capture_into_reserve(destination_before);
                    *self.rank_mut(step.destination) =
                        CardMultiset::singleton(Card::Animal(step.actor), active_player);
                } else {
                    let destination = self.rank_mut(step.destination);
                    *destination = destination
                        .checked_add(CardMultiset::singleton(
                            Card::Animal(step.actor),
                            active_player,
                        ))
                        .expect("a moved animal cannot be duplicated in a valid state");
                }

                if self.leading_action.is_some() {
                    self.leading_action = None;
                    self.active_player = self.active_player.opponent();
                } else {
                    self.leading_action = Some(step);
                }
            }
        }

        Ok(self)
    }

    pub(crate) fn winner_(&self) -> Option<Player> {
        if let Some(winner) = self.captured_snipe_winner() {
            return Some(winner);
        }

        let mut actions = Vec::new();
        self.write_legal_actions_(&mut actions);
        if actions.is_empty() {
            Some(self.active_player.opponent())
        } else {
            None
        }
    }
}

impl InitialStateBuilder {
    pub(crate) const fn build_(self, validate_major_balance: bool) -> Option<State> {
        if validate_major_balance
            && (count_major_animals(&self.alpha_reserve)
                + count_major_animals(&self.r1)
                + count_major_animals(&self.r2)
                + count_major_animals(&self.r3)
                != 4
                || count_major_animals(&self.r4)
                    + count_major_animals(&self.r5)
                    + count_major_animals(&self.r6)
                    + count_major_animals(&self.beta_reserve)
                    != 4)
        {
            return None;
        }

        let mut reserves = CardMultiset::EMPTY;
        let mut r1 = CardMultiset::singleton(Card::Snipe, Player::Alpha);
        let mut r2 = CardMultiset::EMPTY;
        let mut r3 = CardMultiset::EMPTY;
        let mut r4 = CardMultiset::EMPTY;
        let mut r5 = CardMultiset::EMPTY;
        let mut r6 = CardMultiset::singleton(Card::Snipe, Player::Beta);

        reserves = match add_animals(reserves, &self.alpha_reserve, Player::Alpha) {
            Some(cards) => cards,
            None => return None,
        };
        r1 = match add_animals(r1, &self.r1, Player::Alpha) {
            Some(cards) => cards,
            None => return None,
        };
        r2 = match add_animals(r2, &self.r2, Player::Alpha) {
            Some(cards) => cards,
            None => return None,
        };
        r3 = match add_animals(r3, &self.r3, Player::Alpha) {
            Some(cards) => cards,
            None => return None,
        };
        r4 = match add_animals(r4, &self.r4, Player::Beta) {
            Some(cards) => cards,
            None => return None,
        };
        r5 = match add_animals(r5, &self.r5, Player::Beta) {
            Some(cards) => cards,
            None => return None,
        };
        r6 = match add_animals(r6, &self.r6, Player::Beta) {
            Some(cards) => cards,
            None => return None,
        };
        reserves = match add_animals(reserves, &self.beta_reserve, Player::Beta) {
            Some(cards) => cards,
            None => return None,
        };

        let all = match reserves.checked_add(r1) {
            Some(cards) => cards,
            None => return None,
        };
        let all = match all.checked_add(r2) {
            Some(cards) => cards,
            None => return None,
        };
        let all = match all.checked_add(r3) {
            Some(cards) => cards,
            None => return None,
        };
        let all = match all.checked_add(r4) {
            Some(cards) => cards,
            None => return None,
        };
        let all = match all.checked_add(r5) {
            Some(cards) => cards,
            None => return None,
        };
        let all = match all.checked_add(r6) {
            Some(cards) => cards,
            None => return None,
        };

        let mut animal_index = 0;
        while animal_index < 16 {
            let animal = animal_from_index(animal_index);
            if all.count(Card::Animal(animal), Player::Alpha)
                + all.count(Card::Animal(animal), Player::Beta)
                != 2
            {
                return None;
            }
            animal_index += 1;
        }

        Some(State {
            active_player: Player::Beta,
            reserves,
            r1,
            r2,
            r3,
            r4,
            r5,
            r6,
            leading_action: None,
        })
    }
}

const fn count_major_animals(animals: &[Animal]) -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < animals.len() {
        if animals[index].ternary_element().is_some() {
            count += 1;
        }
        index += 1;
    }
    count
}

impl CardMultiset {
    // We manually implement this so Debug shows the actual cards rather than
    // the private bit-vector representation.
    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        let mut animal_index = 0;
        while animal_index < 16 {
            let animal = animal_from_index(animal_index);
            let mut alpha_count = self.count(Card::Animal(animal), Player::Alpha);
            while alpha_count > 0 {
                list.entry(&(Card::Animal(animal), Player::Alpha));
                alpha_count -= 1;
            }
            let mut beta_count = self.count(Card::Animal(animal), Player::Beta);
            while beta_count > 0 {
                list.entry(&(Card::Animal(animal), Player::Beta));
                beta_count -= 1;
            }
            animal_index += 1;
        }
        if self.count(Card::Snipe, Player::Alpha) != 0 {
            list.entry(&(Card::Snipe, Player::Alpha));
        }
        if self.count(Card::Snipe, Player::Beta) != 0 {
            list.entry(&(Card::Snipe, Player::Beta));
        }
        list.finish()
    }

    pub(crate) const fn singleton_(card: Card, allegiance: Player) -> Self {
        match card {
            Card::Animal(animal) => {
                let bit = animal_bit(animal);
                match allegiance {
                    Player::Alpha => Self {
                        alpha_presence: bit,
                        beta_presence: 0,
                        has_allied_twins: 0,
                        snipes: 0,
                    },
                    Player::Beta => Self {
                        alpha_presence: 0,
                        beta_presence: bit,
                        has_allied_twins: 0,
                        snipes: 0,
                    },
                }
            }
            Card::Snipe => Self {
                alpha_presence: 0,
                beta_presence: 0,
                has_allied_twins: 0,
                snipes: match allegiance {
                    Player::Alpha => 0b01,
                    Player::Beta => 0b10,
                },
            },
        }
    }

    pub(crate) const fn count_(self, card: Card, allegiance: Player) -> u8 {
        match card {
            Card::Snipe => {
                let bit = match allegiance {
                    Player::Alpha => 0b01,
                    Player::Beta => 0b10,
                };
                ((self.snipes & bit) != 0) as u8
            }
            Card::Animal(animal) => {
                let bit = animal_bit(animal);
                let presence = match allegiance {
                    Player::Alpha => self.alpha_presence,
                    Player::Beta => self.beta_presence,
                };
                if presence & bit == 0 {
                    0
                } else if self.has_allied_twins & bit != 0 {
                    2
                } else {
                    1
                }
            }
        }
    }

    /// Returns `None` if the sum would have an illegal number of any of the card types.
    /// There can be at most 2 of any animal (regardless of allegiance), at most 1 Alpha Snipe, and at most 1 Beta Snipe.
    pub(crate) const fn checked_add_(self, other: Self) -> Option<Self> {
        if self.snipes & other.snipes & 0b11 != 0 {
            return None;
        }

        let mut result = Self {
            alpha_presence: 0,
            beta_presence: 0,
            has_allied_twins: 0,
            snipes: (self.snipes | other.snipes) & 0b11,
        };
        let mut animal_index = 0;
        while animal_index < 16 {
            let animal = animal_from_index(animal_index);
            let bit = animal_bit(animal);
            let alpha_count = self.count(Card::Animal(animal), Player::Alpha)
                + other.count(Card::Animal(animal), Player::Alpha);
            let beta_count = self.count(Card::Animal(animal), Player::Beta)
                + other.count(Card::Animal(animal), Player::Beta);
            if alpha_count + beta_count > 2 {
                return None;
            }
            if alpha_count != 0 {
                result.alpha_presence |= bit;
            }
            if beta_count != 0 {
                result.beta_presence |= bit;
            }
            if alpha_count == 2 || beta_count == 2 {
                result.has_allied_twins |= bit;
            }
            animal_index += 1;
        }
        Some(result)
    }

    /// Returns `None` if the card (of that allegiance) is not present
    pub(crate) const fn remove_one_(self, card: Card, allegiance: Player) -> Option<Self> {
        if self.count(card, allegiance) == 0 {
            return None;
        }

        let mut result = self;
        match card {
            Card::Snipe => {
                result.snipes &= match allegiance {
                    Player::Alpha => !0b01,
                    Player::Beta => !0b10,
                };
            }
            Card::Animal(animal) => {
                let bit = animal_bit(animal);
                if result.has_allied_twins & bit != 0 {
                    result.has_allied_twins &= !bit;
                } else {
                    match allegiance {
                        Player::Alpha => result.alpha_presence &= !bit,
                        Player::Beta => result.beta_presence &= !bit,
                    }
                }
            }
        }
        Some(result)
    }
}

impl Animal {
    pub(crate) const fn is_retreater_(self) -> bool {
        matches!(
            self,
            Self::Mouse | Self::Rabbit | Self::Snake | Self::Ram | Self::Boar | Self::Squid
        )
    }

    pub(crate) const fn unary_element_(self) -> Option<Element> {
        Some(match self {
            Self::Mouse | Self::Snake | Self::Monkey => Element::Earth,
            Self::Ox | Self::Rabbit | Self::Dog => Element::Water,
            Self::Horse | Self::Ram | Self::Frog => Element::Air,
            Self::Rooster | Self::Boar | Self::Squid => Element::Fire,
            Self::Tiger | Self::Dragon | Self::Fish | Self::Elephant => return None,
        })
    }

    pub(crate) const fn binary_element_(self) -> Option<Element> {
        Some(match self {
            Self::Mouse | Self::Horse | Self::Dog => Element::Fire,
            Self::Ox | Self::Ram | Self::Boar => Element::Earth,
            Self::Rabbit | Self::Monkey | Self::Rooster => Element::Air,
            Self::Snake | Self::Squid | Self::Frog => Element::Water,
            Self::Tiger | Self::Dragon | Self::Fish | Self::Elephant => return None,
        })
    }

    pub(crate) const fn ternary_element_(self) -> Option<Element> {
        Some(match self {
            Self::Tiger => Element::Fire,
            Self::Dragon => Element::Air,
            Self::Fish => Element::Water,
            Self::Elephant => Element::Earth,
            _ => return None,
        })
    }

    pub(crate) const fn would_activate_triplet_by_entering_(
        self,
        destination: CardMultiset,
    ) -> bool {
        let elements = [Element::Fire, Element::Water, Element::Earth, Element::Air];
        let mut element_index = 0;
        while element_index < elements.len() {
            let element = elements[element_index];
            let self_has_element = option_is(self.unary_element(), element)
                || option_is(self.binary_element(), element)
                || option_is(self.ternary_element(), element);
            if self_has_element {
                let mut has_unary = option_is(self.unary_element(), element);
                let mut has_binary = option_is(self.binary_element(), element);
                let mut has_ternary = option_is(self.ternary_element(), element);
                let mut animal_index = 0;
                while animal_index < 16 {
                    let animal = animal_from_index(animal_index);
                    let present = destination.count(Card::Animal(animal), Player::Alpha)
                        + destination.count(Card::Animal(animal), Player::Beta)
                        != 0;
                    if present {
                        has_unary |= option_is(animal.unary_element(), element);
                        has_binary |= option_is(animal.binary_element(), element);
                        has_ternary |= option_is(animal.ternary_element(), element);
                    }
                    animal_index += 1;
                }
                if has_unary && has_binary && has_ternary {
                    return true;
                }
            }
            element_index += 1;
        }
        false
    }
}

impl State {
    fn rank(&self, rank: Rank) -> &CardMultiset {
        match rank {
            Rank::R1 => &self.r1,
            Rank::R2 => &self.r2,
            Rank::R3 => &self.r3,
            Rank::R4 => &self.r4,
            Rank::R5 => &self.r5,
            Rank::R6 => &self.r6,
        }
    }

    fn rank_mut(&mut self, rank: Rank) -> &mut CardMultiset {
        match rank {
            Rank::R1 => &mut self.r1,
            Rank::R2 => &mut self.r2,
            Rank::R3 => &mut self.r3,
            Rank::R4 => &mut self.r4,
            Rank::R5 => &mut self.r5,
            Rank::R6 => &mut self.r6,
        }
    }

    fn captured_snipe_winner(&self) -> Option<Player> {
        if self.reserves.count(Card::Snipe, Player::Beta) != 0 {
            Some(Player::Alpha)
        } else if self.reserves.count(Card::Snipe, Player::Alpha) != 0 {
            Some(Player::Beta)
        } else {
            None
        }
    }

    fn snipe_rank(&self, player: Player) -> Option<Rank> {
        let ranks = all_ranks();
        let mut index = 0;
        while index < ranks.len() {
            let rank = ranks[index];
            if self.rank(rank).count(Card::Snipe, player) != 0 {
                return Some(rank);
            }
            index += 1;
        }
        None
    }

    fn write_legal_snipe_steps<W: ActionWriter>(&self, w: &mut W) {
        let Some(source) = self.snipe_rank(self.active_player) else {
            return;
        };
        if self.rank(source).card_count() <= 1 {
            return;
        }

        if let Some(destination) = advance_destination(source, self.active_player) {
            w.push(Action::SnipeStep(SnipeStep { destination }));
        }
        if let Some(destination) = retreat_destination(source, self.active_player) {
            w.push(Action::SnipeStep(SnipeStep { destination }));
        }
    }

    fn write_legal_drops<W: ActionWriter>(&self, w: &mut W) {
        if self.reserves.animal_count(self.active_player) <= 1 {
            return;
        }

        let ranks = all_ranks();
        let mut animal_index = 0;
        while animal_index < 16 {
            let animal = animal_from_index(animal_index);
            if self
                .reserves
                .count(Card::Animal(animal), self.active_player)
                != 0
            {
                let mut rank_index = 0;
                while rank_index < ranks.len() {
                    let destination = ranks[rank_index];
                    if !animal.is_retreater()
                        || legal_retreater_drop(self.active_player, destination)
                    {
                        w.push(Action::Drop(AnimalDrop {
                            actor: animal,
                            destination,
                        }));
                    }
                    rank_index += 1;
                }
            }
            animal_index += 1;
        }
    }

    fn write_legal_animal_steps<W: ActionWriter>(&self, w: &mut W) {
        let ranks = all_ranks();
        let mut rank_index = 0;
        while rank_index < ranks.len() {
            let source = ranks[rank_index];
            let mut animal_index = 0;
            while animal_index < 16 {
                let animal = animal_from_index(animal_index);
                if self
                    .rank(source)
                    .count(Card::Animal(animal), self.active_player)
                    != 0
                {
                    if let Some(destination) = advance_destination(source, self.active_player) {
                        let step = AnimalStep {
                            actor: animal,
                            direction: StepDirection::Advance,
                            destination,
                        };
                        if self.validate_animal_step(step).is_ok() {
                            w.push(Action::AnimalStep(step));
                        }
                    }
                    if animal.is_retreater() {
                        if let Some(destination) = retreat_destination(source, self.active_player) {
                            let step = AnimalStep {
                                actor: animal,
                                direction: StepDirection::Retreat,
                                destination,
                            };
                            if self.validate_animal_step(step).is_ok() {
                                w.push(Action::AnimalStep(step));
                            }
                        }
                    }
                }
                animal_index += 1;
            }
            rank_index += 1;
        }
    }

    fn validate_snipe_step(&self, step: SnipeStep) -> Result<(), IllegalActionError> {
        if self.leading_action.is_some() {
            return Err(IllegalActionError::AlreadyMovedAnimal);
        }
        let source = self
            .snipe_rank(self.active_player)
            .ok_or(IllegalActionError::CardNotFound)?;
        if advance_destination(source, self.active_player) != Some(step.destination)
            && retreat_destination(source, self.active_player) != Some(step.destination)
        {
            return Err(IllegalActionError::StepDestinationOutOfRange);
        }
        if self.rank(source).card_count() <= 1 {
            return Err(IllegalActionError::CannotEmptyRowWithoutImmediatelyWinning);
        }
        Ok(())
    }

    fn validate_drop(&self, drop: AnimalDrop) -> Result<(), IllegalActionError> {
        if self.leading_action.is_some() {
            return Err(IllegalActionError::AlreadyMovedAnimal);
        }
        let reserve_count = self
            .reserves
            .count(Card::Animal(drop.actor), self.active_player);
        if reserve_count == 0 {
            return Err(IllegalActionError::DroppedAnimalNotInReserve);
        }
        if self.reserves.animal_count(self.active_player) <= 1 {
            return Err(IllegalActionError::CannotEmptyReserve);
        }
        if drop.actor.is_retreater() && !legal_retreater_drop(self.active_player, drop.destination)
        {
            return Err(IllegalActionError::CannotDropRetreaterOnEnemyBackTwoRanks);
        }
        Ok(())
    }

    fn validate_animal_step(&self, step: AnimalStep) -> Result<Rank, IllegalActionError> {
        if step.direction == StepDirection::Retreat && !step.actor.is_retreater() {
            return Err(IllegalActionError::StepDestinationOutOfRange);
        }
        let Some(source) =
            source_for_destination(step.destination, self.active_player, step.direction)
        else {
            return Err(IllegalActionError::StepDestinationOutOfRange);
        };

        let friendly_count = self
            .rank(source)
            .count(Card::Animal(step.actor), self.active_player);
        if friendly_count == 0 {
            if self
                .reserves
                .count(Card::Animal(step.actor), self.active_player)
                != 0
            {
                return Err(IllegalActionError::MovedCardInReserve);
            }
            if self
                .rank(source)
                .count(Card::Animal(step.actor), self.active_player.opponent())
                != 0
            {
                return Err(IllegalActionError::NotYourAnimal);
            }
            if self.animal_is_on_a_rank(step.actor, self.active_player) {
                return Err(IllegalActionError::StepDestinationOutOfRange);
            }
            return Err(IllegalActionError::CardNotFound);
        }

        if let Some(leading) = self.leading_action {
            // The two physical cards of one animal are deliberately represented
            // as a multiplicity. If both are here, the player can choose the one
            // that did not move first; otherwise this would move the same card.
            if leading.actor == step.actor && leading.destination == source && friendly_count < 2 {
                return Err(IllegalActionError::CannotMoveSameAnimalTwice);
            }
        }

        let destination = *self.rank(step.destination);
        let activates = step.actor.would_activate_triplet_by_entering(destination);
        let enemy_snipe = destination.count(Card::Snipe, self.active_player.opponent()) != 0;
        let friendly_snipe = destination.count(Card::Snipe, self.active_player) != 0;

        if self.rank(source).card_count() <= 1 {
            if !activates || !enemy_snipe {
                return Err(IllegalActionError::CannotEmptyRowWithoutImmediatelyWinning);
            }
        } else if activates && friendly_snipe && !enemy_snipe {
            return Err(IllegalActionError::CannotCaptureOwnSnipeWithoutAlsoCapturingOpponent);
        }

        Ok(source)
    }

    fn animal_is_on_a_rank(&self, animal: Animal, player: Player) -> bool {
        let ranks = all_ranks();
        let mut index = 0;
        while index < ranks.len() {
            if self.rank(ranks[index]).count(Card::Animal(animal), player) != 0 {
                return true;
            }
            index += 1;
        }
        false
    }

    fn capture_into_reserve(&mut self, captured: CardMultiset) {
        let mut animal_index = 0;
        while animal_index < 16 {
            let animal = animal_from_index(animal_index);
            let mut count = captured.count(Card::Animal(animal), Player::Alpha)
                + captured.count(Card::Animal(animal), Player::Beta);
            while count > 0 {
                self.reserves = self
                    .reserves
                    .checked_add(CardMultiset::singleton(
                        Card::Animal(animal),
                        self.active_player,
                    ))
                    .expect("capturing preserves the two-copy animal invariant");
                count -= 1;
            }
            animal_index += 1;
        }
        for snipe in [Player::Alpha, Player::Beta] {
            if captured.count(Card::Snipe, snipe) != 0 {
                self.reserves = self
                    .reserves
                    .checked_add(CardMultiset::singleton(Card::Snipe, snipe))
                    .expect("capturing cannot duplicate a snipe in a valid state");
            }
        }
    }
}

impl CardMultiset {
    fn card_count(self) -> u32 {
        let mut count = (self.snipes & 0b11).count_ones();
        let mut animal_index = 0;
        while animal_index < 16 {
            let animal = animal_from_index(animal_index);
            count += u32::from(self.count(Card::Animal(animal), Player::Alpha));
            count += u32::from(self.count(Card::Animal(animal), Player::Beta));
            animal_index += 1;
        }
        count
    }

    fn animal_count(self, player: Player) -> u32 {
        let mut count = 0;
        let mut animal_index = 0;
        while animal_index < 16 {
            count += u32::from(self.count(Card::Animal(animal_from_index(animal_index)), player));
            animal_index += 1;
        }
        count
    }

    fn for_reserve(self, player: Player) -> Self {
        match player {
            Player::Alpha => Self {
                alpha_presence: self.alpha_presence,
                beta_presence: 0,
                has_allied_twins: self.has_allied_twins & self.alpha_presence,
                snipes: self.snipes & 0b10,
            },
            Player::Beta => Self {
                alpha_presence: 0,
                beta_presence: self.beta_presence,
                has_allied_twins: self.has_allied_twins & self.beta_presence,
                snipes: self.snipes & 0b01,
            },
        }
    }
}

const fn add_animals<const N: usize>(
    mut cards: CardMultiset,
    animals: &[Animal; N],
    allegiance: Player,
) -> Option<CardMultiset> {
    let mut index = 0;
    while index < N {
        cards = match cards.checked_add(CardMultiset::singleton(
            Card::Animal(animals[index]),
            allegiance,
        )) {
            Some(cards) => cards,
            None => return None,
        };
        index += 1;
    }
    Some(cards)
}

const fn animal_bit(animal: Animal) -> u16 {
    1 << animal as u16
}

const fn animal_from_index(index: usize) -> Animal {
    match index {
        0 => Animal::Mouse,
        1 => Animal::Ox,
        2 => Animal::Tiger,
        3 => Animal::Rabbit,
        4 => Animal::Dragon,
        5 => Animal::Snake,
        6 => Animal::Horse,
        7 => Animal::Ram,
        8 => Animal::Monkey,
        9 => Animal::Rooster,
        10 => Animal::Dog,
        11 => Animal::Boar,
        12 => Animal::Fish,
        13 => Animal::Elephant,
        14 => Animal::Squid,
        15 => Animal::Frog,
        _ => panic!("animal index out of range"),
    }
}

const fn all_ranks() -> [Rank; 6] {
    [Rank::R1, Rank::R2, Rank::R3, Rank::R4, Rank::R5, Rank::R6]
}

const fn next_rank(rank: Rank) -> Option<Rank> {
    match rank {
        Rank::R1 => Some(Rank::R2),
        Rank::R2 => Some(Rank::R3),
        Rank::R3 => Some(Rank::R4),
        Rank::R4 => Some(Rank::R5),
        Rank::R5 => Some(Rank::R6),
        Rank::R6 => None,
    }
}

const fn previous_rank(rank: Rank) -> Option<Rank> {
    match rank {
        Rank::R1 => None,
        Rank::R2 => Some(Rank::R1),
        Rank::R3 => Some(Rank::R2),
        Rank::R4 => Some(Rank::R3),
        Rank::R5 => Some(Rank::R4),
        Rank::R6 => Some(Rank::R5),
    }
}

const fn advance_destination(source: Rank, player: Player) -> Option<Rank> {
    match player {
        Player::Alpha => next_rank(source),
        Player::Beta => previous_rank(source),
    }
}

const fn retreat_destination(source: Rank, player: Player) -> Option<Rank> {
    match player {
        Player::Alpha => previous_rank(source),
        Player::Beta => next_rank(source),
    }
}

const fn source_for_destination(
    destination: Rank,
    player: Player,
    direction: StepDirection,
) -> Option<Rank> {
    match direction {
        StepDirection::Advance => match player {
            Player::Alpha => previous_rank(destination),
            Player::Beta => next_rank(destination),
        },
        StepDirection::Retreat => match player {
            Player::Alpha => next_rank(destination),
            Player::Beta => previous_rank(destination),
        },
    }
}

const fn legal_retreater_drop(player: Player, destination: Rank) -> bool {
    match player {
        Player::Alpha => matches!(destination, Rank::R1 | Rank::R2 | Rank::R3 | Rank::R4),
        Player::Beta => matches!(destination, Rank::R3 | Rank::R4 | Rank::R5 | Rank::R6),
    }
}

const fn option_is(value: Option<Element>, target: Element) -> bool {
    matches!(
        (value, target),
        (Some(Element::Fire), Element::Fire)
            | (Some(Element::Water), Element::Water)
            | (Some(Element::Earth), Element::Earth)
            | (Some(Element::Air), Element::Air)
    )
}

impl Evaluation {
    pub(crate) fn cmp_(&self, other: &Self) -> Ordering {
        self.compress().cmp(&other.compress())
    }

    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MateInN(evaluation) => evaluation.fmt(f),
            Self::Estimate(evaluation) => evaluation.fmt(f),
        }
    }

    pub(crate) const fn compress_(self) -> CompressedEvaluation {
        match self {
            Self::MateInN(evaluation) => evaluation.compress(),
            Self::Estimate(evaluation) => evaluation.compress(),
        }
    }
}

impl OptimalOutcome {
    pub(crate) const fn as_evaluation_(self) -> Evaluation {
        match self {
            Self::Draw => Evaluation::Estimate(EvaluationEstimate::ZERO),
            Self::MateInN(mate) => Evaluation::MateInN(mate),
        }
    }

    pub(crate) fn cmp_(&self, other: &Self) -> Ordering {
        self.as_evaluation().cmp(&other.as_evaluation())
    }
}

impl MateInN {
    pub(crate) fn cmp_(&self, other: &Self) -> Ordering {
        self.compress().cmp(&other.compress())
    }

    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = match self.winner {
            Player::Alpha => '+',
            Player::Beta => '-',
        };
        write!(f, "{sign}#{}", self.plies)
    }

    pub(crate) const fn compress_(self) -> CompressedEvaluation {
        let plies = self.plies as i32;
        let raw = match self.winner {
            Player::Alpha => 2_000_000 - plies,
            Player::Beta => -2_000_000 + plies,
        };
        CompressedEvaluation { raw }
    }
}

impl EvaluationEstimate {
    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let precision = f.precision().unwrap_or(3);
        let sign = if self.millipoints < 0 { '-' } else { '+' };
        let magnitude = self.millipoints.unsigned_abs();

        if precision >= 3 {
            write!(f, "{sign}{}.{:03}", magnitude / 1_000, magnitude % 1_000)?;
            for _ in 3..precision {
                f.write_str("0")?;
            }
            return Ok(());
        }

        let discarded_digits = 3 - precision;
        let quantum = 10_u32.pow(discarded_digits as u32);
        let rounded = (magnitude + quantum / 2) / quantum;
        if precision == 0 {
            return write!(f, "{sign}{rounded}");
        }

        let fractional_scale = 10_u32.pow(precision as u32);
        write!(
            f,
            "{sign}{}.{:0width$}",
            rounded / fractional_scale,
            rounded % fractional_scale,
            width = precision
        )
    }

    pub(crate) const fn compress_(self) -> CompressedEvaluation {
        CompressedEvaluation {
            raw: self.millipoints,
        }
    }
}

impl CompressedEvaluation {
    pub(crate) fn fmt_(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.decompress().fmt(f)
    }

    pub(crate) const fn decompress_(self) -> Evaluation {
        if self.raw >= 1_000_000 {
            return Evaluation::MateInN(MateInN {
                winner: Player::Alpha,
                plies: (2_000_000 - self.raw) as u32,
            });
        }
        if self.raw <= -1_000_000 {
            return Evaluation::MateInN(MateInN {
                winner: Player::Beta,
                plies: (2_000_000 + self.raw) as u32,
            });
        }
        Evaluation::Estimate(EvaluationEstimate {
            millipoints: self.raw,
        })
    }
}

impl ActionWriter for Vec<Action> {
    fn push(&mut self, action: Action) {
        Vec::push(self, action);
    }

    fn reserve(&mut self, additional: usize) {
        Vec::reserve(self, additional);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cards(entries: &[(Card, Player)]) -> CardMultiset {
        let mut cards = CardMultiset::EMPTY;
        for &(card, player) in entries {
            cards = cards
                .checked_add(CardMultiset::singleton(card, player))
                .unwrap();
        }
        cards
    }

    fn empty_state(active_player: Player) -> State {
        State {
            active_player,
            reserves: CardMultiset::EMPTY,
            r1: CardMultiset::EMPTY,
            r2: CardMultiset::EMPTY,
            r3: CardMultiset::EMPTY,
            r4: CardMultiset::EMPTY,
            r5: CardMultiset::EMPTY,
            r6: CardMultiset::EMPTY,
            leading_action: None,
        }
    }

    fn valid_initial_state_builder() -> InitialStateBuilder {
        InitialStateBuilder {
            alpha_reserve: [Animal::Mouse],
            r1: [Animal::Mouse, Animal::Ox],
            r2: [
                Animal::Ox,
                Animal::Tiger,
                Animal::Tiger,
                Animal::Rabbit,
                Animal::Rabbit,
                Animal::Dragon,
                Animal::Dragon,
                Animal::Snake,
                Animal::Snake,
                Animal::Horse,
                Animal::Horse,
                Animal::Ram,
            ],
            r3: [Animal::Ram],
            r4: [Animal::Monkey],
            r5: [
                Animal::Monkey,
                Animal::Rooster,
                Animal::Rooster,
                Animal::Dog,
                Animal::Dog,
                Animal::Boar,
                Animal::Boar,
                Animal::Fish,
                Animal::Fish,
                Animal::Elephant,
                Animal::Elephant,
                Animal::Squid,
            ],
            r6: [Animal::Squid, Animal::Frog],
            beta_reserve: [Animal::Frog],
        }
    }

    #[test]
    fn evaluation_estimates_enforce_the_public_range() {
        assert_eq!(
            EvaluationEstimate::from_millipoints(-100_000),
            Some(EvaluationEstimate::MIN)
        );
        assert_eq!(
            EvaluationEstimate::from_millipoints(100_000),
            Some(EvaluationEstimate::MAX)
        );
        assert_eq!(EvaluationEstimate::from_millipoints(-100_001), None);
        assert_eq!(EvaluationEstimate::from_millipoints(100_001), None);
    }

    #[test]
    fn evaluation_compression_round_trips_and_preserves_order() {
        let beta_fast = MateInN::new(Player::Beta, 0).unwrap();
        let beta_slow = MateInN::new(Player::Beta, MateInN::MAX_PLIES).unwrap();
        let low_estimate = EvaluationEstimate::MIN;
        let high_estimate = EvaluationEstimate::MAX;
        let alpha_slow = MateInN::new(Player::Alpha, MateInN::MAX_PLIES).unwrap();
        let alpha_fast = MateInN::new(Player::Alpha, 0).unwrap();
        let evaluations = [
            Evaluation::from(beta_fast),
            Evaluation::from(beta_slow),
            Evaluation::from(low_estimate),
            Evaluation::from(high_estimate),
            Evaluation::from(alpha_slow),
            Evaluation::from(alpha_fast),
        ];

        for evaluation in evaluations {
            let compressed = evaluation.compress();
            assert_eq!(compressed.decompress(), evaluation);
        }
        assert!(evaluations.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(evaluations[0].compress().raw, -2_000_000);
        assert_eq!(evaluations[1].compress().raw, -1_000_000);
        assert_eq!(evaluations[2].compress().raw, -100_000);
        assert_eq!(evaluations[3].compress().raw, 100_000);
        assert_eq!(evaluations[4].compress().raw, 1_000_000);
        assert_eq!(evaluations[5].compress().raw, 2_000_000);
        assert_eq!(MateInN::new(Player::Alpha, MateInN::MAX_PLIES + 1), None);

        let beta_one = MateInN::new(Player::Beta, 1).unwrap();
        let alpha_one = MateInN::new(Player::Alpha, 1).unwrap();
        assert!(beta_fast < beta_one && beta_one < beta_slow);
        assert!(alpha_slow < alpha_one && alpha_one < alpha_fast);
        assert!(low_estimate < EvaluationEstimate::ZERO);
        assert!(EvaluationEstimate::ZERO < high_estimate);
    }

    #[test]
    fn optimal_outcomes_use_game_theoretic_evaluation_order() {
        let beta_mate = OptimalOutcome::MateInN(MateInN::new(Player::Beta, 3).unwrap());
        let draw = OptimalOutcome::Draw;
        let alpha_slow = OptimalOutcome::MateInN(MateInN::new(Player::Alpha, 5).unwrap());
        let alpha_fast = OptimalOutcome::MateInN(MateInN::new(Player::Alpha, 3).unwrap());

        assert!(beta_mate < draw);
        assert!(draw < alpha_slow);
        assert!(alpha_slow < alpha_fast);
        assert_eq!(draw.as_evaluation(), EvaluationEstimate::ZERO.into());
    }

    #[test]
    fn evaluation_debug_format_is_stable_and_honors_precision() {
        let negative = EvaluationEstimate::from_millipoints(-1_234).unwrap();
        let positive = EvaluationEstimate::from_millipoints(78_900).unwrap();
        let alpha_mate = MateInN::new(Player::Alpha, 12).unwrap();
        let beta_mate = MateInN::new(Player::Beta, 3).unwrap();

        assert_eq!(format!("{negative:?}"), "-1.234");
        assert_eq!(format!("{positive:?}"), "+78.900");
        assert_eq!(format!("{positive:.1?}"), "+78.9");
        assert_eq!(format!("{negative:.2?}"), "-1.23");
        assert_eq!(
            format!(
                "{:?}",
                EvaluationEstimate::from_millipoints(99_999).unwrap()
            ),
            "+99.999"
        );
        assert_eq!(
            format!(
                "{:.2?}",
                EvaluationEstimate::from_millipoints(99_999).unwrap()
            ),
            "+100.00"
        );
        assert_eq!(format!("{positive:.6?}"), "+78.900000");
        assert_eq!(format!("{alpha_mate:?}"), "+#12");
        assert_eq!(format!("{beta_mate:?}"), "-#3");
        assert_eq!(format!("{:?}", positive.compress()), "+78.900");
    }

    #[test]
    fn card_multiset_supports_every_legal_animal_multiplicity() {
        let alpha = CardMultiset::singleton(Card::Animal(Animal::Mouse), Player::Alpha);
        let beta = CardMultiset::singleton(Card::Animal(Animal::Mouse), Player::Beta);

        let split = alpha.checked_add(beta).unwrap();
        assert_eq!(split.count(Card::Animal(Animal::Mouse), Player::Alpha), 1);
        assert_eq!(split.count(Card::Animal(Animal::Mouse), Player::Beta), 1);

        let allied = alpha.checked_add(alpha).unwrap();
        assert_eq!(allied.count(Card::Animal(Animal::Mouse), Player::Alpha), 2);
        assert_eq!(allied.count(Card::Animal(Animal::Mouse), Player::Beta), 0);
        assert!(allied.checked_add(beta).is_none());

        let singleton = allied
            .remove_one(Card::Animal(Animal::Mouse), Player::Alpha)
            .unwrap();
        assert_eq!(
            singleton.count(Card::Animal(Animal::Mouse), Player::Alpha),
            1
        );
        assert_eq!(
            singleton
                .remove_one(Card::Animal(Animal::Mouse), Player::Alpha)
                .unwrap()
                .count(Card::Animal(Animal::Mouse), Player::Alpha),
            0
        );
    }

    #[test]
    fn card_multiset_rejects_duplicate_snipes() {
        let alpha_snipe = CardMultiset::singleton(Card::Snipe, Player::Alpha);
        let beta_snipe = CardMultiset::singleton(Card::Snipe, Player::Beta);
        assert!(alpha_snipe.checked_add(alpha_snipe).is_none());
        assert!(alpha_snipe.checked_add(beta_snipe).is_some());
    }

    #[test]
    fn animal_properties_match_the_rulebook() {
        assert!(Animal::Mouse.is_retreater());
        assert!(Animal::Squid.is_retreater());
        assert!(!Animal::Tiger.is_retreater());
        assert_eq!(Animal::Rooster.unary_element(), Some(Element::Fire));
        assert_eq!(Animal::Rooster.binary_element(), Some(Element::Air));
        assert_eq!(Animal::Tiger.ternary_element(), Some(Element::Fire));

        let fire_one_and_three = cards(&[
            (Card::Animal(Animal::Rooster), Player::Beta),
            (Card::Animal(Animal::Tiger), Player::Alpha),
        ]);
        assert!(Animal::Mouse.would_activate_triplet_by_entering(fire_one_and_three));
        assert!(!Animal::Ox.would_activate_triplet_by_entering(fire_one_and_three));
    }

    #[test]
    fn initial_state_builder_validates_two_of_each_animal() {
        let builder = valid_initial_state_builder();

        let state = builder.clone().build().unwrap();
        assert_eq!(state.active_player, Player::Beta);
        assert_eq!(state.r1.count(Card::Snipe, Player::Alpha), 1);
        assert_eq!(state.r6.count(Card::Snipe, Player::Beta), 1);

        let mut invalid = builder;
        invalid.beta_reserve = [Animal::Mouse];
        assert!(invalid.clone().build().is_none());
        assert!(invalid.build_without_major_balance_check().is_none());
    }

    #[test]
    fn initial_state_builder_validates_major_animal_balance_unless_bypassed() {
        let mut builder = valid_initial_state_builder();

        assert!(builder.clone().build().is_some());

        // Swap an Alpha minor for a Beta major while preserving two copies of
        // every animal. Alpha now has 5 Major Animals and Beta has 3.
        builder.alpha_reserve = [Animal::Fish];
        builder.r5[7] = Animal::Mouse;

        assert!(builder.clone().build().is_none());
        assert!(builder.build_without_major_balance_check().is_some());
    }

    #[test]
    fn a_legal_first_step_can_immediately_lose() {
        let mut state = empty_state(Player::Alpha);
        state.r1 = cards(&[
            (Card::Snipe, Player::Alpha),
            (Card::Animal(Animal::Rabbit), Player::Alpha),
        ]);
        state.r2 = CardMultiset::singleton(Card::Animal(Animal::Tiger), Player::Beta);
        state.r3 = CardMultiset::singleton(Card::Animal(Animal::Dragon), Player::Beta);
        state.r4 = CardMultiset::singleton(Card::Animal(Animal::Fish), Player::Beta);
        state.r5 = CardMultiset::singleton(Card::Animal(Animal::Elephant), Player::Beta);
        state.r6 = cards(&[
            (Card::Snipe, Player::Beta),
            (Card::Animal(Animal::Ox), Player::Beta),
        ]);

        let step = Action::AnimalStep(AnimalStep {
            actor: Animal::Rabbit,
            direction: StepDirection::Advance,
            destination: Rank::R2,
        });
        let mut legal = Vec::new();
        state.write_legal_actions(&mut legal);
        assert!(legal.contains(&step));
        assert_eq!(state.winner(), None);

        let state = state.apply(step).unwrap();
        assert_eq!(state.winner(), Some(Player::Beta));
        let mut legal = Vec::new();
        state.write_legal_actions(&mut legal);
        assert!(legal.is_empty());
    }

    #[test]
    fn an_animal_step_activates_a_triplet_and_captures_the_destination() {
        let mut state = empty_state(Player::Alpha);
        state.r1 = cards(&[
            (Card::Snipe, Player::Alpha),
            (Card::Animal(Animal::Mouse), Player::Alpha),
        ]);
        state.r2 = cards(&[
            (Card::Animal(Animal::Rooster), Player::Beta),
            (Card::Animal(Animal::Tiger), Player::Beta),
            (Card::Snipe, Player::Beta),
        ]);

        let state = state
            .apply(Action::AnimalStep(AnimalStep {
                actor: Animal::Mouse,
                direction: StepDirection::Advance,
                destination: Rank::R2,
            }))
            .unwrap();

        assert_eq!(
            state.r2.count(Card::Animal(Animal::Mouse), Player::Alpha),
            1
        );
        assert_eq!(state.r2.card_count(), 1);
        assert_eq!(
            state
                .reserves
                .count(Card::Animal(Animal::Rooster), Player::Alpha),
            1
        );
        assert_eq!(
            state
                .reserves
                .count(Card::Animal(Animal::Tiger), Player::Alpha),
            1
        );
        assert_eq!(state.reserves.count(Card::Snipe, Player::Beta), 1);
        assert_eq!(state.winner(), Some(Player::Alpha));
    }

    #[test]
    fn two_copies_allow_same_animal_on_both_steps_but_one_copy_does_not() {
        let leading = AnimalStep {
            actor: Animal::Rabbit,
            direction: StepDirection::Advance,
            destination: Rank::R2,
        };
        let second = Action::AnimalStep(AnimalStep {
            actor: Animal::Rabbit,
            direction: StepDirection::Retreat,
            destination: Rank::R1,
        });

        let mut two_copies = empty_state(Player::Alpha);
        two_copies.leading_action = Some(leading);
        two_copies.r1 = CardMultiset::singleton(Card::Snipe, Player::Alpha);
        two_copies.r2 = cards(&[
            (Card::Animal(Animal::Rabbit), Player::Alpha),
            (Card::Animal(Animal::Rabbit), Player::Alpha),
        ]);
        let after = two_copies.apply(second).unwrap();
        assert_eq!(after.active_player, Player::Beta);
        assert_eq!(after.leading_action, None);
        assert_eq!(
            after.r2.count(Card::Animal(Animal::Rabbit), Player::Alpha),
            1
        );

        let mut one_copy = empty_state(Player::Alpha);
        one_copy.leading_action = Some(leading);
        one_copy.r1 = CardMultiset::singleton(Card::Snipe, Player::Alpha);
        one_copy.r2 = cards(&[
            (Card::Animal(Animal::Rabbit), Player::Alpha),
            (Card::Animal(Animal::Ox), Player::Alpha),
        ]);
        assert_eq!(
            one_copy.apply(second).unwrap_err(),
            IllegalActionError::CannotMoveSameAnimalTwice
        );
    }

    #[test]
    fn standalone_actions_are_forbidden_after_a_leading_animal_step() {
        let mut state = empty_state(Player::Alpha);
        state.leading_action = Some(AnimalStep {
            actor: Animal::Mouse,
            direction: StepDirection::Advance,
            destination: Rank::R2,
        });
        state.r1 = cards(&[
            (Card::Snipe, Player::Alpha),
            (Card::Animal(Animal::Ox), Player::Alpha),
        ]);
        state.reserves = cards(&[
            (Card::Animal(Animal::Rabbit), Player::Alpha),
            (Card::Animal(Animal::Snake), Player::Alpha),
        ]);

        assert_eq!(
            state
                .clone()
                .apply(Action::SnipeStep(SnipeStep {
                    destination: Rank::R2,
                }))
                .unwrap_err(),
            IllegalActionError::AlreadyMovedAnimal
        );
        assert_eq!(
            state
                .apply(Action::Drop(AnimalDrop {
                    actor: Animal::Rabbit,
                    destination: Rank::R3,
                }))
                .unwrap_err(),
            IllegalActionError::AlreadyMovedAnimal
        );
    }

    #[test]
    fn a_snipe_step_and_a_drop_finish_the_turn() {
        let mut snipe_state = empty_state(Player::Alpha);
        snipe_state.r1 = cards(&[
            (Card::Snipe, Player::Alpha),
            (Card::Animal(Animal::Ox), Player::Alpha),
        ]);
        let snipe_state = snipe_state
            .apply(Action::SnipeStep(SnipeStep {
                destination: Rank::R2,
            }))
            .unwrap();
        assert_eq!(snipe_state.active_player, Player::Beta);
        assert_eq!(snipe_state.r2.count(Card::Snipe, Player::Alpha), 1);

        let mut drop_state = empty_state(Player::Beta);
        drop_state.reserves = cards(&[
            (Card::Animal(Animal::Ox), Player::Beta),
            (Card::Animal(Animal::Tiger), Player::Beta),
        ]);
        let drop_state = drop_state
            .apply(Action::Drop(AnimalDrop {
                actor: Animal::Ox,
                destination: Rank::R1,
            }))
            .unwrap();
        assert_eq!(drop_state.active_player, Player::Alpha);
        assert_eq!(
            drop_state.r1.count(Card::Animal(Animal::Ox), Player::Beta),
            1
        );
    }
}
