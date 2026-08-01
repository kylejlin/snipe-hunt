use crate::position::{Position, Turn, activates, advance, retreat};
use snipe_core::Player;
use std::{cmp::Reverse, collections::HashMap, sync::Arc};

const SCOUT_QUIET_BEAM: usize = 12;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TacticalProfile {
    pub(crate) pressed: bool,
    pub(crate) immediate_witnesses: u16,
    pub(crate) near_witnesses: u16,
    pub(crate) controlled_ranks: u8,
    pub(crate) safe_exits: u8,
}

#[derive(Default)]
pub(crate) struct Tactics {
    turns: HashMap<Position, Arc<[Turn]>>,
    direct: HashMap<(Position, Player), Option<Turn>>,
    profiles: HashMap<(Position, Player), TacticalProfile>,
    exact_attacks: HashMap<(Position, Player), Arc<[Turn]>>,
    root_scouts: HashMap<(Position, Player), Arc<[Turn]>>,
}

impl Tactics {
    pub(crate) fn turns(&mut self, position: Position) -> Arc<[Turn]> {
        if let Some(turns) = self.turns.get(&position) {
            return Arc::clone(turns);
        }
        let turns: Arc<[Turn]> = position.turns().into();
        self.turns.insert(position, Arc::clone(&turns));
        turns
    }

    /// Returns a concrete full-ply snipe capture. The caller may ask about
    /// either player; tactical pressure is a board fact, not a turn-order fact.
    pub(crate) fn direct_capture(&mut self, position: Position, attacker: Player) -> Option<Turn> {
        let mut probe = position;
        probe.active = attacker;
        probe.leading = None;
        if let Some(cached) = self.direct.get(&(probe, attacker)) {
            return *cached;
        }
        let victim = attacker.opponent();
        let found = self.turns(probe).iter().copied().find(|turn| {
            turn.next.cells[0].has_snipe(victim) && turn.next.captured_winner() == Some(attacker)
        });
        self.direct.insert((probe, attacker), found);
        found
    }

    pub(crate) fn profile(&mut self, position: Position, attacker: Player) -> TacticalProfile {
        let mut canonical = position;
        canonical.leading = None;
        if let Some(profile) = self.profiles.get(&(canonical, attacker)) {
            return *profile;
        }
        let victim = attacker.opponent();
        let pressed = self.direct_capture(canonical, attacker).is_some();
        let mut profile = local_profile(canonical, attacker, victim);
        profile.pressed = pressed;
        self.profiles.insert((canonical, attacker), profile);
        profile
    }

    /// Iceberg's attacking vocabulary. Every legal turn is retained so bounded
    /// disproofs remain exact; captures, checks, and pressure-building turns
    /// rise to the front where proof-number search will see them first.
    pub(crate) fn attacking_turns(&mut self, position: Position, attacker: Player) -> Arc<[Turn]> {
        let turns = self.turns(position);
        let before = self.profile(position, attacker);
        let mut scored = Vec::with_capacity(turns.len());
        for turn in turns.iter().copied() {
            if turn.next.captured_winner() == Some(attacker) {
                scored.push((i32::MAX, turn));
                continue;
            }
            let after = self.profile(turn.next, attacker);
            let pressure_gain =
                i32::from(after.immediate_witnesses) - i32::from(before.immediate_witnesses);
            let near_gain = i32::from(after.near_witnesses) - i32::from(before.near_witnesses);
            let control_gain =
                i32::from(after.controlled_ranks) - i32::from(before.controlled_ranks);
            let exit_gain = i32::from(before.safe_exits) - i32::from(after.safe_exits);
            let tactical = i32::from(after.pressed) * 2_000_000
                + i32::from(turn.capture_count) * 120_000
                + pressure_gain * 30_000
                + near_gain * 8_000
                + control_gain * 4_000
                + exit_gain * 15_000
                + turn.order_score;
            scored.push((tactical, turn));
        }
        scored.sort_unstable_by_key(|(score, _)| Reverse(*score));
        let result: Arc<[Turn]> = scored
            .into_iter()
            .map(|(_, turn)| turn)
            .collect::<Vec<_>>()
            .into();
        result
    }

