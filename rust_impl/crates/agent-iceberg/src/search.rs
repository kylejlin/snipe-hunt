use crate::{
    position::{Position, Turn},
    tactics::Tactics,
};
use snipe_core::{Action, Player};
use std::{collections::HashMap, sync::Arc};

const INF: u64 = 1_u64 << 60;
const EXPANSIONS_PER_TICK: usize = 24;
const EXACT_NODES_PER_TICK: usize = 4_000;
const OR_ORDER_BIAS: u64 = 1_024;

#[derive(Clone)]
struct Resolution {
    target_wins: bool,
    distance: u8,
    line: Arc<[Action]>,
}

#[derive(Clone)]
enum ExactResult {
    Win(Resolution),
    Loss,
}

#[derive(Default)]
struct ExactKnowledge {
    /// A concrete win of this length works at every equal or larger horizon.
    win: Option<Resolution>,
    /// No win exists at any horizon up to and including this one.
    loss_through: Option<u8>,
}

enum Attempt {
    Win(Resolution),
    Loss,
    Unknown,
}

struct Edge {
    turn: Turn,
    child: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct PreferredTurn {
    first: Action,
    second: Option<Action>,
}

struct Node {
    position: Position,
    plies_left: u8,
    proof: u64,
    disproof: u64,
    expanded: bool,
    children: Vec<Edge>,
    resolution: Option<Resolution>,
}

pub(crate) struct ProofSearch {
    pub(crate) target: Player,
    pub(crate) bound: u8,
    nodes: Vec<Node>,
    table: HashMap<(Position, u8), usize>,
    expansions: u64,
    selective: bool,
    preferred: HashMap<Position, PreferredTurn>,
}

pub(crate) struct ExactSearch {
    root: Position,
    pub(crate) target: Player,
    pub(crate) bound: u8,
    cache: HashMap<Position, ExactKnowledge>,
    preferred: Arc<HashMap<Position, PreferredTurn>>,
    result: Option<ExactResult>,
    visited: u64,
}

impl ExactSearch {
    pub(crate) fn new(
        root: Position,
        target: Player,
        bound: u8,
        preferred: Option<Arc<HashMap<Position, PreferredTurn>>>,
    ) -> Self {
        Self {
            root,
            target,
            bound,
            cache: HashMap::new(),
            preferred: preferred.unwrap_or_else(|| Arc::new(HashMap::new())),
            result: None,
            visited: 0,
        }
    }

    pub(crate) fn tick(&mut self, tactics: &mut Tactics) {
        if self.result.is_some() {
            return;
        }
        let mut budget = EXACT_NODES_PER_TICK;
        let attempt = self.solve(self.root, self.bound, tactics, &mut budget);
        self.visited += (EXACT_NODES_PER_TICK - budget) as u64;
        self.result = match attempt {
            Attempt::Win(resolution) => Some(ExactResult::Win(resolution)),
            Attempt::Loss => Some(ExactResult::Loss),
            Attempt::Unknown => None,
        };
    }

    pub(crate) fn restart(
        &mut self,
        root: Position,
        bound: u8,
        preferred: Option<Arc<HashMap<Position, PreferredTurn>>>,
    ) {
        self.root = root;
        self.bound = bound;
        self.preferred = preferred.unwrap_or_else(|| Arc::new(HashMap::new()));
        self.result = None;
    }

    pub(crate) fn proved_line(&self) -> Option<(u8, Arc<[Action]>)> {
        match self.result.as_ref()? {
            ExactResult::Win(resolution) => {
                Some((resolution.distance, Arc::clone(&resolution.line)))
            }
            ExactResult::Loss => None,
        }
    }

    pub(crate) fn is_resolved(&self) -> bool {
        self.result.is_some()
    }

    pub(crate) fn retained_entries(&self) -> usize {
        self.cache.len() + self.preferred.len()
    }

