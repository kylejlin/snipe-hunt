//! Fajita: a wide residual policy/value MCTS agent.
//!
//! Fajita has the same 256-unit, four-residual-layer architecture as Eel, but
//! uses an independent initialization seed and incompatible checkpoint and
//! optimizer formats. Its weights therefore always begin from a clean slate.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, Card, Evaluation, EvaluationEstimate, MateInN, Player,
    Rank, State, StepDirection,
};
use std::{fs, io, path::Path};

pub const INPUT_SIZE: usize = 263;
pub const WIDTH: usize = 256;
pub const RESIDUAL_LAYERS: usize = 4;
pub const ACTION_SIZE: usize = 294;
pub const INITIAL_SEED: u64 = 0xFA71_7A5E_2026_0001;
pub const PARAM_COUNT: usize = STEM_B
    + WIDTH
    + RESIDUAL_LAYERS * (WIDTH * WIDTH + WIDTH)
    + WIDTH * ACTION_SIZE
    + ACTION_SIZE
    + WIDTH
    + 1;

const MAGIC: &[u8; 8] = b"FAJNET01";
const HEADER_SIZE: usize = 8 + 4 * 4 + 8;
const MAX_SEARCH_DEPTH: usize = 256;
const PUCT: f32 = 1.65;

const STEM_W: usize = 0;
const STEM_B: usize = STEM_W + INPUT_SIZE * WIDTH;
const BLOCKS: usize = STEM_B + WIDTH;
const BLOCK_STRIDE: usize = WIDTH * WIDTH + WIDTH;
const POLICY_W: usize = BLOCKS + RESIDUAL_LAYERS * BLOCK_STRIDE;
const POLICY_B: usize = POLICY_W + WIDTH * ACTION_SIZE;
const VALUE_W: usize = POLICY_B + ACTION_SIZE;
const VALUE_B: usize = VALUE_W + WIDTH;

#[derive(Clone)]
pub struct Model {
    parameters: Vec<f32>,
    pub training_steps: u64,
}

struct Forward {
    activations: Vec<Vec<f32>>,
    logits: [f32; ACTION_SIZE],
    value: f32,
}

