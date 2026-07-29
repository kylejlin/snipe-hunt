use snipe_core::{
    Action, Animal, AnimalDrop, AnimalStep, Card, CardMultiset, Player, Rank, SnipeStep, State,
    StepDirection,
};

pub(crate) const MAX_ACTIONS: usize = 128;
pub(crate) const MAX_LINE: usize = 64;
pub(crate) const MAX_TURNS: usize = 2_048;
const TURN_DEDUP_SLOTS: usize = MAX_TURNS * 2;
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

const RETREATERS: u16 = (1 << Animal::Mouse as u16)
    | (1 << Animal::Rabbit as u16)
    | (1 << Animal::Snake as u16)
    | (1 << Animal::Ram as u16)
    | (1 << Animal::Boar as u16)
    | (1 << Animal::Squid as u16);
const UNARY: [u16; 4] = [
    (1 << Animal::Rooster as u16) | (1 << Animal::Boar as u16) | (1 << Animal::Squid as u16),
    (1 << Animal::Ox as u16) | (1 << Animal::Rabbit as u16) | (1 << Animal::Dog as u16),
    (1 << Animal::Mouse as u16) | (1 << Animal::Snake as u16) | (1 << Animal::Monkey as u16),
    (1 << Animal::Horse as u16) | (1 << Animal::Ram as u16) | (1 << Animal::Frog as u16),
];
const BINARY: [u16; 4] = [
    (1 << Animal::Mouse as u16) | (1 << Animal::Horse as u16) | (1 << Animal::Dog as u16),
    (1 << Animal::Snake as u16) | (1 << Animal::Squid as u16) | (1 << Animal::Frog as u16),
    (1 << Animal::Ox as u16) | (1 << Animal::Ram as u16) | (1 << Animal::Boar as u16),
    (1 << Animal::Rabbit as u16) | (1 << Animal::Monkey as u16) | (1 << Animal::Rooster as u16),
];
const TERNARY: [u16; 4] = [
    1 << Animal::Tiger as u16,
    1 << Animal::Fish as u16,
    1 << Animal::Elephant as u16,
    1 << Animal::Dragon as u16,
];
static ACTIVATION_ACTORS: [u16; 1 << 16] = build_activation_actor_table();

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Cell {
    alpha: u16,
    beta: u16,
    twins: u16,
    snipes: u8,
}

impl Cell {
    fn from_core(cards: CardMultiset) -> Self {
        let mut result = Self::default();
        for (index, animal) in ANIMALS.into_iter().enumerate() {
            for player in [Player::Alpha, Player::Beta] {
                let count = cards.count(Card::Animal(animal), player);
                for _ in 0..count {
                    result.add_animal(player, index);
                }
            }
        }
        result.snipes = (cards.count(Card::Snipe, Player::Alpha) != 0) as u8
            | (((cards.count(Card::Snipe, Player::Beta) != 0) as u8) << 1);
        result
    }

    #[inline]
    pub(crate) fn presence(self) -> u16 {
        self.alpha | self.beta
    }

    #[inline]
    pub(crate) fn owned_presence(self, player: Player) -> u16 {
        match player {
            Player::Alpha => self.alpha,
            Player::Beta => self.beta,
        }
    }

    #[inline]
    pub(crate) fn animal_count(self, player: Player) -> u32 {
        let owned = self.owned_presence(player);
        owned.count_ones() + (self.twins & owned).count_ones()
    }

    #[inline]
    pub(crate) fn total_animals(self) -> u32 {
        self.alpha.count_ones() + self.beta.count_ones() + self.twins.count_ones()
    }

    #[inline]
    pub(crate) fn card_count(self) -> u32 {
        self.total_animals() + (self.snipes & 0b11).count_ones()
    }

    #[inline]
    pub(crate) fn category_count(self, player: Player, category: u16) -> i32 {
        let owned = self.owned_presence(player) & category;
        (owned.count_ones() + (self.twins & owned).count_ones()) as i32
    }

    #[inline]
    pub(crate) fn nonretreater_count(self, player: Player) -> i32 {
        self.category_count(player, !RETREATERS)
    }

    #[inline]
    pub(crate) fn count(self, player: Player, animal: usize) -> u8 {
        let bit = 1 << animal;
        let owned = self.owned_presence(player);
        if owned & bit == 0 {
            0
        } else {
            1 + ((self.twins & bit) != 0) as u8
        }
    }

    #[inline]
    fn add_animal(&mut self, player: Player, animal: usize) {
        let bit = 1 << animal;
        let owned = match player {
            Player::Alpha => &mut self.alpha,
            Player::Beta => &mut self.beta,
        };
        if *owned & bit == 0 {
            *owned |= bit;
        } else {
            debug_assert_eq!(self.twins & bit, 0);
            self.twins |= bit;
        }
    }

    #[inline]
    fn remove_animal(&mut self, player: Player, animal: usize) {
        let bit = 1 << animal;
        if self.twins & bit != 0 {
            self.twins &= !bit;
            return;
        }
        match player {
            Player::Alpha => self.alpha &= !bit,
            Player::Beta => self.beta &= !bit,
        }
    }

    #[inline]
    pub(crate) fn has_snipe(self, player: Player) -> bool {
        self.snipes & player_bit(player) != 0
    }
}

/// An animal leading step is encoded as one plus `(destination << 4 | actor)`.
/// Direction is irrelevant to the only second-step restriction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PackedState {
    pub(crate) cells: [Cell; 7],
    pub(crate) active: Player,
    leading: u8,
}

impl PackedState {
    pub(crate) fn from_core(state: &State) -> Self {
        let leading = state.leading_action.map_or(0, |step| {
            1 + (rank_index(step.destination) as u8) * 16 + step.actor as u8
        });
        Self {
            cells: [
                Cell::from_core(state.r1),
                Cell::from_core(state.r2),
                Cell::from_core(state.r3),
                Cell::from_core(state.r4),
                Cell::from_core(state.r5),
                Cell::from_core(state.r6),
                Cell::from_core(state.reserves),
            ],
            active: state.active_player,
            leading,
        }
    }

    #[inline]
    pub(crate) fn has_leading(self) -> bool {
        self.leading != 0
    }

    #[inline]
    fn leading_actor_and_destination(self) -> Option<(usize, usize)> {
        (self.leading != 0).then(|| {
            let encoded = usize::from(self.leading - 1);
            (encoded & 15, encoded >> 4)
        })
    }