    pub(crate) fn diagnostics(&self) -> String {
        let status = match self.result {
            Some(ExactResult::Win(_)) => "win",
            Some(ExactResult::Loss) => "loss",
            None => "open",
        };
        format!(
            "{:?}@{} {status} cache={} visited={}",
            self.target,
            self.bound,
            self.cache.len(),
            self.visited,
        )
    }

    fn solve(
        &mut self,
        position: Position,
        plies_left: u8,
        tactics: &mut Tactics,
        budget: &mut usize,
    ) -> Attempt {
        if let Some(cached) = self.cache.get(&position) {
            if let Some(resolution) = cached
                .win
                .as_ref()
                .filter(|resolution| resolution.distance <= plies_left)
            {
                return Attempt::Win(resolution.clone());
            }
            if cached
                .loss_through
                .is_some_and(|horizon| horizon >= plies_left)
            {
                return Attempt::Loss;
            }
        }
        if *budget == 0 {
            return Attempt::Unknown;
        }
        *budget -= 1;

        if let Some(winner) = position.winner() {
            let result = if winner == self.target {
                ExactResult::Win(Resolution {
                    target_wins: true,
                    distance: 0,
                    line: Arc::from([]),
                })
            } else {
                ExactResult::Loss
            };
            return self.remember(position, plies_left, result);
        }
        if plies_left == 0 {
            return self.remember(position, plies_left, ExactResult::Loss);
        }
        if position.active == self.target {
            if let Some(turn) = tactics.direct_capture(position, self.target) {
                let resolution = Resolution {
                    target_wins: true,
                    distance: 1,
                    line: prepend(turn, &[]),
                };
                return self.remember(position, plies_left, ExactResult::Win(resolution));
            }
        } else if tactics.direct_capture(position, position.active).is_some() {
            return self.remember(position, plies_left, ExactResult::Loss);
        }

        let is_or = position.active == self.target;
        let turns = if is_or {
            tactics.exact_attacking_turns(position, self.target)
        } else {
            tactics.turns(position)
        };
        if turns.is_empty() {
            return self.remember(position, plies_left, ExactResult::Loss);
        }
        let order = ordered_indices(&turns, self.preferred.get(&position));
        if is_or {
            let mut unknown = false;
            for index in order {
                let turn = turns[index];
                match self.solve(turn.next, plies_left - 1, tactics, budget) {
                    Attempt::Win(child) => {
                        let resolution = Resolution {
                            target_wins: true,
                            distance: child.distance.saturating_add(1),
                            line: prepend(turn, &child.line),
                        };
                        return self.remember(position, plies_left, ExactResult::Win(resolution));
                    }
                    Attempt::Loss => {}
                    Attempt::Unknown => unknown = true,
                }
            }
            if unknown {
                Attempt::Unknown
            } else {
                self.remember(position, plies_left, ExactResult::Loss)
            }
        } else {
            let mut unknown = false;
            let mut longest: Option<(Turn, Resolution)> = None;
            for index in order {
                let turn = turns[index];
                match self.solve(turn.next, plies_left - 1, tactics, budget) {
                    Attempt::Loss => {
                        return self.remember(position, plies_left, ExactResult::Loss);
                    }
                    Attempt::Win(child) => {
                        if longest
                            .as_ref()
                            .is_none_or(|(_, old)| child.distance > old.distance)
                        {
                            longest = Some((turn, child));
                        }
                    }
                    Attempt::Unknown => unknown = true,
                }
            }
            if unknown {
                Attempt::Unknown
            } else {
                let (turn, child) = longest.expect("a nonterminal AND node has children");
                let resolution = Resolution {
                    target_wins: true,
                    distance: child.distance.saturating_add(1),
                    line: prepend(turn, &child.line),
                };
                self.remember(position, plies_left, ExactResult::Win(resolution))
            }
        }
    }