impl Model {
    pub fn seeded(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut parameters = vec![0.0; PARAM_COUNT];
        initialize_matrix(&mut parameters[STEM_W..STEM_B], INPUT_SIZE, &mut rng);
        for layer in 0..RESIDUAL_LAYERS {
            let weights = block_weights(layer);
            // Residual branches start small so a deep random network remains
            // numerically well behaved before it has learned useful features.
            let scale = (2.0 / WIDTH as f32).sqrt() * 0.25;
            for parameter in &mut parameters[weights] {
                *parameter = rng.normal() * scale;
            }
        }
        for parameter in &mut parameters[POLICY_W..POLICY_B] {
            *parameter = rng.normal() * 0.01;
        }
        for parameter in &mut parameters[VALUE_W..VALUE_B] {
            *parameter = rng.normal() * (1.0 / WIDTH as f32).sqrt();
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
        let mut bytes = Vec::with_capacity(HEADER_SIZE + PARAM_COUNT * 4);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&(INPUT_SIZE as u32).to_le_bytes());
        bytes.extend_from_slice(&(WIDTH as u32).to_le_bytes());
        bytes.extend_from_slice(&(RESIDUAL_LAYERS as u32).to_le_bytes());
        bytes.extend_from_slice(&(ACTION_SIZE as u32).to_le_bytes());
        bytes.extend_from_slice(&self.training_steps.to_le_bytes());
        for parameter in &self.parameters {
            bytes.extend_from_slice(&parameter.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != HEADER_SIZE + PARAM_COUNT * 4 || bytes.get(..8) != Some(MAGIC) {
            return Err(invalid("invalid Fajita checkpoint"));
        }
        let read_dimension = |offset: usize| {
            u32::from_le_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("checkpoint length checked"),
            ) as usize
        };
        if read_dimension(8) != INPUT_SIZE
            || read_dimension(12) != WIDTH
            || read_dimension(16) != RESIDUAL_LAYERS
            || read_dimension(20) != ACTION_SIZE
        {
            return Err(invalid("incompatible Fajita checkpoint dimensions"));
        }
        let training_steps =
            u64::from_le_bytes(bytes[24..32].try_into().expect("checkpoint length checked"));
        let parameters = bytes[HEADER_SIZE..]
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
            .collect::<Vec<_>>();
        if parameters.iter().any(|value| !value.is_finite()) {
            return Err(invalid("non-finite Fajita checkpoint"));
        }
        Ok(Self {
            parameters,
            training_steps,
        })
    }

    pub fn predict(&self, state: &State) -> ([f32; ACTION_SIZE], f32) {
        let forward = self.forward(&encode_state(state));
        (forward.logits, forward.value)
    }

    fn forward(&self, input: &[f32; INPUT_SIZE]) -> Forward {
        let mut activations = Vec::with_capacity(RESIDUAL_LAYERS + 1);
        let mut stem = self.parameters[STEM_B..STEM_B + WIDTH].to_vec();
        for (input_index, &value) in input.iter().enumerate() {
            if value == 0.0 {
                continue;
            }
            let weights =
                &self.parameters[STEM_W + input_index * WIDTH..STEM_W + (input_index + 1) * WIDTH];
            for output in 0..WIDTH {
                stem[output] = value.mul_add(weights[output], stem[output]);
            }
        }
        relu(&mut stem);
        activations.push(stem);

        for layer in 0..RESIDUAL_LAYERS {
            let previous = activations.last().expect("stem exists");
            let mut next = self.parameters[block_biases(layer)].to_vec();
            let weights_start = block_weights(layer).start;
            for (input_index, &value) in previous.iter().enumerate() {
                if value == 0.0 {
                    continue;
                }
                let row = &self.parameters[weights_start + input_index * WIDTH
                    ..weights_start + (input_index + 1) * WIDTH];
                for output in 0..WIDTH {
                    next[output] = value.mul_add(row[output], next[output]);
                }
            }
            for (value, &skip) in next.iter_mut().zip(previous) {
                *value = (*value + skip).max(0.0);
            }
            activations.push(next);
        }

        let trunk = activations.last().expect("residual trunk exists");
        let mut logits = [0.0; ACTION_SIZE];
        logits.copy_from_slice(&self.parameters[POLICY_B..POLICY_B + ACTION_SIZE]);
        for (input_index, &value) in trunk.iter().enumerate() {
            if value == 0.0 {
                continue;
            }
            let row = &self.parameters
                [POLICY_W + input_index * ACTION_SIZE..POLICY_W + (input_index + 1) * ACTION_SIZE];
            for action in 0..ACTION_SIZE {
                logits[action] = value.mul_add(row[action], logits[action]);
            }
        }
        let mut raw_value = self.parameters[VALUE_B];
        for (index, &value) in trunk.iter().enumerate() {
            raw_value = value.mul_add(self.parameters[VALUE_W + index], raw_value);
        }
        Forward {
            activations,
            logits,
            value: raw_value.tanh(),
        }
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn initialize_matrix(parameters: &mut [f32], inputs: usize, rng: &mut Rng) {
    let scale = (2.0 / inputs as f32).sqrt();
    for parameter in parameters {
        *parameter = rng.normal() * scale;
    }
}

fn relu(values: &mut [f32]) {
    for value in values {
        *value = value.max(0.0);
    }
}

fn block_weights(layer: usize) -> std::ops::Range<usize> {
    let start = BLOCKS + layer * BLOCK_STRIDE;
    start..start + WIDTH * WIDTH
}

fn block_biases(layer: usize) -> std::ops::Range<usize> {
    let start = block_weights(layer).end;
    start..start + WIDTH
}

struct Edge {
    action: Action,
    prior: f32,
    clean_prior: f32,
    visits: u32,
    value_sum: f32,
    child: Option<usize>,
}

struct Node {
    state: State,
    key: u64,
    edges: Vec<Edge>,
    value: f32,
    expanded: bool,
}

impl Node {
    fn new(state: State) -> Self {
        Self {
            key: state_key(&state),
            state,
            edges: Vec::new(),
            value: 0.0,
            expanded: false,
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
        let mut legal = Vec::with_capacity(ACTION_SIZE);
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
            .map(|action| {
                let prior =
                    (logits[action_index(&self.state, action)] - maximum).exp() / denominator;
                Edge {
                    action,
                    prior,
                    clean_prior: prior,
                    visits: 0,
                    value_sum: 0.0,
                    child: None,
                }
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
                    .then_with(|| left.clean_prior.total_cmp(&right.clean_prior))
            })
            .map(|(index, _)| index)
    }
}

pub struct Search {
    nodes: Vec<Node>,
    root: usize,
    path: Vec<(usize, usize)>,
    simulations: u64,
}

impl Search {
    pub fn new(state: State, model: &Model) -> Self {
        let mut root = Node::new(state);
        root.expand(model);
        Self {
            nodes: vec![root],
            root: 0,
            path: Vec::with_capacity(MAX_SEARCH_DEPTH),
            simulations: 0,
        }
    }

    fn root(&self) -> &Node {
        &self.nodes[self.root]
    }

    fn root_mut(&mut self) -> &mut Node {
        &mut self.nodes[self.root]
    }

    pub fn simulate(&mut self, model: &Model) {
        self.path.clear();
        let mut seen = [0_u64; MAX_SEARCH_DEPTH + 1];
        let mut node_index = self.root;
        seen[0] = self.nodes[node_index].key;
        let (mut value, mut value_player) = loop {
            let depth = self.path.len();
            let player = self.nodes[node_index].state.active_player;
            if depth == MAX_SEARCH_DEPTH {
                break (0.0, player);
            }
            if !self.nodes[node_index].expanded {
                let value = self.nodes[node_index].expand(model);
                break (value, player);
            }
            if self.nodes[node_index].edges.is_empty() {
                break (self.nodes[node_index].value, player);
            }

            let total_visits = self.nodes[node_index]
                .edges
                .iter()
                .map(|edge| u64::from(edge.visits))
                .sum::<u64>();
            let exploration = PUCT * ((total_visits + 1) as f32).sqrt();
            let edge_index = self.nodes[node_index]
                .edges
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    edge_score(left, exploration).total_cmp(&edge_score(right, exploration))
                })
                .map(|(index, _)| index)
                .expect("expanded live node has edges");

            let child = if let Some(child) = self.nodes[node_index].edges[edge_index].child {
                child
            } else {
                let action = self.nodes[node_index].edges[edge_index].action;
                let state = self.nodes[node_index]
                    .state
                    .clone()
                    .apply(action)
                    .expect("Core supplied a legal action");
                let child = self.nodes.len();
                self.nodes.push(Node::new(state));
                self.nodes[node_index].edges[edge_index].child = Some(child);
                child
            };
            self.path.push((node_index, edge_index));
            let child_player = self.nodes[child].state.active_player;
            if seen[..=depth].contains(&self.nodes[child].key) {
                break (0.0, child_player);
            }
            seen[depth + 1] = self.nodes[child].key;
            node_index = child;
        };

        while let Some((parent, edge_index)) = self.path.pop() {
            let parent_player = self.nodes[parent].state.active_player;
            if value_player != parent_player {
                value = -value;
            }
            let edge = &mut self.nodes[parent].edges[edge_index];
            edge.visits += 1;
            edge.value_sum += value;
            value_player = parent_player;
        }
        self.simulations += 1;
    }

    pub fn simulate_n(&mut self, model: &Model, count: usize) {
        for _ in 0..count {
            self.simulate(model);
        }
    }

    pub fn root_action_count(&self) -> usize {
        self.root().edges.len()
    }

    pub fn add_root_noise(&mut self, alpha: f32, fraction: f32, seed: u64) {
        assert!(alpha.is_finite() && alpha > 0.0);
        assert!(fraction.is_finite() && (0.0..=1.0).contains(&fraction));
        if self.root().edges.is_empty() {
            return;
        }
        let mut rng = Rng::new(seed);
        let mut noise = Vec::with_capacity(self.root().edges.len());
        let mut total = 0.0_f64;
        for _ in 0..self.root().edges.len() {
            let sample = rng.gamma(f64::from(alpha));
            noise.push(sample);
            total += sample;
        }
        let clean = 1.0 - fraction;
        for (edge, sample) in self.root_mut().edges.iter_mut().zip(noise) {
            edge.prior = clean * edge.clean_prior + fraction * (sample / total) as f32;
        }
    }

    pub fn policy(&self, temperature: f32) -> Vec<(Action, f32)> {
        let root = self.root();
        if root.edges.is_empty() {
            return Vec::new();
        }
        let visits = root
            .edges
            .iter()
            .map(|edge| u64::from(edge.visits))
            .sum::<u64>();
        let weight = |edge: &Edge| {
            if visits == 0 {
                edge.clean_prior
            } else {
                edge.visits as f32
            }
        };
        if !temperature.is_finite() || temperature <= 0.01 {
            let best = root
                .edges
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| weight(left).total_cmp(&weight(right)))
                .map(|(index, _)| index)
                .expect("root has edges");
            return root
                .edges
                .iter()
                .enumerate()
                .map(|(index, edge)| (edge.action, f32::from(index == best)))
                .collect();
        }
        let inverse_temperature = temperature.recip();
        let maximum = root
            .edges
            .iter()
            .map(|edge| {
                let base = weight(edge);
                if base > 0.0 {
                    base.ln() * inverse_temperature
                } else {
                    f32::NEG_INFINITY
                }
            })
            .fold(f32::NEG_INFINITY, f32::max);
        let mut total = 0.0;
        let mut policy = root
            .edges
            .iter()
            .map(|edge| {
                let base = weight(edge);
                let probability = if base > 0.0 {
                    (base.ln() * inverse_temperature - maximum).exp()
                } else {
                    0.0
                };
                total += probability;
                (edge.action, probability)
            })
            .collect::<Vec<_>>();
        for (_, probability) in &mut policy {
            *probability /= total.max(f32::MIN_POSITIVE);
        }
        policy
    }

    pub fn root_value(&self) -> f32 {
        let root = self.root();
        let visits = root
            .edges
            .iter()
            .map(|edge| u64::from(edge.visits))
            .sum::<u64>();
        if visits == 0 {
            root.value
        } else {
            root.edges.iter().map(|edge| edge.value_sum).sum::<f32>() / visits as f32
        }
    }

    pub fn advance(&mut self, action: Action, model: &Model) -> bool {
        let Some(edge_index) = self
            .root()
            .edges
            .iter()
            .position(|edge| edge.action == action)
        else {
            return false;
        };
        let child = if let Some(child) = self.root().edges[edge_index].child {
            child
        } else {
            let Ok(state) = self.root().state.clone().apply(action) else {
                return false;
            };
            let child = self.nodes.len();
            self.nodes.push(Node::new(state));
            child
        };
        self.root = child;
        if !self.root().expanded {
            self.nodes[child].expand(model);
        }
        self.simulations = 0;
        true
    }

    pub fn best_complete_ply(&self, model: &Model) -> Vec<Action> {
        let player = self.root().state.active_player;
        let mut state = self.root().state.clone();
        let mut node = self.root;
        let mut result = Vec::with_capacity(2);
        while let Some(edge_index) = self.nodes[node].preferred_edge() {
            let action = self.nodes[node].edges[edge_index].action;
            let Ok(next) = state.clone().apply(action) else {
                break;
            };
            result.push(action);
            state = next;
            if state.active_player != player || state.winner().is_some() {
                break;
            }
            if let Some(child) = self.nodes[node].edges[edge_index].child {
                node = child;
            } else {
                let mut continuation = Node::new(state.clone());
                continuation.expand(model);
                if let Some(index) = continuation.preferred_edge() {
                    result.push(continuation.edges[index].action);
                }
                break;
            }
        }
        result
    }
}

fn edge_score(edge: &Edge, exploration: f32) -> f32 {
    let value = if edge.visits == 0 {
        0.0
    } else {
        edge.value_sum / edge.visits as f32
    };
    value + exploration * edge.prior / (edge.visits + 1) as f32
}

pub struct FajitaAnalyzer {
    state: Option<State>,
    model: Model,
    search: Option<Search>,
}

impl Default for FajitaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl FajitaAnalyzer {
    pub fn new() -> Self {
        Self::with_model(Model::seeded(INITIAL_SEED))
    }

