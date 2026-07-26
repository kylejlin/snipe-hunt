//! Cherry: a rules-only policy/value MCTS agent.
//!
//! The browser uses a frozen checkpoint. The optional `training` feature
//! exposes the small amount of machinery needed by the native self-play
//! trainer; neither human games nor hand-authored Snipe Hunt heuristics are
//! part of the model.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, Card, Evaluation, EvaluationEstimate, Player, Rank,
    State, StepDirection,
};
use std::{collections::HashSet, fs, io, path::Path};

pub const INPUT_SIZE: usize = 263;
pub const HIDDEN_SIZE: usize = 128;
pub const ACTION_SIZE: usize = 294;
const MAGIC: &[u8; 8] = b"CHERRY01";
const MAX_SEARCH_DEPTH: usize = 256;

const W1: usize = 0;
const B1: usize = W1 + INPUT_SIZE * HIDDEN_SIZE;
const WR: usize = B1 + HIDDEN_SIZE;
const BR: usize = WR + HIDDEN_SIZE * HIDDEN_SIZE;
const WP: usize = BR + HIDDEN_SIZE;
const BP: usize = WP + HIDDEN_SIZE * ACTION_SIZE;
const WV: usize = BP + ACTION_SIZE;
const BV: usize = WV + HIDDEN_SIZE;
const PARAM_COUNT: usize = BV + 1;

#[derive(Clone)]
pub struct Model {
    parameters: Vec<f32>,
    pub training_steps: u64,
}

impl Model {
    pub fn embedded() -> Self {
        Self::from_bytes(include_bytes!(concat!(env!("OUT_DIR"), "/cherry.bin")))
            .unwrap_or_else(|_| Self::seeded(0xC4E2_9917_D15C_A11E))
    }

    pub fn seeded(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut parameters = vec![0.0; PARAM_COUNT];
        let input_scale = (2.0 / INPUT_SIZE as f32).sqrt();
        for value in &mut parameters[W1..B1] {
            *value = rng.normalish() * input_scale;
        }
        let hidden_scale = (2.0 / HIDDEN_SIZE as f32).sqrt();
        for value in &mut parameters[WR..BR] {
            *value = rng.normalish() * hidden_scale;
        }
        for value in &mut parameters[WP..BP] {
            *value = rng.normalish() * 0.02;
        }
        for value in &mut parameters[WV..BV] {
            *value = rng.normalish() * hidden_scale;
        }
        Self {
            parameters,
            training_steps: 0,
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        fs::write(path, self.to_bytes())
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_bytes(&fs::read(path)?)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 + self.parameters.len() * 4);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&(INPUT_SIZE as u32).to_le_bytes());
        bytes.extend_from_slice(&(HIDDEN_SIZE as u32).to_le_bytes());
        bytes.extend_from_slice(&(ACTION_SIZE as u32).to_le_bytes());
        bytes.extend_from_slice(&self.training_steps.to_le_bytes());
        for value in &self.parameters {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let header = 8 + 4 + 4 + 4 + 8;
        if bytes.len() != header + PARAM_COUNT * 4 || bytes.get(..8) != Some(MAGIC) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Cherry checkpoint",
            ));
        }
        let read_u32 = |offset: usize| {
            u32::from_le_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("checked length"),
            )
        };
        if read_u32(8) as usize != INPUT_SIZE
            || read_u32(12) as usize != HIDDEN_SIZE
            || read_u32(16) as usize != ACTION_SIZE
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incompatible Cherry checkpoint dimensions",
            ));
        }
        let training_steps = u64::from_le_bytes(bytes[20..28].try_into().expect("checked length"));
        let parameters = bytes[28..]
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
            .collect::<Vec<_>>();
        if parameters.iter().any(|value| !value.is_finite()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "non-finite Cherry checkpoint",
            ));
        }
        Ok(Self {
            parameters,
            training_steps,
        })
    }

    fn forward(&self, input: &[f32]) -> Forward {
        debug_assert_eq!(input.len(), INPUT_SIZE);
        let mut hidden_1 = vec![0.0; HIDDEN_SIZE];
        for (output, hidden) in hidden_1.iter_mut().enumerate() {
            let mut sum = self.parameters[B1 + output];
            for (index, &value) in input.iter().enumerate() {
                sum += value * self.parameters[W1 + index * HIDDEN_SIZE + output];
            }
            *hidden = sum.max(0.0);
        }
        let mut hidden_2 = vec![0.0; HIDDEN_SIZE];
        for output in 0..HIDDEN_SIZE {
            let mut sum = hidden_1[output] + self.parameters[BR + output];
            for (index, &value) in hidden_1.iter().enumerate() {
                sum += value * self.parameters[WR + index * HIDDEN_SIZE + output];
            }
            hidden_2[output] = sum.max(0.0);
        }
        let mut logits = vec![0.0; ACTION_SIZE];
        for (action, logit) in logits.iter_mut().enumerate() {
            let mut sum = self.parameters[BP + action];
            for (index, &value) in hidden_2.iter().enumerate() {
                sum += value * self.parameters[WP + index * ACTION_SIZE + action];
            }
            *logit = sum;
        }
        let mut raw_value = self.parameters[BV];
        for (index, &value) in hidden_2.iter().enumerate() {
            raw_value += value * self.parameters[WV + index];
        }
        Forward {
            hidden_1,
            hidden_2,
            logits,
            value: raw_value.tanh(),
        }
    }

    pub fn predict(&self, state: &State) -> (Vec<f32>, f32) {
        let output = self.forward(&encode_state(state));
        (output.logits, output.value)
    }
}