    pub(crate) fn captured_winner(self) -> Option<Player> {
        let reserve = self.cells[6];
        if reserve.has_snipe(Player::Beta) {
            Some(Player::Alpha)
        } else if reserve.has_snipe(Player::Alpha) {
            Some(Player::Beta)
        } else {
            None
        }
    }

    pub(crate) fn snipe_rank(self, player: Player) -> Option<usize> {
        (0..6).find(|&rank| self.cells[rank].has_snipe(player))
    }

    pub(crate) fn hash(self) -> u64 {
        let mut hash = 0x6A09_E667_F3BC_C909u64;
        for (index, cell) in self.cells.into_iter().enumerate() {
            let animals = u64::from(cell.alpha)
                | (u64::from(cell.beta) << 16)
                | (u64::from(cell.twins) << 32)
                | (u64::from(cell.snipes) << 48)
                | ((index as u64) << 56);
            hash ^= mix64(animals.wrapping_add(hash.rotate_left(17)));
        }
        hash ^= match self.active {
            Player::Alpha => 0x243F_6A88_85A3_08D3,
            Player::Beta => 0x1319_8A2E_0370_7344,
        };
        mix64(hash ^ (u64::from(self.leading) << 40))
    }

    /// Writes every legal action except actions cheaply proven to lose by
    /// force. No heap allocation occurs.
    pub(crate) fn write_reasonable_actions(self, output: &mut GeneratedMoves) {
        output.clear();
        if self.captured_winner().is_some() {
            return;
        }
        self.write_actions(false, output);
    }

    pub(crate) fn write_search_candidates(self, output: &mut GeneratedMoves) {
        output.clear();
        if self.captured_winner().is_some() {
            return;
        }
        self.write_actions(true, output);
    }

    /// Writes complete turns. Animal turns contain both constituent steps;
    /// snipe moves, drops, and winning first captures contain one action.
    pub(crate) fn write_reasonable_turns(self, output: &mut TurnList) {
        output.clear();
        let mut seen = [u16::MAX; TURN_DEDUP_SLOTS];
        let mut first_actions = GeneratedMoves::default();
        self.write_search_candidates(&mut first_actions);
        for index in 0..first_actions.moves.len() {
            let first = first_actions.moves.get(index);
            let Some(after_first) = self.reasonable_child(first) else {
                continue;
            };
            if after_first.captured_winner().is_some() || after_first.active != self.active {
                output.push_unique(self, PackedTurn::single(first), after_first, &mut seen);
                continue;
            }

            let mut second_actions = GeneratedMoves::default();
            after_first.write_search_candidates(&mut second_actions);
            for second_index in 0..second_actions.moves.len() {
                let second = second_actions.moves.get(second_index);
                if let Some(after_second) = after_first.reasonable_child(second) {
                    output.push_unique(
                        self,
                        PackedTurn::pair(first, second),
                        after_second,
                        &mut seen,
                    );
                }
            }
        }
    }