    pub fn with_model(model: Model) -> Self {
        Self {
            state: None,
            model,
            search: None,
        }
    }
}

impl Analyzer for FajitaAnalyzer {
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
            return mate_in(winner, 0);
        }
        let value = self.search.as_ref().map_or(0.0, Search::root_value);
        estimate(if state.active_player == Player::Alpha {
            value
        } else {
            -value
        })
    }

    fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        let Some(search) = &self.search else {
            return;
        };
        let actions = search.best_complete_ply(&self.model);
        writer.reserve(actions.len());
        for action in actions {
            writer.push(action);
        }
    }
}

fn mate_in(winner: Player, plies: u32) -> Evaluation {
    MateInN::new(winner, plies)
        .expect("Fajita mate distance is within Core's range")
        .into()
}

fn estimate(value: f32) -> Evaluation {
    let millipoints = (value.clamp(-1.0, 1.0) * 1_000.0).round() as i32;
    EvaluationEstimate::from_millipoints(millipoints)
        .expect("bounded Fajita value is a valid estimate")
        .into()
}

pub fn encode_state(state: &State) -> [f32; INPUT_SIZE] {
    let locations = [
        &state.reserves,
        &state.r1,
        &state.r2,
        &state.r3,
        &state.r4,
        &state.r5,
        &state.r6,
    ];
    let mut encoded = [0.0; INPUT_SIZE];
    let perspective = state.active_player;
    for canonical_location in 0..7 {
        let actual_location = if perspective == Player::Alpha || canonical_location == 0 {
            canonical_location
        } else {
            7 - canonical_location
        };
        let cards = locations[actual_location];
        let base = canonical_location * 34;
        for (index, animal) in ANIMALS.into_iter().enumerate() {
            encoded[base + index * 2] =
                f32::from(cards.count(Card::Animal(animal), perspective)) * 0.5;
            encoded[base + index * 2 + 1] =
                f32::from(cards.count(Card::Animal(animal), perspective.opponent())) * 0.5;
        }
        encoded[base + 32] = f32::from(cards.count(Card::Snipe, perspective));
        encoded[base + 33] = f32::from(cards.count(Card::Snipe, perspective.opponent()));
    }
    if let Some(leading) = state.leading_action {
        encoded[238] = 1.0;
        encoded[239 + animal_index(leading.actor)] = 1.0;
        encoded[255 + usize::from(leading.direction == StepDirection::Retreat)] = 1.0;
        encoded[257 + canonical_rank(perspective, leading.destination)] = 1.0;
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
    let rank = match rank {
        Rank::R1 => 0,
        Rank::R2 => 1,
        Rank::R3 => 2,
        Rank::R4 => 3,
        Rank::R5 => 4,
        Rank::R6 => 5,
    };
    if player == Player::Alpha {
        rank
    } else {
        5 - rank
    }
}

fn animal_index(animal: Animal) -> usize {
    ANIMALS
        .iter()
        .position(|&candidate| candidate == animal)
        .expect("all Core animals have a policy index")
}

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

pub fn state_key(state: &State) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    hash = hash_byte(hash, u8::from(state.active_player == Player::Beta));
    for cards in [
        state.reserves,
        state.r1,
        state.r2,
        state.r3,
        state.r4,
        state.r5,
        state.r6,
    ] {
        for animal in ANIMALS {
            hash = hash_byte(hash, cards.count(Card::Animal(animal), Player::Alpha));
            hash = hash_byte(hash, cards.count(Card::Animal(animal), Player::Beta));
        }
        hash = hash_byte(hash, cards.count(Card::Snipe, Player::Alpha));
        hash = hash_byte(hash, cards.count(Card::Snipe, Player::Beta));
    }
    if let Some(leading) = state.leading_action {
        hash = hash_byte(hash, 1);
        hash = hash_byte(hash, animal_index(leading.actor) as u8);
        hash = hash_byte(hash, u8::from(leading.direction == StepDirection::Retreat));
        hash = hash_byte(
            hash,
            canonical_rank(state.active_player, leading.destination) as u8,
        );
    } else {
        hash = hash_byte(hash, 0);
    }
    hash
}