#[allow(dead_code)]
struct Forward {
    hidden_1: Vec<f32>,
    hidden_2: Vec<f32>,
    logits: Vec<f32>,
    value: f32,
}

struct Edge {
    action: Action,
    prior: f32,
    visits: u32,
    value_sum: f32,
    child: Option<Box<Node>>,
}

struct Node {
    state: State,
    edges: Vec<Edge>,
    expanded: bool,
    value: f32,
}

impl Node {
    fn new(state: State) -> Self {
        Self {
            state,
            edges: Vec::new(),
            expanded: false,
            value: 0.0,
        }
    }

    fn expand(&mut self, model: &Model) -> f32 {
        if let Some(winner) = self.state.winner() {
            self.value = if winner == self.state.active_player {
                1.0
            } else {
                -1.0
            };
            self.expanded = true;
            return self.value;
        }
        let mut legal = Vec::new();
        self.state.write_legal_actions(&mut legal);
        let (logits, value) = model.predict(&self.state);
        let maximum = legal
            .iter()
            .map(|&action| logits[action_index(&self.state, action)])
            .fold(f32::NEG_INFINITY, f32::max);
        let denominator = legal
            .iter()
            .map(|&action| (logits[action_index(&self.state, action)] - maximum).exp())
            .sum::<f32>()
            .max(f32::MIN_POSITIVE);
        self.edges = legal
            .into_iter()
            .map(|action| Edge {
                prior: (logits[action_index(&self.state, action)] - maximum).exp() / denominator,
                action,
                visits: 0,
                value_sum: 0.0,
                child: None,
            })
            .collect();
        self.value = value;
        self.expanded = true;
        value
    }

    fn preferred_edge(&self) -> Option<usize> {
        self.edges
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| {
                left.visits
                    .cmp(&right.visits)
                    .then_with(|| left.prior.total_cmp(&right.prior))
            })
            .map(|(index, _)| index)
    }
}

pub struct Search {
    root: Node,
    simulations: u64,
}

impl Search {
    pub fn new(state: State, model: &Model) -> Self {
        let mut root = Node::new(state);
        root.expand(model);
        Self {
            root,
            simulations: 0,
        }
    }

    pub fn simulate(&mut self, model: &Model) {
        let mut seen = HashSet::new();
        seen.insert(state_fingerprint(&self.root.state));
        simulate_node(&mut self.root, model, 0, &mut seen);
        self.simulations += 1;
    }