    fn remember(&mut self, position: Position, plies_left: u8, result: ExactResult) -> Attempt {
        let knowledge = self.cache.entry(position).or_default();
        match &result {
            ExactResult::Win(resolution) => {
                if knowledge
                    .win
                    .as_ref()
                    .is_none_or(|old| resolution.distance < old.distance)
                {
                    knowledge.win = Some(resolution.clone());
                }
            }
            ExactResult::Loss => {
                knowledge.loss_through = Some(
                    knowledge
                        .loss_through
                        .map_or(plies_left, |old| old.max(plies_left)),
                );
            }
        }
        match result {
            ExactResult::Win(resolution) => Attempt::Win(resolution),
            ExactResult::Loss => Attempt::Loss,
        }
    }
}

fn ordered_indices(turns: &[Turn], preferred: Option<&PreferredTurn>) -> Vec<usize> {
    let mut indices = (0..turns.len()).collect::<Vec<_>>();
    if let Some(preferred) = preferred
        && let Some(index) = indices.iter().position(|&index| {
            turns[index].first == preferred.first && turns[index].second == preferred.second
        })
    {
        indices.swap(0, index);
    }
    indices
}

impl ProofSearch {
    pub(crate) fn new(
        root: Position,
        target: Player,
        bound: u8,
        tactics: &mut Tactics,
        selective: bool,
        preferred_line: Option<&[Action]>,
    ) -> Self {
        let mut search = Self {
            target,
            bound,
            nodes: Vec::new(),
            table: HashMap::new(),
            expansions: 0,
            selective,
            preferred: preferred_line.map_or_else(HashMap::new, |line| preferred_turns(root, line)),
        };
        search.push_node(root, bound, tactics);
        search
    }

    pub(crate) fn tick(&mut self, tactics: &mut Tactics) {
        for _ in 0..EXPANSIONS_PER_TICK {
            if self.is_resolved() {
                return;
            }
            let mut path = vec![0_usize];
            let mut current = 0_usize;
            loop {
                self.update(current);
                let node = &self.nodes[current];
                if node.resolution.is_some() || !node.expanded {
                    break;
                }
                let is_or = node.position.active == self.target;
                let preferred = self.preferred.get(&node.position);
                let next = preferred
                    .and_then(|preferred| {
                        node.children.iter().find(|edge| {
                            edge.turn.first == preferred.first
                                && edge.turn.second == preferred.second
                                && self.nodes[edge.child].resolution.is_none()
                        })
                    })
                    .or_else(|| {
                        node.children
                            .iter()
                            .enumerate()
                            .min_by_key(|(rank, edge)| {
                                let child = &self.nodes[edge.child];
                                if is_or {
                                    child.proof.saturating_add((*rank as u64).saturating_mul(
                                        if self.selective && current == 0 {
                                            OR_ORDER_BIAS
                                        } else {
                                            0
                                        },
                                    ))
                                } else {
                                    child.disproof
                                }
                            })
                            .map(|(_, edge)| edge)
                    });
                let Some(next) = next else {
                    break;
                };
                current = next.child;
                path.push(current);
            }
            if self.nodes[current].resolution.is_none() && !self.nodes[current].expanded {
                self.expand(current, tactics);
                self.expansions += 1;
            }
            for &index in path.iter().rev() {
                self.update(index);
            }
        }
    }

    pub(crate) fn proved_line(&self) -> Option<(u8, Arc<[Action]>)> {
        self.nodes
            .first()
            .and_then(|node| node.resolution.as_ref())
            .filter(|resolution| resolution.target_wins)
            .map(|resolution| (resolution.distance, Arc::clone(&resolution.line)))
    }

    pub(crate) fn leading_turn(&self) -> Option<Turn> {
        self.nodes
            .first()
            .and_then(|node| node.children.first())
            .map(|edge| edge.turn)
    }

    pub(crate) fn is_resolved(&self) -> bool {
        self.nodes
            .first()
            .is_some_and(|node| node.resolution.is_some())
    }

    pub(crate) fn retained_entries(&self) -> usize {
        self.nodes.len() + self.table.len()
    }