fn hash_byte(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
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

    fn open_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / ((1_u64 << 53) as f64 + 1.0)
    }

    fn normal(&mut self) -> f32 {
        let radius = (-2.0 * self.open_unit().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.open_unit();
        (radius * angle.cos()) as f32
    }

    fn gamma(&mut self, shape: f64) -> f64 {
        if shape < 1.0 {
            return self.gamma(shape + 1.0) * self.open_unit().powf(1.0 / shape);
        }
        let d = shape - 1.0 / 3.0;
        let c = (9.0 * d).sqrt().recip();
        loop {
            let x = f64::from(self.normal());
            let base = 1.0 + c * x;
            if base <= 0.0 {
                continue;
            }
            let v = base * base * base;
            let u = self.open_unit();
            if u < 1.0 - 0.0331 * x.powi(4) || u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
                return d * v;
            }
        }
    }
}

#[cfg(feature = "training")]
pub mod training {
    use super::*;

    pub struct Sample {
        pub input: [f32; INPUT_SIZE],
        pub policy: [f32; ACTION_SIZE],
        pub value: f32,
    }

    #[derive(Clone)]
    pub struct Adam {
        first: Vec<f32>,
        second: Vec<f32>,
        step: u64,
    }

    impl Default for Adam {
        fn default() -> Self {
            Self::new()
        }
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
            let mut bytes = Vec::with_capacity(20 + PARAM_COUNT * 8);
            bytes.extend_from_slice(b"FAJOP001");
            bytes.extend_from_slice(&(PARAM_COUNT as u32).to_le_bytes());
            bytes.extend_from_slice(&self.step.to_le_bytes());
            for values in [&self.first, &self.second] {
                for value in values {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
            bytes
        }

        pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
            let header = 20;
            if bytes.len() != header + PARAM_COUNT * 8 || bytes.get(..8) != Some(b"FAJOP001") {
                return Err(invalid("invalid Fajita optimizer checkpoint"));
            }
            let count =
                u32::from_le_bytes(bytes[8..12].try_into().expect("optimizer length checked"))
                    as usize;
            if count != PARAM_COUNT {
                return Err(invalid("incompatible Fajita optimizer dimensions"));
            }
            let step =
                u64::from_le_bytes(bytes[12..20].try_into().expect("optimizer length checked"));
            let floats = bytes[20..]
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
                .collect::<Vec<_>>();
            if floats.iter().any(|value| !value.is_finite()) {
                return Err(invalid("non-finite Fajita optimizer checkpoint"));
            }
            Ok(Self {
                first: floats[..PARAM_COUNT].to_vec(),
                second: floats[PARAM_COUNT..].to_vec(),
                step,
            })
        }
    }