    pub fn simulate_n(&mut self, model: &Model, count: usize) {
        for _ in 0..count {
            self.simulate(model);
        }
    }

    pub fn root_value(&self) -> f32 {
        if self.root.edges.is_empty() {
            return self.root.value;
        }
        let visits = self.root.edges.iter().map(|edge| edge.visits).sum::<u32>();
        if visits == 0 {
            self.root.value
        } else {
            self.root
                .edges
                .iter()
                .map(|edge| edge.value_sum)
                .sum::<f32>()
                / visits as f32
        }
    }

    pub fn policy(&self, temperature: f32) -> Vec<(Action, f32)> {
        if self.root.edges.is_empty() {
            return Vec::new();
        }
        if temperature <= 0.01 {
            let best = self.root.preferred_edge().unwrap_or(0);
            return self
                .root
                .edges
                .iter()
                .enumerate()
                .map(|(index, edge)| (edge.action, (index == best) as u8 as f32))
                .collect();
        }
        let power = 1.0 / temperature;
        let weights = self
            .root
            .edges
            .iter()
            .map(|edge| (edge.visits.max(1) as f32).powf(power))
            .collect::<Vec<_>>();
        let total = weights.iter().sum::<f32>().max(f32::MIN_POSITIVE);
        self.root
            .edges
            .iter()
            .zip(weights)
            .map(|(edge, weight)| (edge.action, weight / total))
            .collect()
    }

    pub fn best_complete_ply(&self, model: &Model) -> Vec<Action> {
        let player = self.root.state.active_player;
        let mut state = self.root.state.clone();
        let mut actions = Vec::new();
        let mut current = &self.root;
        while let Some(index) = current.preferred_edge() {
            let action = current.edges[index].action;
            let Ok(next) = state.clone().apply(action) else {
                break;
            };
            actions.push(action);
            state = next;
            if state.active_player != player || state.winner().is_some() {
                break;
            }
            if let Some(child) = current.edges[index].child.as_deref() {
                current = child;
            } else {
                let mut temporary = Node::new(state.clone());
                temporary.expand(model);
                if let Some(next_index) = temporary.preferred_edge() {
                    actions.push(temporary.edges[next_index].action);
                }
                break;
            }
        }
        actions
    }
}

fn simulate_node(node: &mut Node, model: &Model, depth: usize, seen: &mut HashSet<u64>) -> f32 {
    if depth >= MAX_SEARCH_DEPTH {
        return 0.0;
    }
    if !node.expanded {
        return node.expand(model);
    }
    if node.edges.is_empty() {
        return node.value;
    }
    let total = node.edges.iter().map(|edge| edge.visits).sum::<u32>();
    let exploration = 1.5 * ((total + 1) as f32).sqrt();
    let index = node
        .edges
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            let score = |edge: &Edge| {
                let q = if edge.visits == 0 {
                    0.0
                } else {
                    edge.value_sum / edge.visits as f32
                };
                q + exploration * edge.prior / (edge.visits + 1) as f32
            };
            score(left).total_cmp(&score(right))
        })
        .map(|(index, _)| index)
        .unwrap_or(0);
    let parent_player = node.state.active_player;
    let action = node.edges[index].action;
    if node.edges[index].child.is_none() {
        let child_state = node
            .state
            .clone()
            .apply(action)
            .expect("Core advertised a legal action");
        node.edges[index].child = Some(Box::new(Node::new(child_state)));
    }
    let child = node.edges[index].child.as_mut().expect("created child");
    let fingerprint = state_fingerprint(&child.state);
    let child_value = if !seen.insert(fingerprint) {
        0.0
    } else {
        let result = simulate_node(child, model, depth + 1, seen);
        seen.remove(&fingerprint);
        result
    };
    let value = if child.state.active_player == parent_player {
        child_value
    } else {
        -child_value
    };
    node.edges[index].visits += 1;
    node.edges[index].value_sum += value;
    value
}

pub struct CherryAnalyzer {
    state: Option<State>,
    model: Model,
    search: Option<Search>,
}