    /// A deliberately incomplete attacking set used only to discover sound
    /// upper-bound mate proofs. Exact shortest-mate searches use
    /// `attacking_turns`; the scout may miss a mate but can never invent one.
    pub(crate) fn scouting_turns(&mut self, position: Position, attacker: Player) -> Arc<[Turn]> {
        let ordered = self.attacking_turns(position, attacker);
        let before = self.profile(position, attacker);
        let mut tactical = Vec::new();
        let mut quiet = Vec::new();
        for turn in ordered.iter().copied() {
            if turn.next.captured_winner() == Some(attacker) {
                tactical.push(turn);
                continue;
            }
            let after = self.profile(turn.next, attacker);
            if turn.capture_count != 0
                || after.pressed
                || after.immediate_witnesses > before.immediate_witnesses
                || after.near_witnesses > before.near_witnesses
                || after.controlled_ranks > before.controlled_ranks
                || after.safe_exits < before.safe_exits
            {
                tactical.push(turn);
            } else if quiet.len() < SCOUT_QUIET_BEAM {
                quiet.push(turn);
            }
        }
        tactical.extend(quiet);
        tactical.into()
    }

    /// Expensive root-only ordering for checking candidates. It looks through
    /// every legal reply and asks how much forcing pressure the attacker can
    /// recover on the following ply. This encodes the useful human distinction
    /// between a decorative check and a check whose capture and non-capture
    /// evasions are both met by another threat. A modest tempo bonus favors a
    /// two-action check over one that unnecessarily forfeits the second action.
    pub(crate) fn root_scouting_turns(
        &mut self,
        position: Position,
        attacker: Player,
    ) -> Arc<[Turn]> {
        if let Some(turns) = self.root_scouts.get(&(position, attacker)) {
            return Arc::clone(turns);
        }
        let turns = self.scouting_turns(position, attacker);
        let mut scored = Vec::with_capacity(turns.len());
        for (rank, turn) in turns.iter().copied().enumerate() {
            let profile = self.profile(turn.next, attacker);
            let forcing = if profile.pressed {
                self.reply_coverage_score(turn.next, attacker)
            } else {
                i32::MIN / 2
            };
            let forcing =
                forcing.saturating_add(i32::from(turn.second.is_some()).saturating_mul(250_000));
            scored.push((profile.pressed, forcing, rank, turn));
        }
        scored.sort_unstable_by_key(|(pressed, forcing, rank, _)| {
            (Reverse(*pressed), Reverse(*forcing), *rank)
        });
        let ordered: Arc<[Turn]> = scored
            .into_iter()
            .map(|(_, _, _, turn)| turn)
            .collect::<Vec<_>>()
            .into();
        self.root_scouts
            .insert((position, attacker), Arc::clone(&ordered));
        ordered
    }

    /// Complete attacker ordering for shortestness proofs. Unlike the scout,
    /// this scores every child using only local pressure geometry; recursively
    /// enumerating a full capture witness for every legal turn would cost more
    /// than the exact search itself.
    pub(crate) fn exact_attacking_turns(
        &mut self,
        position: Position,
        attacker: Player,
    ) -> Arc<[Turn]> {
        if let Some(turns) = self.exact_attacks.get(&(position, attacker)) {
            return Arc::clone(turns);
        }
        let victim = attacker.opponent();
        let before = local_profile(position, attacker, victim);
        let turns = self.turns(position);
        let mut scored = turns
            .iter()
            .copied()
            .map(|turn| {
                let after = local_profile(turn.next, attacker, victim);
                let score = i32::from(turn.next.captured_winner() == Some(attacker)) * 4_000_000
                    + i32::from(turn.capture_count) * 120_000
                    + (i32::from(after.immediate_witnesses)
                        - i32::from(before.immediate_witnesses))
                        * 30_000
                    + (i32::from(after.near_witnesses) - i32::from(before.near_witnesses)) * 8_000
                    + (i32::from(after.controlled_ranks) - i32::from(before.controlled_ranks))
                        * 4_000
                    + (i32::from(before.safe_exits) - i32::from(after.safe_exits)) * 15_000
                    + turn.order_score;
                (score, turn)
            })
            .collect::<Vec<_>>();
        scored.sort_unstable_by_key(|(score, _)| Reverse(*score));
        let ordered: Arc<[Turn]> = scored
            .into_iter()
            .map(|(_, turn)| turn)
            .collect::<Vec<_>>()
            .into();
        self.exact_attacks
            .insert((position, attacker), Arc::clone(&ordered));
        ordered
    }