    impl Model {
        pub fn train_batch(
            &mut self,
            samples: &[Sample],
            optimizer: &mut Adam,
            learning_rate: f32,
        ) -> f32 {
            if samples.is_empty() {
                return 0.0;
            }
            let mut gradients = vec![0.0; PARAM_COUNT];
            let mut total_loss = 0.0;
            for sample in samples {
                let forward = self.forward(&sample.input);
                let maximum = forward
                    .logits
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max);
                let mut probabilities = [0.0; ACTION_SIZE];
                let denominator = forward
                    .logits
                    .iter()
                    .map(|logit| (*logit - maximum).exp())
                    .sum::<f32>()
                    .max(f32::MIN_POSITIVE);
                for (index, probability) in probabilities.iter_mut().enumerate() {
                    *probability = (forward.logits[index] - maximum).exp() / denominator;
                    if sample.policy[index] > 0.0 {
                        total_loss -= sample.policy[index] * probability.max(1e-9).ln();
                    }
                }
                let value_error = forward.value - sample.value;
                total_loss += value_error * value_error;

                let trunk = forward.activations.last().expect("trunk exists");
                let mut trunk_gradient = vec![0.0; WIDTH];
                for action in 0..ACTION_SIZE {
                    let derivative = probabilities[action] - sample.policy[action];
                    gradients[POLICY_B + action] += derivative;
                    for input in 0..WIDTH {
                        gradients[POLICY_W + input * ACTION_SIZE + action] +=
                            trunk[input] * derivative;
                        trunk_gradient[input] +=
                            self.parameters[POLICY_W + input * ACTION_SIZE + action] * derivative;
                    }
                }

                let raw_value_gradient = 2.0 * value_error * (1.0 - forward.value * forward.value);
                gradients[VALUE_B] += raw_value_gradient;
                for input in 0..WIDTH {
                    gradients[VALUE_W + input] += trunk[input] * raw_value_gradient;
                    trunk_gradient[input] += self.parameters[VALUE_W + input] * raw_value_gradient;
                }

                for layer in (0..RESIDUAL_LAYERS).rev() {
                    let input = &forward.activations[layer];
                    let output = &forward.activations[layer + 1];
                    let weights_start = block_weights(layer).start;
                    let biases_start = block_biases(layer).start;
                    let mut input_gradient = vec![0.0; WIDTH];
                    for destination in 0..WIDTH {
                        if output[destination] <= 0.0 {
                            trunk_gradient[destination] = 0.0;
                        }
                        let derivative = trunk_gradient[destination];
                        gradients[biases_start + destination] += derivative;
                        input_gradient[destination] += derivative;
                        for source in 0..WIDTH {
                            gradients[weights_start + source * WIDTH + destination] +=
                                input[source] * derivative;
                            input_gradient[source] += self.parameters
                                [weights_start + source * WIDTH + destination]
                                * derivative;
                        }
                    }
                    trunk_gradient = input_gradient;
                }

                let stem = &forward.activations[0];
                for output in 0..WIDTH {
                    if stem[output] <= 0.0 {
                        trunk_gradient[output] = 0.0;
                    }
                    let derivative = trunk_gradient[output];
                    gradients[STEM_B + output] += derivative;
                    for input in 0..INPUT_SIZE {
                        gradients[STEM_W + input * WIDTH + output] +=
                            sample.input[input] * derivative;
                    }
                }
            }