impl Default for CherryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CherryAnalyzer {
    pub fn new() -> Self {
        Self::with_model(Model::embedded())
    }

    pub fn with_model(model: Model) -> Self {
        Self {
            state: None,
            model,
            search: None,
        }
    }
}

impl Analyzer for CherryAnalyzer {
    fn set_state(&mut self, state: State) {
        self.search = Some(Search::new(state.clone(), &self.model));
        self.state = Some(state);
    }

    fn think_for_one_tick(&mut self) {
        if let Some(search) = &mut self.search {
            search.simulate(&self.model);
        }
    }

    fn evaluation(&self) -> Evaluation {
        let Some(state) = &self.state else {
            return estimate(0.0);
        };
        if let Some(winner) = state.winner() {
            return Evaluation::MateInN { winner, plies: 0 };
        }
        let value = self.search.as_ref().map_or(0.0, Search::root_value) as f64;
        estimate(if state.active_player == Player::Alpha {
            value
        } else {
            -value
        })
    }

    fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        let Some(search) = &self.search else { return };
        let actions = search.best_complete_ply(&self.model);
        writer.reserve(actions.len());
        for action in actions {
            writer.push(action);
        }
    }
}

fn estimate(value: f64) -> Evaluation {
    Evaluation::Estimate(EvaluationEstimate::new(value).expect("finite model evaluation"))
}

pub fn encode_state(state: &State) -> Vec<f32> {
    let locations = [
        &state.reserves,
        &state.r1,
        &state.r2,
        &state.r3,
        &state.r4,
        &state.r5,
        &state.r6,
    ];
    let animals = animals();
    let mut encoded = vec![0.0; INPUT_SIZE];
    let perspective = state.active_player;
    for canonical_location in 0..7 {
        let actual_location = if perspective == Player::Alpha || canonical_location == 0 {
            canonical_location
        } else {
            7 - canonical_location
        };
        let cards = locations[actual_location];
        let base = canonical_location * 34;
        for (animal_index, animal) in animals.into_iter().enumerate() {
            encoded[base + animal_index * 2] =
                f32::from(cards.count(Card::Animal(animal), perspective)) / 2.0;
            encoded[base + animal_index * 2 + 1] =
                f32::from(cards.count(Card::Animal(animal), perspective.opponent())) / 2.0;
        }
        encoded[base + 32] = f32::from(cards.count(Card::Snipe, perspective));
        encoded[base + 33] = f32::from(cards.count(Card::Snipe, perspective.opponent()));
    }
    if let Some(leading) = state.leading_action {
        encoded[238] = 1.0;
        encoded[239 + animal_index(leading.actor)] = 1.0;
        encoded[255 + usize::from(leading.direction == StepDirection::Retreat)] = 1.0;
        encoded[257 + canonical_rank(state.active_player, leading.destination)] = 1.0;
    }
    encoded
}

pub fn action_index(state: &State, action: Action) -> usize {
    match action {
        Action::AnimalStep(step) => {
            animal_index(step.actor) * 12
                + usize::from(step.direction == StepDirection::Retreat) * 6
                + canonical_rank(state.active_player, step.destination)
        }
        Action::SnipeStep(step) => 192 + canonical_rank(state.active_player, step.destination),
        Action::Drop(drop) => {
            198 + animal_index(drop.actor) * 6
                + canonical_rank(state.active_player, drop.destination)
        }
    }
}

fn canonical_rank(player: Player, rank: Rank) -> usize {
    let actual = match rank {
        Rank::R1 => 0,
        Rank::R2 => 1,
        Rank::R3 => 2,
        Rank::R4 => 3,
        Rank::R5 => 4,
        Rank::R6 => 5,
    };
    if player == Player::Alpha {
        actual
    } else {
        5 - actual
    }
}

fn state_fingerprint(state: &State) -> u64 {
    encode_state(state).into_iter().fold(
        if state.leading_action.is_some() {
            0xCBF2_9CE4_8422_2325
        } else {
            0x8422_2325_CBF2_9CE4
        },
        |hash, value| (hash ^ u64::from(value.to_bits())).wrapping_mul(0x100_0000_01B3),
    )
}