    pub(crate) fn proven_preferences(&self) -> Arc<HashMap<Position, PreferredTurn>> {
        let mut preferred = HashMap::new();
        for node in &self.nodes {
            let Some(resolution) = node
                .resolution
                .as_ref()
                .filter(|resolution| resolution.target_wins)
            else {
                continue;
            };
            let Some(&first) = resolution.line.first() else {
                continue;
            };
            let Some(after_first) = node.position.apply(first) else {
                continue;
            };
            let second = if after_first.active == node.position.active
                && after_first.captured_winner().is_none()
                && after_first.has_legal_action()
            {
                resolution.line.get(1).copied()
            } else {
                None
            };
            preferred
                .entry(node.position)
                .or_insert(PreferredTurn { first, second });
        }
        Arc::new(preferred)
    }

    pub(crate) fn diagnostics(&self) -> String {
        let root = &self.nodes[0];
        format!(
            "{:?}@{} p{} d{} nodes={} solved={} expansions={}",
            self.target,
            self.bound,
            root.proof,
            root.disproof,
            self.nodes.len(),
            root.resolution.is_some(),
            self.expansions,
        )
    }

    fn push_node(&mut self, position: Position, plies_left: u8, tactics: &mut Tactics) -> usize {
        if let Some(&index) = self.table.get(&(position, plies_left)) {
            return index;
        }
        let index = self.nodes.len();
        let resolution = self.initial_resolution(position, plies_left, tactics);
        let (proof, disproof) = match &resolution {
            Some(resolution) if resolution.target_wins => (0, INF),
            Some(_) => (INF, 0),
            None => (1, 1),
        };
        self.nodes.push(Node {
            position,
            plies_left,
            proof,
            disproof,
            expanded: resolution.is_some(),
            children: Vec::new(),
            resolution,
        });
        self.table.insert((position, plies_left), index);
        index
    }

    fn initial_resolution(
        &mut self,
        position: Position,
        plies_left: u8,
        tactics: &mut Tactics,
    ) -> Option<Resolution> {
        if let Some(winner) = position.winner() {
            return Some(Resolution {
                target_wins: winner == self.target,
                distance: 0,
                line: Arc::from([]),
            });
        }
        if plies_left == 0 {
            return Some(Resolution {
                target_wins: false,
                distance: 0,
                line: Arc::from([]),
            });
        }
        if position.active == self.target
            && let Some(turn) = tactics.direct_capture(position, self.target)
        {
            let mut line = Vec::with_capacity(2);
            line.push(turn.first);
            if let Some(second) = turn.second {
                line.push(second);
            }
            return Some(Resolution {
                target_wins: true,
                distance: 1,
                line: line.into(),
            });
        }
        if position.active != self.target
            && tactics.direct_capture(position, position.active).is_some()
        {
            return Some(Resolution {
                target_wins: false,
                distance: 0,
                line: Arc::from([]),
            });
        }
        None
    }

    fn expand(&mut self, index: usize, tactics: &mut Tactics) {
        let position = self.nodes[index].position;
        let plies_left = self.nodes[index].plies_left;
        let is_or = position.active == self.target;
        let turns = if is_or {
            if self.selective {
                if index == 0 {
                    tactics.root_scouting_turns(position, self.target)
                } else {
                    tactics.scouting_turns(position, self.target)
                }
            } else {
                tactics.attacking_turns(position, self.target)
            }
        } else {
            tactics.turns(position)
        };
        let mut edges = Vec::with_capacity(turns.len());
        for turn in turns.iter().copied() {
            let child = self.push_node(turn.next, plies_left - 1, tactics);
            edges.push(Edge { turn, child });
        }
        self.nodes[index].expanded = true;
        self.nodes[index].children = edges;
        if self.nodes[index].children.is_empty() {
            self.nodes[index].proof = INF;
            self.nodes[index].disproof = 0;
            self.nodes[index].resolution = Some(Resolution {
                target_wins: false,
                distance: 0,
                line: Arc::from([]),
            });
        } else {
            self.update(index);
        }
    }