            let inverse_batch = 1.0 / samples.len() as f32;
            let squared_norm = gradients
                .iter()
                .map(|gradient| (gradient * inverse_batch).powi(2))
                .sum::<f32>();
            let clipping = if squared_norm > 25.0 {
                5.0 / squared_norm.sqrt()
            } else {
                1.0
            };
            optimizer.step += 1;
            let correction_1 = 1.0 - 0.9_f32.powf(optimizer.step as f32);
            let correction_2 = 1.0 - 0.999_f32.powf(optimizer.step as f32);
            for (index, parameter) in self.parameters.iter_mut().enumerate() {
                let mut gradient = gradients[index] * inverse_batch * clipping;
                if is_weight(index) {
                    gradient += 1e-5 * *parameter;
                }
                optimizer.first[index] = 0.9 * optimizer.first[index] + 0.1 * gradient;
                optimizer.second[index] =
                    0.999 * optimizer.second[index] + 0.001 * gradient * gradient;
                let first = optimizer.first[index] / correction_1;
                let second = optimizer.second[index] / correction_2;
                *parameter -= learning_rate * first / (second.sqrt() + 1e-8);
            }
            self.training_steps += 1;
            total_loss * inverse_batch
        }
    }

    fn is_weight(index: usize) -> bool {
        if index < STEM_B {
            return true;
        }
        for layer in 0..RESIDUAL_LAYERS {
            if block_weights(layer).contains(&index) {
                return true;
            }
        }
        (POLICY_W..POLICY_B).contains(&index) || (VALUE_W..VALUE_B).contains(&index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_prng::initial_state;
    use std::collections::HashSet;

    #[test]
    fn checkpoint_round_trip_preserves_predictions() {
        let model = Model::seeded(7);
        let state = initial_state(11);
        let expected = model.predict(&state);
        let restored = Model::from_bytes(&model.to_bytes()).unwrap();
        let actual = restored.predict(&state);
        assert_eq!(expected, actual);
    }

    #[test]
    fn rejects_eel_checkpoint_format() {
        let mut bytes = Model::seeded(INITIAL_SEED).to_bytes();
        bytes[..8].copy_from_slice(b"EELNET01");
        assert!(Model::from_bytes(&bytes).is_err());
    }

    #[test]
    fn legal_actions_have_unique_policy_indices() {
        for seed in 0..32 {
            let state = initial_state(seed);
            let mut actions = Vec::new();
            state.write_legal_actions(&mut actions);
            let indices = actions
                .iter()
                .map(|&action| action_index(&state, action))
                .collect::<HashSet<_>>();
            assert_eq!(actions.len(), indices.len());
            assert!(indices.iter().all(|&index| index < ACTION_SIZE));
        }
    }

    #[test]
    fn analyzer_returns_a_complete_legal_ply() {
        for seed in 0..8 {
            let mut state = initial_state(seed);
            let player = state.active_player;
            let mut analyzer = FajitaAnalyzer::new();
            analyzer.set_state(state.clone());
            analyzer.think(4);
            let mut line = Vec::new();
            analyzer.write_optimal_lop(&mut line);
            assert!(!line.is_empty());
            for action in line {
                state = state.apply(action).unwrap();
                if state.active_player != player || state.winner().is_some() {
                    break;
                }
            }
            assert!(state.active_player != player || state.winner().is_some());
        }
    }

    #[cfg(feature = "training")]
    #[test]
    fn training_moves_policy_and_value_toward_a_target() {
        let state = initial_state(5);
        let mut model = Model::seeded(3);
        let before = model.predict(&state);
        let mut policy = [0.0; ACTION_SIZE];
        let mut actions = Vec::new();
        state.write_legal_actions(&mut actions);
        policy[action_index(&state, actions[0])] = 1.0;
        let sample = training::Sample {
            input: encode_state(&state),
            policy,
            value: 1.0,
        };
        let target = action_index(&state, actions[0]);
        let mut optimizer = training::Adam::new();
        let mut loss = 0.0;
        for _ in 0..12 {
            loss = model.train_batch(std::slice::from_ref(&sample), &mut optimizer, 0.001);
        }
        let after = model.predict(&state);
        assert!(loss.is_finite());
        assert!(after.0[target] > before.0[target]);
        assert!(after.1 > before.1);
    }
}