fn animal_index(animal: Animal) -> usize {
    animals()
        .iter()
        .position(|&candidate| candidate == animal)
        .expect("all animals are indexed")
}

fn animals() -> [Animal; 16] {
    [
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
    ]
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1_u32 << 24) as f32
    }

    fn normalish(&mut self) -> f32 {
        (0..6).map(|_| self.unit()).sum::<f32>() - 3.0
    }
}

#[cfg(feature = "training")]
pub mod training {
    use super::*;

    pub struct Sample {
        pub input: Vec<f32>,
        pub policy: Vec<f32>,
        pub value: f32,
    }

    pub struct Adam {
        first: Vec<f32>,
        second: Vec<f32>,
        step: u64,
    }

    impl Adam {
        pub fn new() -> Self {
            Self {
                first: vec![0.0; PARAM_COUNT],
                second: vec![0.0; PARAM_COUNT],
                step: 0,
            }
        }

        pub fn to_bytes(&self) -> Vec<u8> {
            let mut bytes = Vec::with_capacity(24 + PARAM_COUNT * 8);
            bytes.extend_from_slice(b"CHADAM01");
            bytes.extend_from_slice(&self.step.to_le_bytes());
            bytes.extend_from_slice(&(PARAM_COUNT as u64).to_le_bytes());
            for value in self.first.iter().chain(&self.second) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes
        }

        pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
            let expected = 24 + PARAM_COUNT * 8;
            if bytes.len() != expected || bytes.get(..8) != Some(b"CHADAM01") {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid Cherry optimizer checkpoint",
                ));
            }
            let step = u64::from_le_bytes(bytes[8..16].try_into().expect("checked length"));
            let count =
                u64::from_le_bytes(bytes[16..24].try_into().expect("checked length")) as usize;
            if count != PARAM_COUNT {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "incompatible Cherry optimizer dimensions",
                ));
            }
            let values = bytes[24..]
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
                .collect::<Vec<_>>();
            if values.iter().any(|value| !value.is_finite()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "non-finite Cherry optimizer checkpoint",
                ));
            }
            Ok(Self {
                first: values[..PARAM_COUNT].to_vec(),
                second: values[PARAM_COUNT..].to_vec(),
                step,
            })
        }
    }

    impl Default for Adam {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Model {
        pub fn train_batch(
            &mut self,
            samples: &[Sample],
            optimizer: &mut Adam,
            learning_rate: f32,
        ) -> f32 {
            let mut gradient = vec![0.0; PARAM_COUNT];
            let mut total_loss = 0.0;
            for sample in samples {
                let forward = self.forward(&sample.input);
                let max_logit = forward
                    .logits
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max);
                let mut probabilities = forward
                    .logits
                    .iter()
                    .map(|logit| (*logit - max_logit).exp())
                    .collect::<Vec<_>>();
                let denominator = probabilities.iter().sum::<f32>().max(f32::MIN_POSITIVE);
                for probability in &mut probabilities {
                    *probability /= denominator;
                }
                for (action, probability) in probabilities.iter_mut().enumerate() {
                    let target = sample.policy[action];
                    if target > 0.0 {
                        total_loss -= target * probability.max(1e-12).ln();
                    }
                    *probability -= target;
                }
                let value_error = forward.value - sample.value;
                total_loss += value_error * value_error;
                let value_delta = 2.0 * value_error * (1.0 - forward.value * forward.value);

                let mut hidden_2_gradient = vec![0.0; HIDDEN_SIZE];
                for hidden in 0..HIDDEN_SIZE {
                    for action in 0..ACTION_SIZE {
                        gradient[WP + hidden * ACTION_SIZE + action] +=
                            forward.hidden_2[hidden] * probabilities[action];
                        hidden_2_gradient[hidden] += self.parameters
                            [WP + hidden * ACTION_SIZE + action]
                            * probabilities[action];
                    }
                    gradient[WV + hidden] += forward.hidden_2[hidden] * value_delta;
                    hidden_2_gradient[hidden] += self.parameters[WV + hidden] * value_delta;
                }
                for action in 0..ACTION_SIZE {
                    gradient[BP + action] += probabilities[action];
                }
                gradient[BV] += value_delta;

                let mut hidden_1_gradient = vec![0.0; HIDDEN_SIZE];
                for output in 0..HIDDEN_SIZE {
                    if forward.hidden_2[output] <= 0.0 {
                        hidden_2_gradient[output] = 0.0;
                    }
                    gradient[BR + output] += hidden_2_gradient[output];
                    hidden_1_gradient[output] += hidden_2_gradient[output];
                    for input in 0..HIDDEN_SIZE {
                        gradient[WR + input * HIDDEN_SIZE + output] +=
                            forward.hidden_1[input] * hidden_2_gradient[output];
                        hidden_1_gradient[input] += self.parameters
                            [WR + input * HIDDEN_SIZE + output]
                            * hidden_2_gradient[output];
                    }
                }
                for hidden in 0..HIDDEN_SIZE {
                    if forward.hidden_1[hidden] <= 0.0 {
                        hidden_1_gradient[hidden] = 0.0;
                    }
                    gradient[B1 + hidden] += hidden_1_gradient[hidden];
                    for input in 0..INPUT_SIZE {
                        gradient[W1 + input * HIDDEN_SIZE + hidden] +=
                            sample.input[input] * hidden_1_gradient[hidden];
                    }
                }
            }

            let scale = 1.0 / samples.len().max(1) as f32;
            optimizer.step += 1;
            let correction_1 = 1.0 - 0.9_f32.powi(optimizer.step.min(i32::MAX as u64) as i32);
            let correction_2 = 1.0 - 0.999_f32.powi(optimizer.step.min(i32::MAX as u64) as i32);
            for (index, parameter) in self.parameters.iter_mut().enumerate() {
                let grad = gradient[index] * scale + 1e-5 * *parameter;
                optimizer.first[index] = 0.9 * optimizer.first[index] + 0.1 * grad;
                optimizer.second[index] = 0.999 * optimizer.second[index] + 0.001 * grad * grad;
                let first = optimizer.first[index] / correction_1;
                let second = optimizer.second[index] / correction_2;
                *parameter -= learning_rate * first / (second.sqrt() + 1e-8);
            }
            self.training_steps += 1;
            total_loss * scale
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::initial_state;

    #[test]
    fn action_indices_are_unique_for_legal_actions() {
        for seed in 0..16 {
            let state = initial_state(seed);
            let mut actions = Vec::new();
            state.write_legal_actions(&mut actions);
            let indices = actions
                .iter()
                .map(|&action| action_index(&state, action))
                .collect::<HashSet<_>>();
            assert_eq!(indices.len(), actions.len());
            assert!(indices.iter().all(|&index| index < ACTION_SIZE));
        }
    }

    #[test]
    fn analyzer_always_returns_a_complete_legal_ply() {
        let state = initial_state(7071);
        let mut analyzer = CherryAnalyzer::with_model(Model::seeded(1));
        analyzer.set_state(state.clone());
        analyzer.think(8);
        let mut actions = Vec::new();
        analyzer.write_optimal_lop(&mut actions);
        assert!(!actions.is_empty());
        let player = state.active_player;
        let mut after = state;
        for action in actions {
            after = after.apply(action).unwrap();
        }
        assert!(after.active_player != player || after.winner().is_some());
    }

    #[test]
    fn checkpoints_round_trip() {
        let model = Model::seeded(42);
        let rebuilt = Model::from_bytes(&model.to_bytes()).unwrap();
        let state = initial_state(4);
        assert_eq!(model.predict(&state), rebuilt.predict(&state));
    }

    #[cfg(feature = "training")]
    #[test]
    fn optimizer_checkpoints_round_trip() {
        let optimizer = training::Adam::new();
        let bytes = optimizer.to_bytes();
        assert_eq!(
            training::Adam::from_bytes(&bytes).unwrap().to_bytes(),
            bytes
        );
    }
}
