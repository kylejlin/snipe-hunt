//! Cherry: a rules-only policy/value MCTS agent.
//!
//! The browser uses a frozen checkpoint. The optional `training` feature
//! exposes the small amount of machinery needed by the native self-play
//! trainer; neither human games nor hand-authored Snipe Hunt heuristics are
//! part of the model.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, Card, Evaluation, EvaluationEstimate, MateInN,
    OptimalOutcome, Player, Rank, State, StepDirection,
};
use std::{fs, io, path::Path};

pub const INPUT_SIZE: usize = 263;
pub const HIDDEN_SIZE: usize = 128;
pub const ACTION_SIZE: usize = 294;
const MAGIC: &[u8; 8] = b"CHERRY01";
const MAX_SEARCH_DEPTH: usize = 256;
const MAX_LEGAL_ACTIONS: usize = ACTION_SIZE;
const ARGMAX_TEMPERATURE: f32 = 0.01;
const ARENA_INITIAL_CAPACITY: usize = 1024;

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

    fn forward(&self, input: &[f32; INPUT_SIZE]) -> Forward {
        let mut hidden_1 = [0.0; HIDDEN_SIZE];
        hidden_1.copy_from_slice(&self.parameters[B1..B1 + HIDDEN_SIZE]);
        for (index, &value) in input.iter().enumerate() {
            // State features are sparse. Besides avoiding useless arithmetic,
            // traversing a whole weight row at a time gives LLVM a simple,
            // contiguous loop to vectorize.
            if value != 0.0 {
                let weights =
                    &self.parameters[W1 + index * HIDDEN_SIZE..W1 + (index + 1) * HIDDEN_SIZE];
                for output in 0..HIDDEN_SIZE {
                    hidden_1[output] = value.mul_add(weights[output], hidden_1[output]);
                }
            }
        }
        for hidden in &mut hidden_1 {
            *hidden = hidden.max(0.0);
        }

        let mut hidden_2 = [0.0; HIDDEN_SIZE];
        for output in 0..HIDDEN_SIZE {
            hidden_2[output] = hidden_1[output] + self.parameters[BR + output];
        }
        for (index, &value) in hidden_1.iter().enumerate() {
            if value != 0.0 {
                let weights =
                    &self.parameters[WR + index * HIDDEN_SIZE..WR + (index + 1) * HIDDEN_SIZE];
                for output in 0..HIDDEN_SIZE {
                    hidden_2[output] = value.mul_add(weights[output], hidden_2[output]);
                }
            }
        }
        for hidden in &mut hidden_2 {
            *hidden = hidden.max(0.0);
        }

        let mut logits = [0.0; ACTION_SIZE];
        logits.copy_from_slice(&self.parameters[BP..BP + ACTION_SIZE]);
        for (index, &value) in hidden_2.iter().enumerate() {
            if value != 0.0 {
                let weights =
                    &self.parameters[WP + index * ACTION_SIZE..WP + (index + 1) * ACTION_SIZE];
                for action in 0..ACTION_SIZE {
                    logits[action] = value.mul_add(weights[action], logits[action]);
                }
            }
        }
        let mut raw_value = self.parameters[BV];
        for (index, &value) in hidden_2.iter().enumerate() {
            raw_value = value.mul_add(self.parameters[WV + index], raw_value);
        }
        Forward {
            hidden_1,
            hidden_2,
            logits,
            value: raw_value.tanh(),
        }
    }

    pub fn predict(&self, state: &State) -> ([f32; ACTION_SIZE], f32) {
        let output = self.forward(&encode_state(state));
        (output.logits, output.value)
    }
}

#[allow(dead_code)]
struct Forward {
    hidden_1: [f32; HIDDEN_SIZE],
    hidden_2: [f32; HIDDEN_SIZE],
    logits: [f32; ACTION_SIZE],
    value: f32,
}

struct Edge {
    action: Action,
    prior: f32,
    network_prior: f32,
    visits: u32,
    value_sum: f32,
    child: Option<usize>,
}

struct Node {
    state: State,
    fingerprint: u64,
    edges: Vec<Edge>,
    expanded: bool,
    value: f32,
}