    pub(crate) fn retained_entries(&self) -> usize {
        self.turns.len()
            + self.direct.len()
            + self.profiles.len()
            + self.exact_attacks.len()
            + self.root_scouts.len()
    }

    fn reply_coverage_score(&mut self, checked: Position, attacker: Player) -> i32 {
        let victim = attacker.opponent();
        let replies = self.turns(checked);
        let mut worst = i32::MAX;
        for reply in replies.iter().copied() {
            if let Some(winner) = reply.next.winner() {
                let score = if winner == attacker {
                    4_000_000
                } else {
                    -4_000_000
                };
                worst = worst.min(score);
                continue;
            }
            let immediate = local_profile(reply.next, attacker, victim);
            let mut best = geometry_score(immediate)
                .saturating_add(i32::from(immediate.immediate_witnesses != 0) * 500_000);
            for continuation in self.turns(reply.next).iter().copied() {
                let score = if continuation.next.captured_winner() == Some(attacker) {
                    4_000_000
                } else {
                    geometry_score(local_profile(continuation.next, attacker, victim))
                        .saturating_add(i32::from(continuation.capture_count) * 40_000)
                };
                best = best.max(score);
            }
            worst = worst.min(best);
        }
        worst
    }
}

fn geometry_score(profile: TacticalProfile) -> i32 {
    i32::from(profile.immediate_witnesses) * 1_000_000
        + i32::from(profile.near_witnesses) * 30_000
        + i32::from(profile.controlled_ranks) * 5_000
        - i32::from(profile.safe_exits) * 20_000
}

fn local_profile(position: Position, attacker: Player, victim: Player) -> TacticalProfile {
    let Some(snipe) = position.snipe_location(victim) else {
        return TacticalProfile {
            pressed: true,
            ..TacticalProfile::default()
        };
    };
    let target_presence = position.cells[snipe].presence();
    let mut immediate_witnesses = 0_u16;
    let mut near_witnesses = 0_u16;
    for source in [retreat(attacker, snipe), advance(attacker, snipe)]
        .into_iter()
        .flatten()
    {
        let retreating = advance(attacker, snipe) == Some(source);
        let mut animals = position.cells[source].owned_presence(attacker);
        while animals != 0 {
            let animal = animals.trailing_zeros() as usize;
            animals &= animals - 1;
            if retreating && !crate::position::ANIMALS[animal].is_retreater() {
                continue;
            }
            let missing = missing_roles(animal, target_presence);
            if missing == 0 {
                immediate_witnesses = immediate_witnesses.saturating_add(1);
            } else if missing == 1 {
                near_witnesses = near_witnesses.saturating_add(1);
            }
        }
    }

    let mut controlled_ranks = 0_u8;
    for rank in 1..=6 {
        if rank_has_single_step_capture(position, attacker, rank) {
            controlled_ranks += 1;
        }
    }
    let safe_exits = [advance(victim, snipe), retreat(victim, snipe)]
        .into_iter()
        .flatten()
        .filter(|&destination| !rank_has_single_step_capture(position, attacker, destination))
        .count() as u8;
    TacticalProfile {
        pressed: false,
        immediate_witnesses,
        near_witnesses,
        controlled_ranks,
        safe_exits,
    }
}

fn rank_has_single_step_capture(position: Position, attacker: Player, target: usize) -> bool {
    let presence = position.cells[target].presence();
    for source in [retreat(attacker, target), advance(attacker, target)]
        .into_iter()
        .flatten()
    {
        let retreating = advance(attacker, target) == Some(source);
        let mut animals = position.cells[source].owned_presence(attacker);
        while animals != 0 {
            let animal = animals.trailing_zeros() as usize;
            animals &= animals - 1;
            if (!retreating || crate::position::ANIMALS[animal].is_retreater())
                && activates(animal, presence)
            {
                return true;
            }
        }
    }
    false
}

fn missing_roles(actor: usize, destination: u16) -> u8 {
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
    let actor_mask = 1_u16 << actor;
    let animals = destination | actor_mask;
    ROLE_MASKS
        .iter()
        .filter(|roles| roles.iter().any(|role| role & actor_mask != 0))
        .map(|roles| roles.iter().filter(|role| **role & animals == 0).count() as u8)
        .min()
        .unwrap_or(3)
}