    /// Returns a complete legal turn for positions where every turn is a
    /// proven loss and the reasonable list is therefore empty.
    pub(crate) fn first_legal_turn(self) -> Option<PackedTurn> {
        let mut first_actions = GeneratedMoves::default();
        self.write_search_candidates(&mut first_actions);
        for index in 0..first_actions.moves.len() {
            let first = first_actions.moves.get(index);
            let after_first = self.apply(first);
            if after_first.captured_winner().is_some() || after_first.active != self.active {
                return Some(PackedTurn::single(first));
            }

            let mut second_actions = GeneratedMoves::default();
            after_first.write_search_candidates(&mut second_actions);
            if !second_actions.moves.is_empty() {
                return Some(PackedTurn::pair(first, second_actions.moves.get(0)));
            }
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn write_legal_actions(self, output: &mut MoveList) {
        output.clear();
        if self.captured_winner().is_some() {
            return;
        }
        let mut generated = GeneratedMoves::default();
        self.write_actions(true, &mut generated);
        *output = generated.moves;
    }

    fn write_actions(self, include_unreasonable: bool, output: &mut GeneratedMoves) {
        if !self.has_leading() {
            if let Some(source) = self.snipe_rank(self.active)
                && self.cells[source].card_count() > 1
            {
                if let Some(destination) = advance(source, self.active) {
                    self.push_candidate(
                        PackedMove::snipe(destination),
                        include_unreasonable,
                        output,
                    );
                }
                if let Some(destination) = retreat(source, self.active) {
                    self.push_candidate(
                        PackedMove::snipe(destination),
                        include_unreasonable,
                        output,
                    );
                }
            }

            let reserve = self.cells[6];
            if reserve.animal_count(self.active) > 1 {
                let mut animals = reserve.owned_presence(self.active);
                while animals != 0 {
                    let actor = animals.trailing_zeros() as usize;
                    animals &= animals - 1;
                    for destination in 0..6 {
                        if is_retreater(actor) && !legal_retreater_drop(self.active, destination) {
                            continue;
                        }
                        self.push_candidate(
                            PackedMove::drop(actor, destination),
                            include_unreasonable,
                            output,
                        );
                    }
                }
            }
        }

        for source in 0..6 {
            let mut animals = self.cells[source].owned_presence(self.active);
            while animals != 0 {
                let actor = animals.trailing_zeros() as usize;
                animals &= animals - 1;
                if let Some(destination) = advance(source, self.active)
                    && self.legal_animal_step(source, actor, destination)
                {
                    self.push_candidate(
                        PackedMove::animal(actor, false, destination),
                        include_unreasonable,
                        output,
                    );
                }
                if is_retreater(actor)
                    && let Some(destination) = retreat(source, self.active)
                    && self.legal_animal_step(source, actor, destination)
                {
                    self.push_candidate(
                        PackedMove::animal(actor, true, destination),
                        include_unreasonable,
                        output,
                    );
                }
            }
        }
    }

    #[inline]
    fn push_candidate(
        self,
        action: PackedMove,
        include_unreasonable: bool,
        output: &mut GeneratedMoves,
    ) {
        output.legal_count += 1;
        if output.fallback.is_none() {
            output.fallback = Some(action);
        }
        let first_animal_step = !self.has_leading() && matches!(action.kind(), MoveKind::Animal);
        if include_unreasonable || first_animal_step || !self.action_is_unreasonable(action) {
            output.moves.push(action);
        }
    }

    pub(crate) fn action_is_unreasonable(self, action: PackedMove) -> bool {
        self.reasonable_child(action).is_none()
    }

    pub(crate) fn reasonable_child(self, action: PackedMove) -> Option<Self> {
        let child = self.apply(action);
        if let Some(winner) = child.captured_winner() {
            return (winner == self.active).then_some(child);
        }
        if child.active == self.active {
            return Some(child);
        }
        (!child.opponent_can_capture_snipe_this_turn(self.active)).then_some(child)
    }

    fn legal_animal_step(self, source: usize, actor: usize, destination: usize) -> bool {
        debug_assert_ne!(source, destination);
        let friendly_count = self.cells[source].count(self.active, actor);
        if friendly_count == 0 {
            return false;
        }
        if let Some((leading_actor, leading_destination)) = self.leading_actor_and_destination()
            && leading_actor == actor
            && leading_destination == source
            && friendly_count < 2
        {
            return false;
        }

        let destination_cell = self.cells[destination];
        let activates = activates_triplet(actor, destination_cell.presence());
        let enemy_snipe = destination_cell.has_snipe(self.active.opponent());
        let friendly_snipe = destination_cell.has_snipe(self.active);
        if self.cells[source].card_count() <= 1 {
            activates && enemy_snipe
        } else {
            !(activates && friendly_snipe && !enemy_snipe)
        }
    }

    fn opponent_can_capture_snipe_immediately(self, victim: Player) -> bool {
        debug_assert_eq!(self.active, victim.opponent());
        let Some(destination) = self.snipe_rank(victim) else {
            return false;
        };
        let attacker = self.active;
        for retreating in [false, true] {
            let source = if retreating {
                advance(destination, attacker)
            } else {
                retreat(destination, attacker)
            };
            let Some(source) = source else {
                continue;
            };
            let mut animals = self.cells[source].owned_presence(attacker);
            while animals != 0 {
                let actor = animals.trailing_zeros() as usize;
                animals &= animals - 1;
                if retreating && !is_retreater(actor) {
                    continue;
                }
                if self.legal_animal_step(source, actor, destination) {
                    let action = PackedMove::animal(actor, retreating, destination);
                    if self.apply(action).captured_winner() == Some(attacker) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn opponent_can_capture_snipe_this_turn(self, victim: Player) -> bool {
        if self.opponent_can_capture_snipe_immediately(victim) {
            return true;
        }
        let Some(destination) = self.snipe_rank(victim) else {
            return false;
        };
        let attacker = self.active;
        for retreating in [false, true] {
            let source = if retreating {
                advance(destination, attacker)
            } else {
                retreat(destination, attacker)
            };
            let Some(source) = source else {
                continue;
            };
            let mut animals = self.cells[source].owned_presence(attacker);
            while animals != 0 {
                let actor = animals.trailing_zeros() as usize;
                animals &= animals - 1;
                if retreating && !is_retreater(actor) {
                    continue;
                }
                if !self.legal_animal_step(source, actor, destination) {
                    continue;
                }
                let setup = self.apply(PackedMove::animal(actor, retreating, destination));
                if setup.captured_winner() == Some(attacker)
                    || setup.opponent_can_capture_snipe_immediately(victim)
                {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn rank_is_pressed_by(self, attacker: Player, destination: usize) -> bool {
        let destination_cell = self.cells[destination];
        self.rank_is_pressed_by_with_presence(attacker, destination, destination_cell.presence())
    }

    fn rank_is_pressed_by_with_presence(
        self,
        attacker: Player,
        destination: usize,
        destination_presence: u16,
    ) -> bool {
        let destination_cell = self.cells[destination];
        let activation_actors = activation_actor_mask(destination_presence);
        if activation_actors == 0 {
            return false;
        }
        let enemy_snipe = destination_cell.has_snipe(attacker.opponent());
        if destination_cell.has_snipe(attacker) && !enemy_snipe {
            return false;
        }
        for retreating in [false, true] {
            let source = if retreating {
                advance(destination, attacker)
            } else {
                retreat(destination, attacker)
            };
            let Some(source) = source else {
                continue;
            };
            let source_cell = self.cells[source];
            let mut animals = source_cell.owned_presence(attacker) & activation_actors;
            if retreating {
                animals &= RETREATERS;
            }
            if animals != 0 && (source_cell.card_count() > 1 || enemy_snipe) {
                return true;
            }
        }
        false
    }

    /// Counts distinct reserve drops that would give the attacker a capture
    /// of the victim's snipe on the attacker's following full animal turn.
    ///
    /// The drop can matter on the snipe's rank or on an adjacent source rank:
    /// Cherry, for example, likes to assemble a two-animal activation next to
    /// the snipe and then use the first animal as setup for the second.
    pub(crate) fn snipe_setup_drop_count(self, victim: Player) -> u32 {
        let Some(snipe_rank) = self.snipe_rank(victim) else {
            return 0;
        };
        let attacker = victim.opponent();
        if self.player_can_capture_snipe_this_turn(attacker, victim)
            || self.cells[6].animal_count(attacker) <= 1
        {
            return 0;
        }
        let mut attacker_state = self;
        attacker_state.active = attacker;
        attacker_state.leading = 0;
        let mut drops = attacker_state.cells[6].owned_presence(attacker);
        let mut count = 0;
        while drops != 0 {
            let actor = drops.trailing_zeros() as usize;
            drops &= drops - 1;
            let first_destination = snipe_rank.saturating_sub(1);
            let last_destination = (snipe_rank + 1).min(5);
            for destination in first_destination..=last_destination {
                if is_retreater(actor) && !legal_retreater_drop(attacker, destination) {
                    continue;
                }
                let after_drop = attacker_state.apply(PackedMove::drop(actor, destination));
                count += u32::from(after_drop.player_can_capture_snipe_this_turn(attacker, victim));
            }
        }
        count
    }

    /// Detects a sound mate-in-two motif at the quiescence horizon: the
    /// active player can make a legal, non-losing drop after which every
    /// complete reply still permits capture of the opposing snipe.
    pub(crate) fn active_has_mating_setup_drop(self) -> bool {
        let attacker = self.active;
        let victim = attacker.opponent();
        let Some(snipe_rank) = self.snipe_rank(victim) else {
            return false;
        };
        if self.cells[6].animal_count(attacker) <= 1 {
            return false;
        }

        let mut drops = self.cells[6].owned_presence(attacker);
        while drops != 0 {
            let actor = drops.trailing_zeros() as usize;
            drops &= drops - 1;
            let first_destination = snipe_rank.saturating_sub(1);
            let last_destination = (snipe_rank + 1).min(5);
            for destination in first_destination..=last_destination {
                if is_retreater(actor) && !legal_retreater_drop(attacker, destination) {
                    continue;
                }
                let action = PackedMove::drop(actor, destination);
                let Some(after_drop) = self.reasonable_child(action) else {
                    continue;
                };
                if !after_drop.player_can_capture_snipe_this_turn(attacker, victim) {
                    continue;
                }
                let mut evasions = TurnList::default();
                after_drop.write_reasonable_turns(&mut evasions);
                if evasions.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    /// Proves a short forcing net using only attack turns that leave the
    /// victim's snipe under a direct full-turn threat. Searching a subset of
    /// attacks can miss mates, but a reported mate is sound because every
    /// reasonable victim reply is checked.
    pub(crate) fn active_can_force_snipe_capture_in(self, attack_turns: u8) -> bool {
        debug_assert!(attack_turns > 0);
        let attacker = self.active;
        let mut attacks = TurnList::default();
        self.write_reasonable_turns(&mut attacks);
        for index in 0..attacks.len() {
            let after_attack = self.apply_turn(attacks.get(index));
            if after_attack.captured_winner() == Some(attacker) {
                return true;
            }
            if !after_attack.active_snipe_is_pressed() {
                continue;
            }

            let mut replies = TurnList::default();
            after_attack.write_reasonable_turns(&mut replies);
            if replies.is_empty() {
                return true;
            }
            if attack_turns == 1 {
                continue;
            }
            let all_replies_lose = (0..replies.len()).all(|reply_index| {
                let after_reply = after_attack.apply_turn(replies.get(reply_index));
                after_reply.active_can_force_snipe_capture_in(attack_turns - 1)
            });
            if all_replies_lose {
                return true;
            }
        }
        false
    }

    pub(crate) fn active_snipe_is_pressed(self) -> bool {
        self.player_can_capture_snipe_this_turn(self.active.opponent(), self.active)
    }

    pub(crate) fn snipe_safe_exit_count(self, player: Player) -> u32 {
        let Some(source) = self.snipe_rank(player) else {
            return 0;
        };
        if self.cells[source].card_count() <= 1 {
            return 0;
        }
        let mut probe = self;
        probe.active = player;
        probe.leading = 0;
        let mut count = 0;
        if let Some(destination) = advance(source, player) {
            let child = probe.apply(PackedMove::snipe(destination));
            count += u32::from(!child.opponent_can_capture_snipe_this_turn(player));
        }
        if let Some(destination) = retreat(source, player) {
            let child = probe.apply(PackedMove::snipe(destination));
            count += u32::from(!child.opponent_can_capture_snipe_this_turn(player));
        }
        count
    }

    pub(crate) fn player_can_capture_snipe_this_turn(
        self,
        attacker: Player,
        victim: Player,
    ) -> bool {
        if self.captured_winner() == Some(attacker) {
            return true;
        }
        if self.snipe_rank(victim).is_none() {
            return false;
        }
        let mut attack = self;
        attack.active = attacker;
        attack.leading = 0;
        attack.opponent_can_capture_snipe_this_turn(victim)
    }

    pub(crate) fn snipe_near_pressure_count(self, victim: Player) -> u32 {
        let Some(destination) = self.snipe_rank(victim) else {
            return 0;
        };
        let attacker = victim.opponent();
        let presence = self.cells[destination].presence();
        let mut count = 0;
        for retreating in [false, true] {
            let source = if retreating {
                advance(destination, attacker)
            } else {
                retreat(destination, attacker)
            };
            let Some(source) = source else {
                continue;
            };
            let mut animals = self.cells[source].owned_presence(attacker);
            if retreating {
                animals &= RETREATERS;
            }
            while animals != 0 {
                let actor = animals.trailing_zeros() as usize;
                animals &= animals - 1;
                count += u32::from(activation_missing_roles(actor, presence) == 1);
            }
        }
        count
    }

    pub(crate) fn snipe_near_attacker_count(self, victim: Player) -> u32 {
        let Some(snipe_rank) = self.snipe_rank(victim) else {
            return 0;
        };
        let attacker = victim.opponent();
        let mut count = 0;
        if snipe_rank > 0 {
            count += self.cells[snipe_rank - 1].animal_count(attacker);
        }
        if snipe_rank < 5 {
            count += self.cells[snipe_rank + 1].animal_count(attacker);
        }
        count
    }

    pub(crate) fn apply(mut self, action: PackedMove) -> Self {
        match action.kind() {
            MoveKind::Snipe => {
                let source = self
                    .snipe_rank(self.active)
                    .expect("generated snipe has a source");
                let bit = player_bit(self.active);
                self.cells[source].snipes &= !bit;
                self.cells[action.destination()].snipes |= bit;
                self.active = self.active.opponent();
            }
            MoveKind::Drop => {
                self.cells[6].remove_animal(self.active, action.actor());
                self.cells[action.destination()].add_animal(self.active, action.actor());
                self.active = self.active.opponent();
            }
            MoveKind::Animal => {
                let destination = action.destination();
                let source = if action.retreating() {
                    advance(destination, self.active)
                } else {
                    retreat(destination, self.active)
                }
                .expect("generated animal step has a source");
                self.cells[source].remove_animal(self.active, action.actor());
                if activates_triplet(action.actor(), self.cells[destination].presence()) {
                    let captured = self.cells[destination];
                    self.capture_into_reserve(captured);
                    self.cells[destination] = Cell::default();
                }
                self.cells[destination].add_animal(self.active, action.actor());
                if self.has_leading() {
                    self.leading = 0;
                    self.active = self.active.opponent();
                } else {
                    self.leading = 1 + (destination as u8) * 16 + action.actor() as u8;
                }
            }
        }
        self
    }

    #[inline]
    pub(crate) fn apply_turn(self, turn: PackedTurn) -> Self {
        let mut child = self.apply(turn.first());
        if child.captured_winner().is_none()
            && let Some(second) = turn.second()
        {
            child = child.apply(second);
        }
        child
    }

    fn capture_into_reserve(&mut self, captured: Cell) {
        let mut animals = captured.presence();
        while animals != 0 {
            let actor = animals.trailing_zeros() as usize;
            animals &= animals - 1;
            let count = captured.count(Player::Alpha, actor) + captured.count(Player::Beta, actor);
            for _ in 0..count {
                self.cells[6].add_animal(self.active, actor);
            }
        }
        self.cells[6].snipes |= captured.snipes;
    }

    pub(crate) fn move_order_score(self, action: PackedMove) -> i32 {
        match action.kind() {
            MoveKind::Animal => {
                let destination = action.destination();
                if activates_triplet(action.actor(), self.cells[destination].presence()) {
                    let target = self.cells[destination];
                    let mut score = target.total_animals() as i32 * 1_000;
                    if target.has_snipe(self.active.opponent()) {
                        score += 1_000_000;
                    }
                    if target.has_snipe(self.active) {
                        score -= 500_000;
                    }
                    score
                } else {
                    let progress = match self.active {
                        Player::Alpha => destination as i32,
                        Player::Beta => 5 - destination as i32,
                    };
                    300 + progress * 8 + i32::from(action.retreating()) * 3
                }
            }
            MoveKind::Snipe => 620,
            MoveKind::Drop => {
                let destination = action.destination();
                let actor = action.actor();
                let progress = match self.active {
                    Player::Alpha => destination as i32,
                    Player::Beta => 5 - destination as i32,
                };
                let major = matches!(actor, 2 | 4 | 12 | 13);
                let setup = activates_triplet(actor, self.cells[destination].presence());
                560 + progress * 6 + i32::from(major) * 90 + i32::from(setup) * 180
            }
        }
    }

    pub(crate) fn turn_order_score(
        self,
        turn: PackedTurn,
        preferred: Option<PackedTurn>,
        history: &[i32; 768],
    ) -> i32 {
        if preferred == Some(turn) {
            return 4_000_000;
        }
        let mut score = self.move_order_score(turn.first()) + history[turn.first().raw()];
        if let Some(second) = turn.second() {
            let after_first = self.apply(turn.first());
            score += after_first.move_order_score(second) + history[second.raw()];
        }
        if self.turn_creates_snipe_pressure(turn) {
            score += 800_000;
        }
        score
    }

    #[inline]
    pub(crate) fn move_is_capture(self, action: PackedMove) -> bool {
        matches!(action.kind(), MoveKind::Animal)
            && activates_triplet(action.actor(), self.cells[action.destination()].presence())
    }

    #[inline]
    pub(crate) fn turn_is_capture(self, turn: PackedTurn) -> bool {
        if self.move_is_capture(turn.first()) {
            return true;
        }
        turn.second()
            .is_some_and(|second| self.apply(turn.first()).move_is_capture(second))
    }

    pub(crate) fn turn_is_forcing(self, turn: PackedTurn) -> bool {
        self.turn_is_capture(turn) || self.turn_creates_snipe_pressure(turn)
    }

    fn turn_creates_snipe_pressure(self, turn: PackedTurn) -> bool {
        let child = self.apply_turn(turn);
        let attacker = self.active;
        let victim = self.active.opponent();
        child.player_can_capture_snipe_this_turn(attacker, victim)
            && !self.player_can_capture_snipe_this_turn(attacker, victim)
    }

    pub(crate) fn to_core_action(self, action: PackedMove) -> Action {
        match action.kind() {
            MoveKind::Snipe => Action::SnipeStep(SnipeStep {
                destination: rank(action.destination()),
            }),
            MoveKind::Drop => Action::Drop(AnimalDrop {
                actor: ANIMALS[action.actor()],
                destination: rank(action.destination()),
            }),
            MoveKind::Animal => Action::AnimalStep(AnimalStep {
                actor: ANIMALS[action.actor()],
                direction: if action.retreating() {
                    StepDirection::Retreat
                } else {
                    StepDirection::Advance
                },
                destination: rank(action.destination()),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PackedMove(u16);

impl PackedMove {
    #[inline]
    pub(crate) fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    #[inline]
    pub(crate) fn as_u16(self) -> u16 {
        self.0
    }

    #[inline]
    fn animal(actor: usize, retreating: bool, destination: usize) -> Self {
        Self(actor as u16 | ((destination as u16) << 4) | ((retreating as u16) << 7))
    }

    #[inline]
    fn snipe(destination: usize) -> Self {
        Self((1 << 8) | ((destination as u16) << 4))
    }

    #[inline]
    fn drop(actor: usize, destination: usize) -> Self {
        Self((2 << 8) | actor as u16 | ((destination as u16) << 4))
    }

    #[inline]
    pub(crate) fn raw(self) -> usize {
        usize::from(self.0)
    }

    #[inline]
    fn kind(self) -> MoveKind {
        match self.0 >> 8 {
            0 => MoveKind::Animal,
            1 => MoveKind::Snipe,
            2 => MoveKind::Drop,
            _ => unreachable!("invalid packed move"),
        }
    }

    #[inline]
    fn actor(self) -> usize {
        usize::from(self.0 & 15)
    }

    #[inline]
    fn destination(self) -> usize {
        usize::from((self.0 >> 4) & 7)
    }

    #[inline]
    fn retreating(self) -> bool {
        self.0 & (1 << 7) != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PackedTurn {
    first: PackedMove,
    second: Option<PackedMove>,
}

impl PackedTurn {
    #[inline]
    fn single(first: PackedMove) -> Self {
        Self {
            first,
            second: None,
        }
    }

    #[inline]
    fn pair(first: PackedMove, second: PackedMove) -> Self {
        Self {
            first,
            second: Some(second),
        }
    }

    #[inline]
    pub(crate) fn from_raw(first: u16, second: u16) -> Self {
        Self {
            first: PackedMove::from_raw(first),
            second: (second != u16::MAX).then(|| PackedMove::from_raw(second)),
        }
    }

    #[inline]
    pub(crate) fn first(self) -> PackedMove {
        self.first
    }

    #[inline]
    pub(crate) fn second(self) -> Option<PackedMove> {
        self.second
    }

    #[inline]
    pub(crate) fn first_raw(self) -> u16 {
        self.first.as_u16()
    }

    #[inline]
    pub(crate) fn second_raw(self) -> u16 {
        self.second.map_or(u16::MAX, PackedMove::as_u16)
    }
}

impl Default for PackedTurn {
    fn default() -> Self {
        Self::single(PackedMove::from_raw(0))
    }
}

#[derive(Clone, Copy)]
enum MoveKind {
    Animal,
    Snipe,
    Drop,
}

#[derive(Clone, Copy)]
pub(crate) struct MoveList {
    actions: [PackedMove; MAX_ACTIONS],
    len: u8,
}

impl Default for MoveList {
    fn default() -> Self {
        Self {
            actions: [PackedMove(0); MAX_ACTIONS],
            len: 0,
        }
    }
}

impl MoveList {
    #[inline]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    fn push(&mut self, action: PackedMove) {
        let index = usize::from(self.len);
        debug_assert!(index < MAX_ACTIONS);
        self.actions[index] = action;
        self.len += 1;
    }

    #[inline]
    pub(crate) fn len(self) -> usize {
        usize::from(self.len)
    }

    #[inline]
    pub(crate) fn is_empty(self) -> bool {
        self.len == 0
    }

    #[inline]
    pub(crate) fn get(self, index: usize) -> PackedMove {
        debug_assert!(index < self.len());
        self.actions[index]
    }

    #[cfg(test)]
    pub(crate) fn contains(self, action: PackedMove) -> bool {
        self.actions[..self.len()].contains(&action)
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct GeneratedMoves {
    pub(crate) moves: MoveList,
    pub(crate) fallback: Option<PackedMove>,
    pub(crate) legal_count: u16,
}

#[derive(Clone, Copy, Default)]
struct TurnEntry {
    turn: PackedTurn,
    score: i32,
}

#[derive(Clone, Copy)]
pub(crate) struct TurnList {
    entries: [TurnEntry; MAX_TURNS],
    len: u16,
}

impl Default for TurnList {
    fn default() -> Self {
        Self {
            entries: [TurnEntry::default(); MAX_TURNS],
            len: 0,
        }
    }
}

impl TurnList {
    #[inline]
    fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    fn push_unique(
        &mut self,
        root: PackedState,
        turn: PackedTurn,
        child: PackedState,
        seen: &mut [u16; TURN_DEDUP_SLOTS],
    ) {
        let mut slot = child.hash() as usize & (TURN_DEDUP_SLOTS - 1);
        loop {
            let existing = seen[slot];
            if existing == u16::MAX {
                let index = self.len();
                assert!(index < MAX_TURNS, "complete-turn buffer exhausted");
                self.entries[index].turn = turn;
                self.len += 1;
                seen[slot] = index as u16;
                return;
            }
            let existing_turn = self.entries[usize::from(existing)].turn;
            if root.apply_turn(existing_turn) == child {
                return;
            }
            slot = (slot + 1) & (TURN_DEDUP_SLOTS - 1);
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        usize::from(self.len)
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub(crate) fn get(&self, index: usize) -> PackedTurn {
        debug_assert!(index < self.len());
        self.entries[index].turn
    }

    pub(crate) fn contains(&self, turn: PackedTurn) -> bool {
        self.entries[..self.len()]
            .iter()
            .any(|entry| entry.turn == turn)
    }

    pub(crate) fn sort_by_scores<F>(&mut self, mut score: F)
    where
        F: FnMut(PackedTurn) -> i32,
    {
        for entry in &mut self.entries[..usize::from(self.len)] {
            entry.score = score(entry.turn);
        }
        self.entries[..usize::from(self.len)]
            .sort_unstable_by_key(|entry| std::cmp::Reverse(entry.score));
    }

    pub(crate) fn truncate_for_search(&mut self, maximum: usize) {
        if self.len() > maximum {
            self.len = maximum as u16;
        }
    }

    pub(crate) fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(PackedTurn) -> bool,
    {
        let mut kept = 0;
        for index in 0..self.len() {
            let entry = self.entries[index];
            if keep(entry.turn) {
                self.entries[kept] = entry;
                kept += 1;
            }
        }
        self.len = kept as u16;
    }
}

impl GeneratedMoves {
    fn clear(&mut self) {
        self.moves.clear();
        self.fallback = None;
        self.legal_count = 0;
    }
}

#[inline]
pub(crate) const fn player_sign(player: Player) -> i32 {
    match player {
        Player::Alpha => 1,
        Player::Beta => -1,
    }
}

#[inline]
const fn player_bit(player: Player) -> u8 {
    match player {
        Player::Alpha => 1,
        Player::Beta => 2,
    }
}

#[inline]
fn is_retreater(actor: usize) -> bool {
    RETREATERS & (1 << actor) != 0
}

#[inline]
fn legal_retreater_drop(player: Player, destination: usize) -> bool {
    match player {
        Player::Alpha => destination <= 3,
        Player::Beta => destination >= 2,
    }
}

#[inline]
fn advance(source: usize, player: Player) -> Option<usize> {
    match player {
        Player::Alpha => (source < 5).then_some(source + 1),
        Player::Beta => source.checked_sub(1),
    }
}

#[inline]
fn retreat(source: usize, player: Player) -> Option<usize> {
    match player {
        Player::Alpha => source.checked_sub(1),
        Player::Beta => (source < 5).then_some(source + 1),
    }
}

#[inline]
fn activates_triplet(actor: usize, destination_presence: u16) -> bool {
    activation_actor_mask(destination_presence) & (1 << actor) != 0
}

#[inline]
fn activation_actor_mask(destination_presence: u16) -> u16 {
    ACTIVATION_ACTORS[destination_presence as usize]
}

fn activation_missing_roles(actor: usize, destination_presence: u16) -> u8 {
    let with_actor = destination_presence | (1 << actor);
    let actor_bit = 1 << actor;
    let mut minimum = 3;
    for element in 0..4 {
        if (UNARY[element] | BINARY[element] | TERNARY[element]) & actor_bit == 0 {
            continue;
        }
        let missing = u8::from(with_actor & UNARY[element] == 0)
            + u8::from(with_actor & BINARY[element] == 0)
            + u8::from(with_actor & TERNARY[element] == 0);
        minimum = minimum.min(missing);
    }
    minimum
}

const fn build_activation_actor_table() -> [u16; 1 << 16] {
    let mut table = [0; 1 << 16];
    let mut presence = 0usize;
    while presence < (1usize << 16) {
        let bits = presence as u16;
        let mut actors = 0;
        let mut element = 0;
        while element < 4 {
            if bits & BINARY[element] != 0 && bits & TERNARY[element] != 0 {
                actors |= UNARY[element];
            }
            if bits & UNARY[element] != 0 && bits & TERNARY[element] != 0 {
                actors |= BINARY[element];
            }
            if bits & UNARY[element] != 0 && bits & BINARY[element] != 0 {
                actors |= TERNARY[element];
            }
            element += 1;
        }
        table[presence] = actors;
        presence += 1;
    }
    table
}

#[inline]
fn rank(index: usize) -> Rank {
    match index {
        0 => Rank::R1,
        1 => Rank::R2,
        2 => Rank::R3,
        3 => Rank::R4,
        4 => Rank::R5,
        5 => Rank::R6,
        _ => unreachable!("rank index out of range"),
    }
}

#[inline]
fn rank_index(rank: Rank) -> usize {
    match rank {
        Rank::R1 => 0,
        Rank::R2 => 1,
        Rank::R3 => 2,
        Rank::R4 => 3,
        Rank::R5 => 4,
        Rank::R6 => 5,
    }
}

#[inline]
fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::Analyzer;
    use snipe_prng::{initial_state, splitmix64};

    #[test]
    fn packed_generator_and_apply_match_core_across_rollouts() {
        let mut chooser = 1u64;
        for seed in 0..32 {
            let mut core = initial_state(seed);
            for _ in 0..96 {
                let packed = PackedState::from_core(&core);
                let mut core_actions = Vec::new();
                core.write_legal_actions(&mut core_actions);
                let mut packed_actions = MoveList::default();
                packed.write_legal_actions(&mut packed_actions);
                let converted: Vec<_> = (0..packed_actions.len())
                    .map(|index| packed.to_core_action(packed_actions.get(index)))
                    .collect();
                assert_eq!(converted.len(), core_actions.len(), "{core:?}");
                for action in &core_actions {
                    assert!(converted.contains(action), "missing {action:?} in {core:?}");
                }
                if core_actions.is_empty() {
                    break;
                }

                chooser = splitmix64(chooser);
                let index = chooser as usize % core_actions.len();
                let action = core_actions[index];
                let packed_action = (0..packed_actions.len())
                    .map(|i| packed_actions.get(i))
                    .find(|&candidate| packed.to_core_action(candidate) == action)
                    .unwrap();
                let expected = core.clone().apply(action).unwrap();
                assert_eq!(
                    packed.apply(packed_action),
                    PackedState::from_core(&expected)
                );
                core = expected;
                if core.winner().is_some() {
                    break;
                }
            }
        }
    }

    #[test]
    fn reasonable_generator_only_omits_proven_losses() {
        let mut chooser = 9u64;
        for seed in 0..64 {
            let mut core = initial_state(seed);
            for _ in 0..96 {
                let packed = PackedState::from_core(&core);
                let mut legal = MoveList::default();
                packed.write_legal_actions(&mut legal);
                let mut reasonable = GeneratedMoves::default();
                packed.write_reasonable_actions(&mut reasonable);
                for index in 0..reasonable.moves.len() {
                    assert!(legal.contains(reasonable.moves.get(index)));
                }
                for index in 0..legal.len() {
                    let action = legal.get(index);
                    if !reasonable.moves.contains(action) {
                        assert!(packed.action_is_unreasonable(action));
                        let mover = core.active_player;
                        let child = core
                            .clone()
                            .apply(packed.to_core_action(action))
                            .expect("generated action is legal");
                        assert!(
                            child.winner() == Some(mover.opponent())
                                || opponent_has_winning_turn(&child, mover.opponent()),
                            "omitted action is not a forced loss: {:?} in {core:?}",
                            packed.to_core_action(action),
                        );
                    }
                }
                if legal.is_empty() {
                    break;
                }
                chooser = splitmix64(chooser);
                let action = legal.get(chooser as usize % legal.len());
                core = core
                    .apply(packed.to_core_action(action))
                    .expect("packed action is legal");
                if core.winner().is_some() {
                    break;
                }
            }
        }
    }

    fn opponent_has_winning_turn(state: &State, opponent: Player) -> bool {
        if state.active_player != opponent {
            return false;
        }
        let mut first_actions = Vec::new();
        state.write_legal_actions(&mut first_actions);
        for first in first_actions {
            let after_first = state.clone().apply(first).unwrap();
            if after_first.winner() == Some(opponent) {
                return true;
            }
            if after_first.active_player != opponent {
                continue;
            }
            let mut second_actions = Vec::new();
            after_first.write_legal_actions(&mut second_actions);
            for second in second_actions {
                if after_first.clone().apply(second).unwrap().winner() == Some(opponent) {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn detects_two_animal_snipe_attack_and_the_drop_that_sets_it_up() {
        let before_dragon_drop = State {
            active_player: Player::Beta,
            reserves: cards(&[
                (Card::Animal(Animal::Dragon), Player::Alpha),
                (Card::Animal(Animal::Horse), Player::Alpha),
            ]),
            r1: CardMultiset::EMPTY,
            r2: CardMultiset::EMPTY,
            r3: CardMultiset::EMPTY,
            r4: CardMultiset::EMPTY,
            r5: cards(&[
                (Card::Animal(Animal::Fish), Player::Beta),
                (Card::Animal(Animal::Monkey), Player::Alpha),
            ]),
            r6: cards(&[
                (Card::Animal(Animal::Frog), Player::Alpha),
                (Card::Animal(Animal::Dog), Player::Beta),
                (Card::Snipe, Player::Beta),
            ]),
            leading_action: None,
        };
        let packed = PackedState::from_core(&before_dragon_drop);
        assert!(packed.snipe_setup_drop_count(Player::Beta) >= 1);

        let after_dragon_drop = State {
            active_player: Player::Beta,
            reserves: cards(&[(Card::Animal(Animal::Horse), Player::Alpha)]),
            r5: cards(&[
                (Card::Animal(Animal::Fish), Player::Beta),
                (Card::Animal(Animal::Monkey), Player::Alpha),
                (Card::Animal(Animal::Dragon), Player::Alpha),
            ]),
            ..before_dragon_drop
        };
        let packed = PackedState::from_core(&after_dragon_drop);
        assert!(!packed.rank_is_pressed_by(Player::Alpha, 5));
        assert!(packed.player_can_capture_snipe_this_turn(Player::Alpha, Player::Beta));
        assert!(packed.active_snipe_is_pressed());
    }

    #[test]
    fn official_seed_zero_cherry_threat_is_prepruned() {
        use StepDirection::{Advance, Retreat};

        let animal = |actor, direction, destination| {
            Action::AnimalStep(AnimalStep {
                actor,
                direction,
                destination,
            })
        };
        let drop = |actor, destination| Action::Drop(AnimalDrop { actor, destination });
        let snipe = |destination| Action::SnipeStep(SnipeStep { destination });

        let transcript = [
            vec![
                animal(Animal::Ox, Advance, Rank::R4),
                animal(Animal::Fish, Advance, Rank::R4),
            ],
            vec![
                animal(Animal::Ox, Advance, Rank::R3),
                animal(Animal::Dragon, Advance, Rank::R3),
            ],
            vec![drop(Animal::Ox, Rank::R2)],
            vec![
                animal(Animal::Monkey, Advance, Rank::R2),
                animal(Animal::Rabbit, Advance, Rank::R3),
            ],
            vec![drop(Animal::Elephant, Rank::R6)],
            vec![
                animal(Animal::Rooster, Advance, Rank::R2),
                animal(Animal::Rabbit, Retreat, Rank::R2),
            ],
            vec![
                animal(Animal::Squid, Advance, Rank::R4),
                animal(Animal::Fish, Advance, Rank::R3),
            ],
            vec![drop(Animal::Fish, Rank::R6)],
            vec![
                animal(Animal::Mouse, Advance, Rank::R4),
                animal(Animal::Squid, Advance, Rank::R3),
            ],
            vec![drop(Animal::Monkey, Rank::R5)],
            vec![
                animal(Animal::Boar, Advance, Rank::R4),
                animal(Animal::Mouse, Advance, Rank::R3),
            ],
            vec![drop(Animal::Ox, Rank::R6)],
            vec![snipe(Rank::R5)],
            vec![drop(Animal::Boar, Rank::R4)],
            vec![
                animal(Animal::Tiger, Advance, Rank::R4),
                animal(Animal::Mouse, Retreat, Rank::R6),
            ],
            vec![drop(Animal::Tiger, Rank::R5)],
            vec![
                animal(Animal::Horse, Advance, Rank::R4),
                animal(Animal::Squid, Advance, Rank::R2),
            ],
            vec![drop(Animal::Squid, Rank::R3)],
            vec![snipe(Rank::R6)],
            vec![
                animal(Animal::Tiger, Advance, Rank::R6),
                animal(Animal::Monkey, Advance, Rank::R6),
            ],
            vec![snipe(Rank::R5)],
            vec![drop(Animal::Ram, Rank::R4)],
            vec![drop(Animal::Ox, Rank::R2)],
            vec![
                animal(Animal::Rabbit, Advance, Rank::R3),
                animal(Animal::Ram, Advance, Rank::R5),
            ],
            vec![drop(Animal::Fish, Rank::R1)],
            vec![snipe(Rank::R2)],
            vec![drop(Animal::Fish, Rank::R3)],
        ];
        let mut state = initial_state(0);
        let mut before_snipe_step = None;
        for (turn_index, turn) in transcript.into_iter().enumerate() {
            for action in turn {
                state = state.apply(action).unwrap_or_else(|error| {
                    panic!(
                        "recorded action in turn {} must be legal: {action:?}: {error:?}",
                        turn_index + 1
                    )
                });
            }
            if turn_index + 1 == 25 {
                before_snipe_step = Some(state.clone());
            }
        }

        let packed = PackedState::from_core(&state);
        assert_eq!(packed.active, Player::Alpha);
        assert!(packed.player_can_capture_snipe_this_turn(Player::Beta, Player::Alpha));
        assert!(packed.active_snipe_is_pressed());

        let losing_turn = PackedTurn::pair(
            PackedMove::animal(Animal::Rabbit as usize, false, 3),
            PackedMove::animal(Animal::Squid as usize, false, 3),
        );
        let mut reasonable = TurnList::default();
        packed.write_reasonable_turns(&mut reasonable);
        assert!(reasonable.is_empty());
        assert!(!reasonable.contains(losing_turn));
        let after_losing_turn = state
            .clone()
            .apply(animal(Animal::Rabbit, Advance, Rank::R4))
            .unwrap()
            .apply(animal(Animal::Squid, Advance, Rank::R4))
            .unwrap();
        assert!(opponent_has_winning_turn(&after_losing_turn, Player::Beta));

        let mut analyzer = crate::v1::DumplingV1Analyzer::new();
        analyzer.set_state(state.clone());
        for tick in 0..=16 {
            if tick != 0 {
                analyzer.think_for_one_tick();
            }
            let mut line = Vec::new();
            analyzer.write_optimal_lop(&mut line);
            let mut chosen = state.clone();
            for &action in &line {
                let player = chosen.active_player;
                chosen = chosen.apply(action).unwrap();
                if chosen.active_player != player || chosen.winner().is_some() {
                    break;
                }
            }
            assert!(
                chosen.active_player == Player::Beta || chosen.winner().is_some(),
                "Dumpling emitted an incomplete fallback turn after tick {tick}: {line:?}"
            );
        }

        let before_snipe_step = before_snipe_step.unwrap();
        let after_snipe_step = before_snipe_step.clone().apply(snipe(Rank::R2)).unwrap();
        assert!(PackedState::from_core(&after_snipe_step).active_has_mating_setup_drop());

        let before_snipe_step = PackedState::from_core(&before_snipe_step);
        let mut alpha_turns = TurnList::default();
        before_snipe_step.write_reasonable_turns(&mut alpha_turns);
        let mut survives_reply = 0;
        for alpha_index in 0..alpha_turns.len() {
            let after_alpha = before_snipe_step.apply_turn(alpha_turns.get(alpha_index));
            let mut beta_turns = TurnList::default();
            after_alpha.write_reasonable_turns(&mut beta_turns);
            let beta_has_forcing_reply = (0..beta_turns.len()).any(|beta_index| {
                let after_beta = after_alpha.apply_turn(beta_turns.get(beta_index));
                if after_beta.captured_winner() == Some(Player::Beta) {
                    return true;
                }
                let mut alpha_evasions = TurnList::default();
                after_beta.write_reasonable_turns(&mut alpha_evasions);
                alpha_evasions.is_empty()
            });
            survives_reply += usize::from(!beta_has_forcing_reply);
        }
        assert_eq!(alpha_turns.len(), 1);
        assert_eq!(survives_reply, 0);
    }

    fn cards(entries: &[(Card, Player)]) -> CardMultiset {
        entries
            .iter()
            .fold(CardMultiset::EMPTY, |cards, &(card, player)| {
                cards
                    .checked_add(CardMultiset::singleton(card, player))
                    .expect("test position has legal multiplicities")
            })
    }

    #[test]
    fn fixed_move_buffer_covers_the_maximal_drop_position() {
        let state = initial_state(7_071);
        let packed = PackedState::from_core(&state);
        let mut actions = MoveList::default();
        packed.write_legal_actions(&mut actions);
        assert!(actions.len() <= MAX_ACTIONS);
    }
}