    fn update(&mut self, index: usize) {
        if self.nodes[index].resolution.is_some() || !self.nodes[index].expanded {
            return;
        }
        let is_or = self.nodes[index].position.active == self.target;
        if is_or {
            let mut proof = INF;
            let mut disproof = 0_u64;
            let mut proved: Option<(Turn, usize, u8)> = None;
            for edge in &self.nodes[index].children {
                let child = &self.nodes[edge.child];
                proof = proof.min(child.proof);
                disproof = disproof.saturating_add(child.disproof).min(INF);
                if child.proof == 0 {
                    let distance = child
                        .resolution
                        .as_ref()
                        .map_or(u8::MAX, |resolution| resolution.distance);
                    if proved.is_none_or(|(_, _, best_distance)| distance < best_distance) {
                        proved = Some((edge.turn, edge.child, distance));
                    }
                }
            }
            self.nodes[index].proof = proof;
            self.nodes[index].disproof = disproof;
            if proof == 0 {
                let (turn, child, child_distance) =
                    proved.expect("a proved OR node has a proved child");
                let line = prepend(turn, &self.nodes[child].resolution.as_ref().unwrap().line);
                self.nodes[index].resolution = Some(Resolution {
                    target_wins: true,
                    distance: child_distance.saturating_add(1),
                    line,
                });
            } else if disproof == 0 {
                self.nodes[index].resolution = Some(Resolution {
                    target_wins: false,
                    distance: 0,
                    line: Arc::from([]),
                });
            }
        } else {
            let mut proof = 0_u64;
            let mut disproof = INF;
            let mut longest: Option<(Turn, usize, u8)> = None;
            for edge in &self.nodes[index].children {
                let child = &self.nodes[edge.child];
                proof = proof.saturating_add(child.proof).min(INF);
                disproof = disproof.min(child.disproof);
                if let Some(resolution) = child.resolution.as_ref()
                    && resolution.target_wins
                    && longest.is_none_or(|(_, _, distance)| resolution.distance > distance)
                {
                    longest = Some((edge.turn, edge.child, resolution.distance));
                }
            }
            self.nodes[index].proof = proof;
            self.nodes[index].disproof = disproof;
            if proof == 0 {
                let (turn, child, child_distance) =
                    longest.expect("a proved AND node has proved children");
                let line = prepend(turn, &self.nodes[child].resolution.as_ref().unwrap().line);
                self.nodes[index].resolution = Some(Resolution {
                    target_wins: true,
                    distance: child_distance.saturating_add(1),
                    line,
                });
            } else if disproof == 0 {
                self.nodes[index].resolution = Some(Resolution {
                    target_wins: false,
                    distance: 0,
                    line: Arc::from([]),
                });
            }
        }
    }
}

fn preferred_turns(root: Position, line: &[Action]) -> HashMap<Position, PreferredTurn> {
    let mut preferred = HashMap::new();
    let mut position = root;
    let mut index = 0;
    while index < line.len() && position.captured_winner().is_none() {
        let first = line[index];
        let Some(after_first) = position.apply(first) else {
            break;
        };
        index += 1;
        let (second, next) = if after_first.active == position.active
            && after_first.captured_winner().is_none()
            && after_first.has_legal_action()
        {
            let Some(&second) = line.get(index) else {
                break;
            };
            let Some(next) = after_first.apply(second) else {
                break;
            };
            index += 1;
            (Some(second), next)
        } else {
            (None, after_first)
        };
        preferred.insert(position, PreferredTurn { first, second });
        position = next;
    }
    preferred
}

fn prepend(turn: Turn, tail: &[Action]) -> Arc<[Action]> {
    let mut line = Vec::with_capacity(2 + tail.len());
    line.push(turn.first);
    if let Some(second) = turn.second {
        line.push(second);
    }
    line.extend_from_slice(tail);
    line.into()
}