impl Node {
    fn new(state: State) -> Self {
        let fingerprint = state_fingerprint(&state);
        Self {
            state,
            fingerprint,
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
        let mut legal = ActionBuffer::new();
        self.state.write_legal_actions(&mut legal);
        let (logits, value) = model.predict(&self.state);
        let maximum = legal
            .as_slice()
            .iter()
            .map(|&action| logits[action_index(&self.state, action)])
            .fold(f32::NEG_INFINITY, f32::max);
        let denominator = legal
            .as_slice()
            .iter()
            .map(|&action| (logits[action_index(&self.state, action)] - maximum).exp())
            .sum::<f32>()
            .max(f32::MIN_POSITIVE);
        self.edges.clear();
        self.edges.reserve(legal.len());
        self.edges
            .extend(legal.as_slice().iter().copied().map(|action| {
                let prior =
                    (logits[action_index(&self.state, action)] - maximum).exp() / denominator;
                Edge {
                    prior,
                    network_prior: prior,
                    action,
                    visits: 0,
                    value_sum: 0.0,
                    child: None,
                }
            }));
        self.value = value;
        self.expanded = true;
        value
    }

    fn reset(&mut self, state: State) {
        self.fingerprint = state_fingerprint(&state);
        self.state = state;
        self.edges.clear();
        self.expanded = false;
        self.value = 0.0;
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
    nodes: Vec<Node>,
    free_nodes: Vec<usize>,
    root: usize,
    simulations: u64,
    simulation_path: Vec<(usize, usize)>,
    reclaim_stack: Vec<usize>,
}

impl Search {
    pub fn new(state: State, model: &Model) -> Self {
        let mut root = Node::new(state);
        root.expand(model);
        let mut nodes = Vec::with_capacity(ARENA_INITIAL_CAPACITY);
        nodes.push(root);
        Self {
            nodes,
            free_nodes: Vec::new(),
            root: 0,
            simulations: 0,
            simulation_path: Vec::with_capacity(MAX_SEARCH_DEPTH),
            reclaim_stack: Vec::new(),
        }
    }

    fn root_node(&self) -> &Node {
        &self.nodes[self.root]
    }

    fn root_node_mut(&mut self) -> &mut Node {
        &mut self.nodes[self.root]
    }

    fn allocate_node(&mut self, state: State) -> usize {
        if let Some(index) = self.free_nodes.pop() {
            self.nodes[index].reset(state);
            index
        } else {
            let index = self.nodes.len();
            self.nodes.push(Node::new(state));
            index
        }
    }

    fn reclaim_subtree(&mut self, root: usize) {
        self.reclaim_stack.clear();
        self.reclaim_stack.push(root);
        while let Some(index) = self.reclaim_stack.pop() {
            self.reclaim_stack
                .extend(self.nodes[index].edges.iter().filter_map(|edge| edge.child));
            self.free_nodes.push(index);
        }
    }

    pub fn simulate(&mut self, model: &Model) {
        self.simulation_path.clear();
        let mut seen = [0_u64; MAX_SEARCH_DEPTH + 1];
        let mut node_index = self.root;
        seen[0] = self.nodes[node_index].fingerprint;

        let (mut value, mut value_player) = loop {
            let depth = self.simulation_path.len();
            let player = self.nodes[node_index].state.active_player;
            if depth >= MAX_SEARCH_DEPTH {
                break (0.0, player);
            }
            if !self.nodes[node_index].expanded {
                let value = self.nodes[node_index].expand(model);
                break (value, player);
            }
            if self.nodes[node_index].edges.is_empty() {
                break (self.nodes[node_index].value, player);
            }

            let total = self.nodes[node_index]
                .edges
                .iter()
                .map(|edge| u64::from(edge.visits))
                .sum::<u64>();
            let exploration = 1.5 * ((total + 1) as f32).sqrt();
            let edge_index = self.nodes[node_index]
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

            let child_index = if let Some(child) = self.nodes[node_index].edges[edge_index].child {
                child
            } else {
                let action = self.nodes[node_index].edges[edge_index].action;
                let child_state = self.nodes[node_index]
                    .state
                    .clone()
                    .apply(action)
                    .expect("Core advertised a legal action");
                let child = self.allocate_node(child_state);
                self.nodes[node_index].edges[edge_index].child = Some(child);
                child
            };
            self.simulation_path.push((node_index, edge_index));
            let fingerprint = self.nodes[child_index].fingerprint;
            let child_player = self.nodes[child_index].state.active_player;
            if seen[..=depth].contains(&fingerprint) {
                break (0.0, child_player);
            }
            seen[depth + 1] = fingerprint;
            node_index = child_index;
        };

        while let Some((parent_index, edge_index)) = self.simulation_path.pop() {
            let parent_player = self.nodes[parent_index].state.active_player;
            if value_player != parent_player {
                value = -value;
            }
            let edge = &mut self.nodes[parent_index].edges[edge_index];
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

    /// Mixes symmetric Dirichlet noise into root priors for self-play search.
    ///
    /// `alpha` is the concentration of each legal action and `epsilon` is the
    /// fraction of the noisy prior. Original network priors remain available
    /// for zero-visit policy targets.
    pub fn add_root_dirichlet_noise(&mut self, alpha: f32, epsilon: f32, seed: u64) {
        assert!(
            alpha.is_finite() && alpha > 0.0,
            "Dirichlet alpha must be finite and positive"
        );
        assert!(
            epsilon.is_finite() && (0.0..=1.0).contains(&epsilon),
            "Dirichlet epsilon must be finite and in [0, 1]"
        );
        if self.root_node().edges.is_empty() {
            return;
        }
        if epsilon == 0.0 {
            for edge in &mut self.root_node_mut().edges {
                edge.prior = edge.network_prior;
            }
            return;
        }

        let action_count = self.root_node().edges.len();
        debug_assert!(action_count <= MAX_LEGAL_ACTIONS);
        let mut rng = Rng::new(seed);
        let mut noise = [0.0_f64; MAX_LEGAL_ACTIONS];
        let mut total = 0.0;
        for sample in &mut noise[..action_count] {
            *sample = rng.gamma(f64::from(alpha));
            total += *sample;
        }
        debug_assert!(total.is_finite() && total > 0.0);
        let clean_fraction = 1.0 - epsilon;
        for (edge, sample) in self.root_node_mut().edges.iter_mut().zip(noise) {
            edge.prior = clean_fraction * edge.network_prior + epsilon * (sample / total) as f32;
        }
    }

    /// Re-roots after an atomic action while retaining the explored child tree.
    ///
    /// Nodes from discarded sibling trees are recycled by later simulations.
    pub fn advance(&mut self, action: Action, model: &Model) -> bool {
        let Some(edge_index) = self
            .root_node()
            .edges
            .iter()
            .position(|edge| edge.action == action)
        else {
            return false;
        };

        let old_root = self.root;
        let child = if let Some(child) = self.nodes[old_root].edges[edge_index].child.take() {
            child
        } else {
            let Ok(state) = self.nodes[old_root].state.clone().apply(action) else {
                return false;
            };
            self.allocate_node(state)
        };
        self.root = child;
        self.reclaim_subtree(old_root);
        if !self.root_node().expanded {
            let root = self.root;
            self.nodes[root].expand(model);
        }
        self.simulations = 0;
        true
    }

    pub fn root_action_count(&self) -> usize {
        self.root_node().edges.len()
    }

    pub fn root_value(&self) -> f32 {
        let root = self.root_node();
        if root.edges.is_empty() {
            return root.value;
        }
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

    pub fn policy(&self, temperature: f32) -> Vec<(Action, f32)> {
        let root = self.root_node();
        if root.edges.is_empty() {
            return Vec::new();
        }
        let total_visits = root
            .edges
            .iter()
            .map(|edge| u64::from(edge.visits))
            .sum::<u64>();
        let base = |edge: &Edge| {
            if total_visits == 0 {
                edge.network_prior
            } else {
                edge.visits as f32
            }
        };
        if !temperature.is_finite() || temperature <= ARGMAX_TEMPERATURE {
            let best = root
                .edges
                .iter()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    base(left)
                        .total_cmp(&base(right))
                        .then_with(|| left.network_prior.total_cmp(&right.network_prior))
                })
                .map(|(index, _)| index)
                .unwrap_or(0);
            return root
                .edges
                .iter()
                .enumerate()
                .map(|(index, edge)| (edge.action, f32::from(index == best)))
                .collect();
        }

        let inverse_temperature = 1.0 / temperature;
        let maximum = root
            .edges
            .iter()
            .map(|edge| {
                let weight = base(edge);
                if weight > 0.0 {
                    weight.ln() * inverse_temperature
                } else {
                    f32::NEG_INFINITY
                }
            })
            .fold(f32::NEG_INFINITY, f32::max);
        let mut policy = Vec::with_capacity(root.edges.len());
        let mut total = 0.0;
        for edge in &root.edges {
            let weight = base(edge);
            let probability = if weight > 0.0 {
                (weight.ln() * inverse_temperature - maximum).exp()
            } else {
                0.0
            };
            total += probability;
            policy.push((edge.action, probability));
        }
        debug_assert!(total.is_finite() && total > 0.0);
        for (_, probability) in &mut policy {
            *probability /= total;
        }
        policy
    }

    pub fn best_complete_line(&self, model: &Model) -> Vec<Action> {
        let mut ply_player = self.root_node().state.active_player;
        let mut state = self.root_node().state.clone();
        let mut actions = Vec::new();
        let mut completed_actions = 0;
        let mut current_index = self.root;
        while actions.len() < MAX_SEARCH_DEPTH {
            let Some(index) = self.nodes[current_index].preferred_edge() else {
                break;
            };
            let child = self.nodes[current_index].edges[index].child;
            if completed_actions > 0 && child.is_none() {
                break;
            }
            let action = self.nodes[current_index].edges[index].action;
            let Ok(next) = state.clone().apply(action) else {
                break;
            };
            actions.push(action);
            state = next;
            if state.active_player != ply_player || state.winner().is_some() {
                completed_actions = actions.len();
                ply_player = state.active_player;
            }
            if state.winner().is_some() {
                break;
            }
            if let Some(child) = child {
                current_index = child;
            } else {
                if completed_actions > 0 {
                    break;
                }
                // Preserve the Analyzer contract even before MCTS has visited
                // enough actions to finish the root ply. Deeper speculative
                // actions are deliberately omitted from the published line.
                let mut temporary = Node::new(state.clone());
                temporary.expand(model);
                if let Some(next_index) = temporary.preferred_edge() {
                    let action = temporary.edges[next_index].action;
                    if let Ok(next) = state.clone().apply(action) {
                        actions.push(action);
                        state = next;
                        if state.active_player != ply_player || state.winner().is_some() {
                            completed_actions = actions.len();
                        }
                    }
                }
                break;
            }
        }
        actions.truncate(completed_actions);
        actions
    }
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
        if self.is_fully_solved().is_some() {
            return;
        }
        if let Some(search) = &mut self.search {
            search.simulate(&self.model);
        }
    }

    fn is_fully_solved(&self) -> Option<OptimalOutcome> {
        let winner = self.state.as_ref()?.winner()?;
        Some(OptimalOutcome::MateInN(
            MateInN::new(winner, 0).expect("zero is a supported mate distance"),
        ))
    }

    fn evaluation(&self) -> Evaluation {
        let Some(state) = &self.state else {
            return estimate(0.0);
        };
        if let Some(winner) = state.winner() {
            return mate_in(winner, 0);
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
        let actions = search.best_complete_line(&self.model);
        writer.reserve(actions.len());
        for action in actions {
            writer.push(action);
        }
    }
}

fn mate_in(winner: Player, plies: u32) -> Evaluation {
    MateInN::new(winner, plies)
        .expect("reported mate distance is within the supported range")
        .into()
}

fn estimate(value: f64) -> Evaluation {
    assert!(value.is_finite(), "model evaluation must be finite");
    let millipoints = (value * 1_000.0).round().clamp(
        f64::from(EvaluationEstimate::MIN.millipoints()),
        f64::from(EvaluationEstimate::MAX.millipoints()),
    ) as i32;
    EvaluationEstimate::from_millipoints(millipoints)
        .expect("clamped model evaluation is in range")
        .into()
}

struct ActionBuffer {
    actions: [Action; MAX_LEGAL_ACTIONS],
    len: usize,
}

impl ActionBuffer {
    fn new() -> Self {
        Self {
            actions: [Action::SnipeStep(snipe_core::SnipeStep {
                destination: Rank::R1,
            }); MAX_LEGAL_ACTIONS],
            len: 0,
        }
    }

    fn as_slice(&self) -> &[Action] {
        &self.actions[..self.len]
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl ActionWriter for ActionBuffer {
    fn push(&mut self, action: Action) {
        assert!(
            self.len < MAX_LEGAL_ACTIONS,
            "legal action count exceeds policy head size"
        );
        self.actions[self.len] = action;
        self.len += 1;
    }

    fn reserve(&mut self, additional: usize) {
        assert!(
            self.len + additional <= MAX_LEGAL_ACTIONS,
            "legal action count exceeds policy head size"
        );
    }
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
    let animals = animals();
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
    let locations = [
        &state.reserves,
        &state.r1,
        &state.r2,
        &state.r3,
        &state.r4,
        &state.r5,
        &state.r6,
    ];
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    hash = hash_byte(hash, u8::from(state.active_player == Player::Beta));
    for cards in locations {
        for animal in animals() {
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
        hash = hash_byte(hash, rank_index(leading.destination) as u8);
    } else {
        hash = hash_byte(hash, 0);
    }
    hash
}

/// Compact, allocation-free state identity for replay/cycle detection.
pub fn state_key(state: &State) -> u64 {
    state_fingerprint(state)
}

fn hash_byte(hash: u64, value: u8) -> u64 {
    (hash ^ u64::from(value)).wrapping_mul(0x100_0000_01B3)
}

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

    fn unit_open_f64(&mut self) -> f64 {
        // Taking the high 53 bits and adding half a unit keeps both endpoints
        // open, which is required by logarithmic samplers.
        ((self.next_u64() >> 11) as f64 + 0.5) * (1.0 / ((1_u64 << 53) as f64))
    }

    fn standard_normal(&mut self) -> f64 {
        let radius = (-2.0 * self.unit_open_f64().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.unit_open_f64();
        radius * angle.cos()
    }

    /// Marsaglia-Tsang Gamma(shape, 1), including the shape < 1 transform.
    fn gamma(&mut self, shape: f64) -> f64 {
        debug_assert!(shape.is_finite() && shape > 0.0);
        if shape < 1.0 {
            return self.gamma(shape + 1.0) * self.unit_open_f64().powf(1.0 / shape);
        }

        let d = shape - 1.0 / 3.0;
        let c = 1.0 / (9.0 * d).sqrt();
        loop {
            let x = self.standard_normal();
            let candidate = 1.0 + c * x;
            if candidate <= 0.0 {
                continue;
            }
            let candidate_cubed = candidate * candidate * candidate;
            let uniform = self.unit_open_f64();
            if uniform < 1.0 - 0.0331 * x * x * x * x
                || uniform.ln() < 0.5 * x * x + d * (1.0 - candidate_cubed + candidate_cubed.ln())
            {
                return d * candidate_cubed;
            }
        }
    }

    fn normalish(&mut self) -> f32 {
        (0..6).map(|_| self.unit()).sum::<f32>() - 3.0
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

    pub struct Adam {
        first: Vec<f32>,
        second: Vec<f32>,
        gradient: Vec<f32>,
        step: u64,
    }

    impl Adam {
        pub fn new() -> Self {
            Self {
                first: vec![0.0; PARAM_COUNT],
                second: vec![0.0; PARAM_COUNT],
                gradient: vec![0.0; PARAM_COUNT],
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
                gradient: vec![0.0; PARAM_COUNT],
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
            optimizer.gradient.fill(0.0);
            let gradient = &mut optimizer.gradient;
            let mut total_loss = 0.0;
            for sample in samples {
                let mut forward = self.forward(&sample.input);
                let max_logit = forward
                    .logits
                    .iter()
                    .copied()
                    .fold(f32::NEG_INFINITY, f32::max);
                for logit in &mut forward.logits {
                    *logit = (*logit - max_logit).exp();
                }
                let denominator = forward.logits.iter().sum::<f32>().max(f32::MIN_POSITIVE);
                for probability in &mut forward.logits {
                    *probability /= denominator;
                }
                for (action, probability) in forward.logits.iter_mut().enumerate() {
                    let target = sample.policy[action];
                    if target > 0.0 {
                        total_loss -= target * probability.max(1e-12).ln();
                    }
                    *probability -= target;
                }
                let value_error = forward.value - sample.value;
                total_loss += value_error * value_error;
                let value_delta = 2.0 * value_error * (1.0 - forward.value * forward.value);

                let mut hidden_2_gradient = [0.0; HIDDEN_SIZE];
                for hidden in 0..HIDDEN_SIZE {
                    let weights = &self.parameters
                        [WP + hidden * ACTION_SIZE..WP + (hidden + 1) * ACTION_SIZE];
                    let weight_gradient =
                        &mut gradient[WP + hidden * ACTION_SIZE..WP + (hidden + 1) * ACTION_SIZE];
                    let activation = forward.hidden_2[hidden];
                    for action in 0..ACTION_SIZE {
                        weight_gradient[action] =
                            activation.mul_add(forward.logits[action], weight_gradient[action]);
                        hidden_2_gradient[hidden] = weights[action]
                            .mul_add(forward.logits[action], hidden_2_gradient[hidden]);
                    }
                    gradient[WV + hidden] += forward.hidden_2[hidden] * value_delta;
                    hidden_2_gradient[hidden] += self.parameters[WV + hidden] * value_delta;
                }
                for action in 0..ACTION_SIZE {
                    gradient[BP + action] += forward.logits[action];
                }
                gradient[BV] += value_delta;

                for output in 0..HIDDEN_SIZE {
                    if forward.hidden_2[output] <= 0.0 {
                        hidden_2_gradient[output] = 0.0;
                    }
                    gradient[BR + output] += hidden_2_gradient[output];
                }

                let mut hidden_1_gradient = hidden_2_gradient;
                for input in 0..HIDDEN_SIZE {
                    let weights =
                        &self.parameters[WR + input * HIDDEN_SIZE..WR + (input + 1) * HIDDEN_SIZE];
                    let weight_gradient =
                        &mut gradient[WR + input * HIDDEN_SIZE..WR + (input + 1) * HIDDEN_SIZE];
                    let activation = forward.hidden_1[input];
                    let mut input_gradient = hidden_1_gradient[input];
                    for output in 0..HIDDEN_SIZE {
                        weight_gradient[output] =
                            activation.mul_add(hidden_2_gradient[output], weight_gradient[output]);
                        input_gradient =
                            weights[output].mul_add(hidden_2_gradient[output], input_gradient);
                    }
                    hidden_1_gradient[input] = input_gradient;
                }

                for output in 0..HIDDEN_SIZE {
                    if forward.hidden_1[output] <= 0.0 {
                        hidden_1_gradient[output] = 0.0;
                    }
                    gradient[B1 + output] += hidden_1_gradient[output];
                }
                for input in 0..INPUT_SIZE {
                    let activation = sample.input[input];
                    if activation != 0.0 {
                        let weight_gradient =
                            &mut gradient[W1 + input * HIDDEN_SIZE..W1 + (input + 1) * HIDDEN_SIZE];
                        for hidden in 0..HIDDEN_SIZE {
                            weight_gradient[hidden] = activation
                                .mul_add(hidden_1_gradient[hidden], weight_gradient[hidden]);
                        }
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
    use snipe_core::CardMultiset;
    use snipe_prng::initial_state;
    use std::collections::HashSet;

    fn cards(entries: &[(Card, Player)]) -> CardMultiset {
        entries
            .iter()
            .fold(CardMultiset::EMPTY, |cards, &(card, player)| {
                cards
                    .checked_add(CardMultiset::singleton(card, player))
                    .expect("fixture card multiplicities are valid")
            })
    }

    fn reference_forward(model: &Model, input: &[f32; INPUT_SIZE]) -> Forward {
        let mut hidden_1 = [0.0; HIDDEN_SIZE];
        for (output, hidden) in hidden_1.iter_mut().enumerate() {
            let mut sum = model.parameters[B1 + output];
            for (index, &value) in input.iter().enumerate() {
                sum += value * model.parameters[W1 + index * HIDDEN_SIZE + output];
            }
            *hidden = sum.max(0.0);
        }
        let mut hidden_2 = [0.0; HIDDEN_SIZE];
        for output in 0..HIDDEN_SIZE {
            let mut sum = hidden_1[output] + model.parameters[BR + output];
            for (index, &value) in hidden_1.iter().enumerate() {
                sum += value * model.parameters[WR + index * HIDDEN_SIZE + output];
            }
            hidden_2[output] = sum.max(0.0);
        }
        let mut logits = [0.0; ACTION_SIZE];
        for (action, logit) in logits.iter_mut().enumerate() {
            let mut sum = model.parameters[BP + action];
            for (index, &value) in hidden_2.iter().enumerate() {
                sum += value * model.parameters[WP + index * ACTION_SIZE + action];
            }
            *logit = sum;
        }
        let mut raw_value = model.parameters[BV];
        for (index, &value) in hidden_2.iter().enumerate() {
            raw_value += value * model.parameters[WV + index];
        }
        Forward {
            hidden_1,
            hidden_2,
            logits,
            value: raw_value.tanh(),
        }
    }

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
    fn policy_uses_network_priors_until_an_edge_is_visited() {
        let model = Model::seeded(7);
        let mut search = Search::new(initial_state(9), &model);
        let network_priors = search
            .root_node()
            .edges
            .iter()
            .map(|edge| edge.network_prior)
            .collect::<Vec<_>>();

        // Root noise guides search but must not leak into a zero-visit target.
        search.add_root_dirichlet_noise(0.3, 0.25, 1234);
        let fallback = search.policy(1.0);
        for ((_, actual), expected) in fallback.iter().zip(&network_priors) {
            assert!((actual - expected).abs() < 1e-7);
        }

        search.simulate(&model);
        let visited = search.policy(1.0);
        assert_eq!(
            visited
                .iter()
                .filter(|(_, probability)| *probability > 0.0)
                .count(),
            1,
            "unvisited actions must have zero visit-policy mass"
        );
        assert!(
            (visited
                .iter()
                .map(|(_, probability)| probability)
                .sum::<f32>()
                - 1.0)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn low_temperature_policy_is_finite_and_normalized() {
        let model = Model::seeded(11);
        let mut search = Search::new(initial_state(4), &model);
        let root = search.root_node_mut();
        root.edges[0].visits = 100;
        root.edges[1].visits = 99;

        let policy = search.policy(0.05);
        assert!(
            policy
                .iter()
                .all(|(_, probability)| probability.is_finite())
        );
        assert!(
            (policy
                .iter()
                .map(|(_, probability)| probability)
                .sum::<f32>()
                - 1.0)
                .abs()
                < 1e-6
        );
        assert!(policy[0].1 > policy[1].1);
        assert!(
            policy
                .iter()
                .skip(2)
                .all(|(_, probability)| *probability == 0.0)
        );
    }

    #[test]
    fn dirichlet_noise_is_seeded_normalized_and_preserves_network_priors() {
        let model = Model::seeded(17);
        let mut first = Search::new(initial_state(5), &model);
        let mut second = Search::new(initial_state(5), &model);
        let clean = first
            .root_node()
            .edges
            .iter()
            .map(|edge| edge.network_prior)
            .collect::<Vec<_>>();

        first.add_root_dirichlet_noise(0.3, 0.25, 0xD1A1_C4E7);
        second.add_root_dirichlet_noise(0.3, 0.25, 0xD1A1_C4E7);
        let first_priors = first
            .root_node()
            .edges
            .iter()
            .map(|edge| edge.prior)
            .collect::<Vec<_>>();
        let second_priors = second
            .root_node()
            .edges
            .iter()
            .map(|edge| edge.prior)
            .collect::<Vec<_>>();

        assert_eq!(first_priors, second_priors);
        assert!(
            first_priors
                .iter()
                .zip(&clean)
                .any(|(noisy, original)| noisy != original)
        );
        assert!((first_priors.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        assert_eq!(
            first
                .root_node()
                .edges
                .iter()
                .map(|edge| edge.network_prior)
                .collect::<Vec<_>>(),
            clean
        );
    }

    #[test]
    fn gamma_sampler_has_expected_dirichlet_moments() {
        const COMPONENTS: usize = 3;
        const SAMPLES: usize = 10_000;
        let alpha = 0.5;
        let mut rng = Rng::new(0xD1A1_C4E7_5EED);
        let mut sum = 0.0;
        let mut square_sum = 0.0;
        for _ in 0..SAMPLES {
            let samples = [rng.gamma(alpha), rng.gamma(alpha), rng.gamma(alpha)];
            let component = samples[0] / samples.iter().sum::<f64>();
            sum += component;
            square_sum += component * component;
        }
        let mean = sum / SAMPLES as f64;
        let variance = square_sum / SAMPLES as f64 - mean * mean;
        let expected_mean = 1.0 / COMPONENTS as f64;
        let expected_variance = (COMPONENTS - 1) as f64
            / (COMPONENTS * COMPONENTS) as f64
            / (COMPONENTS as f64 * alpha + 1.0);
        assert!((mean - expected_mean).abs() < 0.01, "{mean}");
        assert!(
            (variance - expected_variance).abs() < 0.01,
            "{variance} != {expected_variance}"
        );
    }

    #[test]
    fn advance_retains_the_selected_arena_subtree_and_recycles_siblings() {
        let model = Model::seeded(23);
        let mut search = Search::new(initial_state(19), &model);
        search.simulate_n(&model, 48);
        let edge_index = search.root_node().preferred_edge().unwrap();
        let action = search.root_node().edges[edge_index].action;
        let child = search.root_node().edges[edge_index]
            .child
            .expect("a visited edge has a child");
        let retained_action_count = search.nodes[child].edges.len();

        assert!(search.advance(action, &model));
        assert_eq!(search.root, child);
        assert_eq!(search.root_action_count(), retained_action_count);
        assert!(!search.free_nodes.is_empty());

        let allocated_before = search.nodes.len();
        search.simulate_n(&model, 16);
        assert_eq!(
            search.nodes.len(),
            allocated_before,
            "discarded arena slots should be reused before the arena grows"
        );
    }

    fn swap_allegiances(cards: CardMultiset) -> CardMultiset {
        let mut swapped = CardMultiset::EMPTY;
        for card in animals()
            .into_iter()
            .map(Card::Animal)
            .chain(std::iter::once(Card::Snipe))
        {
            for _ in 0..cards.count(card, Player::Alpha) {
                swapped = swapped
                    .checked_add(CardMultiset::singleton(card, Player::Beta))
                    .unwrap();
            }
            for _ in 0..cards.count(card, Player::Beta) {
                swapped = swapped
                    .checked_add(CardMultiset::singleton(card, Player::Alpha))
                    .unwrap();
            }
        }
        swapped
    }

    fn opposite_rank(rank: Rank) -> Rank {
        match rank {
            Rank::R1 => Rank::R6,
            Rank::R2 => Rank::R5,
            Rank::R3 => Rank::R4,
            Rank::R4 => Rank::R3,
            Rank::R5 => Rank::R2,
            Rank::R6 => Rank::R1,
        }
    }

    #[test]
    fn state_keys_distinguish_raw_mirror_swapped_positions() {
        let state = initial_state(31);
        let mirrored = State {
            active_player: state.active_player.opponent(),
            reserves: swap_allegiances(state.reserves),
            r1: swap_allegiances(state.r6),
            r2: swap_allegiances(state.r5),
            r3: swap_allegiances(state.r4),
            r4: swap_allegiances(state.r3),
            r5: swap_allegiances(state.r2),
            r6: swap_allegiances(state.r1),
            leading_action: state.leading_action.map(|leading| snipe_core::AnimalStep {
                actor: leading.actor,
                direction: leading.direction,
                destination: opposite_rank(leading.destination),
            }),
        };
        assert_eq!(
            encode_state(&state),
            encode_state(&mirrored),
            "the network intentionally canonicalizes these positions"
        );
        assert_ne!(
            state_key(&state),
            state_key(&mirrored),
            "cycle detection must use raw, non-canonical position identity"
        );
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
        let mut ply_player = state.active_player;
        let mut after = state;
        let mut completed_actions = 0;
        for (index, &action) in actions.iter().enumerate() {
            after = after.apply(action).unwrap();
            if after.active_player != ply_player || after.winner().is_some() {
                completed_actions = index + 1;
                ply_player = after.active_player;
            }
        }
        assert_eq!(completed_actions, actions.len());
    }

    #[test]
    fn best_line_follows_complete_plies_through_the_explored_tree() {
        let model = Model::seeded(31);
        let mut state = initial_state(7071);
        let mut search = Search::new(state.clone(), &model);
        let mut node = search.root;
        let mut ply_player = state.active_player;
        let mut completed_plies = 0;
        let mut expected = Vec::new();

        while completed_plies < 3 {
            let edge_index = search.nodes[node].preferred_edge().unwrap();
            let action = search.nodes[node].edges[edge_index].action;
            let next = state
                .clone()
                .apply(action)
                .expect("preferred action is legal");
            assert_eq!(next.winner(), None, "fixture ended before three plies");
            let child = search.allocate_node(next.clone());
            search.nodes[child].expand(&model);
            let edge = &mut search.nodes[node].edges[edge_index];
            edge.visits = 100;
            edge.child = Some(child);
            expected.push(action);
            if next.active_player != ply_player {
                completed_plies += 1;
                ply_player = next.active_player;
            }
            state = next;
            node = child;
        }

        assert_eq!(search.best_complete_line(&model), expected);
    }

    #[test]
    fn thinking_builds_a_multi_ply_principal_line() {
        let model = Model::seeded(31);
        let mut state = initial_state(7071);
        let mut search = Search::new(state.clone(), &model);
        search.simulate_n(&model, 512);
        let line = search.best_complete_line(&model);
        let mut ply_player = state.active_player;
        let mut completed_plies = 0;
        for action in line {
            state = state.apply(action).expect("published action is legal");
            if state.active_player != ply_player || state.winner().is_some() {
                completed_plies += 1;
                ply_player = state.active_player;
            }
        }
        assert!(completed_plies > 1);
    }

    #[test]
    fn public_estimates_are_rounded_and_bounded_millipoints() {
        assert_eq!(
            estimate(0.1236),
            EvaluationEstimate::from_millipoints(124).unwrap().into()
        );
        assert_eq!(estimate(1_000.0), EvaluationEstimate::MAX.into());
        assert_eq!(estimate(-1_000.0), EvaluationEstimate::MIN.into());
    }

    #[test]
    fn search_fixture_finds_a_short_triplet_capture_without_a_mate_overlay() {
        use Animal::{Dog, Dragon, Fish, Horse, Mouse, Ox, Rooster, Tiger};

        // Rooster entering r3 completes the fire unary/binary/ternary triplet
        // with Horse and Tiger and captures Beta's Snipe. Extra legal drops and
        // steps make this a search fixture rather than a single-action state.
        let state = State {
            active_player: Player::Alpha,
            reserves: cards(&[
                (Card::Animal(Dragon), Player::Alpha),
                (Card::Animal(Fish), Player::Alpha),
            ]),
            r1: cards(&[
                (Card::Snipe, Player::Alpha),
                (Card::Animal(Dog), Player::Alpha),
            ]),
            r2: cards(&[
                (Card::Animal(Rooster), Player::Alpha),
                (Card::Animal(Mouse), Player::Alpha),
            ]),
            r3: cards(&[
                (Card::Animal(Horse), Player::Beta),
                (Card::Animal(Tiger), Player::Beta),
                (Card::Snipe, Player::Beta),
            ]),
            r4: cards(&[(Card::Animal(Ox), Player::Alpha)]),
            r5: CardMultiset::EMPTY,
            r6: CardMultiset::EMPTY,
            leading_action: None,
        };
        assert_eq!(state.winner(), None);

        let model = Model::seeded(0x7AC7_1CA1);
        let mut search = Search::new(state.clone(), &model);
        search.simulate_n(&model, 2_048);
        let actions = search.best_complete_line(&model);
        assert!(!actions.is_empty());

        let mut after = state;
        for action in actions {
            after = after.apply(action).expect("search fixture action is legal");
        }
        assert_eq!(
            after.winner(),
            Some(Player::Alpha),
            "corrected MCTS should discover the short terminal tactic"
        );
    }

    #[test]
    fn checkpoints_round_trip() {
        let model = Model::seeded(42);
        let rebuilt = Model::from_bytes(&model.to_bytes()).unwrap();
        let state = initial_state(4);
        assert_eq!(model.predict(&state), rebuilt.predict(&state));
    }

    #[test]
    fn optimized_forward_matches_reference() {
        let model = Model::seeded(0xA11C_E5E5);
        for seed in 0..16 {
            let input = encode_state(&initial_state(seed));
            let optimized = model.forward(&input);
            let reference = reference_forward(&model, &input);
            for (actual, expected) in optimized.logits.iter().zip(reference.logits) {
                assert!((actual - expected).abs() < 2e-6, "{actual} != {expected}");
            }
            assert!((optimized.value - reference.value).abs() < 2e-6);
        }
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
