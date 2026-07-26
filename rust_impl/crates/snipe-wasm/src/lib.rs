//! Clean browser bridge for `snipe-core`, Avocado, and Blueberry.

use agent_avocado::AvocadoAnalyzer;
use agent_blueberry::BlueberryAnalyzer;
use serde::{Deserialize, Serialize};
use snipe_core::{
    Action, Analyzer, Animal, AnimalDrop, Card, CardMultiset, Evaluation, InitialStateBuilder,
    Player, Rank, SnipeStep, State, StepDirection,
};
use wasm_bindgen::prelude::*;

const DEFAULT_SEED: u32 = 7_071;
const MAX_TIME_MS: u64 = 120_000;
const BATCH_TICKS: usize = 8;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Strategy {
    Avocado,
    Blueberry,
}

impl Strategy {
    fn label(self) -> &'static str {
        match self {
            Self::Avocado => "Avocado",
            Self::Blueberry => "Blueberry",
        }
    }
}

enum BrowserAnalyzer {
    Avocado(AvocadoAnalyzer),
    Blueberry(BlueberryAnalyzer),
}

impl BrowserAnalyzer {
    fn new(strategy: Strategy, state: State) -> Self {
        match strategy {
            Strategy::Avocado => {
                let mut analyzer = AvocadoAnalyzer::new();
                analyzer.set_state(state);
                Self::Avocado(analyzer)
            }
            Strategy::Blueberry => {
                let mut analyzer = BlueberryAnalyzer::new();
                analyzer.set_state(state);
                Self::Blueberry(analyzer)
            }
        }
    }

    fn think(&mut self, ticks: usize) {
        match self {
            Self::Avocado(analyzer) => analyzer.think(ticks),
            Self::Blueberry(analyzer) => analyzer.think(ticks),
        }
    }

    fn evaluation(&self) -> Evaluation {
        match self {
            Self::Avocado(analyzer) => analyzer.evaluation(),
            Self::Blueberry(analyzer) => analyzer.evaluation(),
        }
    }

    fn line(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        match self {
            Self::Avocado(analyzer) => analyzer.write_optimal_lop(&mut actions),
            Self::Blueberry(analyzer) => analyzer.write_optimal_lop(&mut actions),
        }
        actions
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum PlayerDto {
    Alpha,
    Beta,
}

impl From<Player> for PlayerDto {
    fn from(value: Player) -> Self {
        match value {
            Player::Alpha => Self::Alpha,
            Player::Beta => Self::Beta,
        }
    }
}

impl From<PlayerDto> for Player {
    fn from(value: PlayerDto) -> Self {
        match value {
            PlayerDto::Alpha => Self::Alpha,
            PlayerDto::Beta => Self::Beta,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardDto {
    id: String,
    animal: String,
    owner: PlayerDto,
    is_snipe: bool,
    can_retreat: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocationsDto {
    #[serde(rename = "alpha-reserve")]
    alpha_reserve: Vec<CardDto>,
    #[serde(rename = "beta-reserve")]
    beta_reserve: Vec<CardDto>,
    #[serde(rename = "row-1")]
    row_1: Vec<CardDto>,
    #[serde(rename = "row-2")]
    row_2: Vec<CardDto>,
    #[serde(rename = "row-3")]
    row_3: Vec<CardDto>,
    #[serde(rename = "row-4")]
    row_4: Vec<CardDto>,
    #[serde(rename = "row-5")]
    row_5: Vec<CardDto>,
    #[serde(rename = "row-6")]
    row_6: Vec<CardDto>,
}

impl LocationsDto {
    fn get(&self, location: Location) -> &[CardDto] {
        match location {
            Location::AlphaReserve => &self.alpha_reserve,
            Location::R1 => &self.row_1,
            Location::R2 => &self.row_2,
            Location::R3 => &self.row_3,
            Location::R4 => &self.row_4,
            Location::R5 => &self.row_5,
            Location::R6 => &self.row_6,
            Location::BetaReserve => &self.beta_reserve,
        }
    }

    fn get_mut(&mut self, location: Location) -> &mut Vec<CardDto> {
        match location {
            Location::AlphaReserve => &mut self.alpha_reserve,
            Location::R1 => &mut self.row_1,
            Location::R2 => &mut self.row_2,
            Location::R3 => &mut self.row_3,
            Location::R4 => &mut self.row_4,
            Location::R5 => &mut self.row_5,
            Location::R6 => &mut self.row_6,
            Location::BetaReserve => &mut self.beta_reserve,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionDto {
    schema_version: u8,
    seed: u64,
    turn: PlayerDto,
    turn_number: u32,
    winner: Option<PlayerDto>,
    locations: LocationsDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveStepDto {
    card_id: String,
    from: String,
    to: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TurnMoveDto {
    id: String,
    player: PlayerDto,
    label: String,
    steps: Vec<MoveStepDto>,
    captures: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisRequestDto {
    position: PositionDto,
    time_limit_ms: u64,
    request_id: u64,
    strategy: Strategy,
    #[serde(default)]
    first_step: Option<MoveStepDto>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum EvaluationDto {
    Mate { winner: PlayerDto, plies: usize },
    Estimate { value: f64 },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisUpdateDto {
    request_id: u64,
    best_move: TurnMoveDto,
    evaluation: EvaluationDto,
    ticks: u64,
    elapsed_ms: u64,
    recommended_line: Vec<TurnMoveDto>,
    strategy: Strategy,
    engine_name: &'static str,
}

#[derive(Clone, Copy)]
enum Location {
    AlphaReserve,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    BetaReserve,
}

const LOCATIONS: [Location; 8] = [
    Location::AlphaReserve,
    Location::R1,
    Location::R2,
    Location::R3,
    Location::R4,
    Location::R5,
    Location::R6,
    Location::BetaReserve,
];
const STATE_LOCATIONS: [Location; 7] = [
    Location::AlphaReserve,
    Location::R1,
    Location::R2,
    Location::R3,
    Location::R4,
    Location::R5,
    Location::R6,
];

#[wasm_bindgen]
pub fn create_game(seed: Option<u32>) -> Result<String, JsValue> {
    let seed = seed.unwrap_or(DEFAULT_SEED);
    let state = initial_state(u64::from(seed));
    encode(&state_to_dto(&state, u64::from(seed), 1, None))
}

#[wasm_bindgen]
pub fn legal_moves(position_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let state = dto_to_state(&position)?;
    let moves = full_turns(&state)
        .into_iter()
        .map(|actions| actions_to_dto(&position, &state, &actions))
        .collect::<Result<Vec<_>, _>>()?;
    encode(&moves)
}

#[wasm_bindgen]
pub fn preview_first_step(position_json: &str, step_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let requested: MoveStepDto = decode(step_json)?;
    let state = dto_to_state(&position)?;
    let action = find_first_action(&position, &state, &requested)?;
    let next = state
        .apply(action)
        .map_err(|error| js_error(format!("illegal first step: {error:?}")))?;
    encode(&state_to_dto(
        &next,
        position.seed,
        position.turn_number,
        Some(&position),
    ))
}

#[wasm_bindgen]
pub fn apply_move(position_json: &str, move_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let requested: TurnMoveDto = decode(move_json)?;
    let state = dto_to_state(&position)?;
    let actions = full_turns(&state)
        .into_iter()
        .find(|actions| action_id(actions) == requested.id)
        .ok_or_else(|| js_error("move is not legal in this position"))?;
    let next = execute(state, &actions)?;
    encode(&state_to_dto(
        &next,
        position.seed,
        position.turn_number.saturating_add(1),
        Some(&position),
    ))
}

#[wasm_bindgen]
pub fn analyze(request_json: &str) -> Result<String, JsValue> {
    let request: AnalysisRequestDto = decode(request_json)?;
    let result = run_analysis(&request, None)?;
    encode(&result)
}

#[wasm_bindgen]
pub fn analyze_live(request_json: &str, on_progress: &js_sys::Function) -> Result<String, JsValue> {
    let request: AnalysisRequestDto = decode(request_json)?;
    let result = run_analysis(&request, Some(on_progress))?;
    encode(&result)
}

fn run_analysis(
    request: &AnalysisRequestDto,
    callback: Option<&js_sys::Function>,
) -> Result<AnalysisUpdateDto, JsValue> {
    let base = dto_to_state(&request.position)?;
    let state = if let Some(step) = &request.first_step {
        let action = find_first_action(&request.position, &base, step)?;
        base.apply(action)
            .map_err(|error| js_error(format!("illegal first step: {error:?}")))?
    } else {
        base
    };
    let mut analyzer = BrowserAnalyzer::new(request.strategy, state.clone());
    let start = js_sys::Date::now();
    let deadline = start + request.time_limit_ms.clamp(1, MAX_TIME_MS) as f64;
    let mut ticks = 0u64;
    let mut last_progress = start;
    while js_sys::Date::now() < deadline {
        analyzer.think(BATCH_TICKS);
        ticks += BATCH_TICKS as u64;
        let now = js_sys::Date::now();
        if let Some(callback) = callback
            && now - last_progress >= 75.0
        {
            let update = analysis_update(request, &state, &analyzer, ticks, now - start)?;
            let json =
                serde_json::to_string(&update).map_err(|error| js_error(error.to_string()))?;
            callback.call1(&JsValue::NULL, &JsValue::from_str(&json))?;
            last_progress = now;
        }
    }
    analysis_update(
        request,
        &state,
        &analyzer,
        ticks,
        js_sys::Date::now() - start,
    )
}

fn analysis_update(
    request: &AnalysisRequestDto,
    analyzed_state: &State,
    analyzer: &BrowserAnalyzer,
    ticks: u64,
    elapsed: f64,
) -> Result<AnalysisUpdateDto, JsValue> {
    let actions = analyzer.line();
    if actions.is_empty() {
        return Err(js_error("no legal moves are available"));
    }
    let position = if request.first_step.is_some() {
        state_to_dto(
            analyzed_state,
            request.position.seed,
            request.position.turn_number,
            Some(&request.position),
        )
    } else {
        request.position.clone()
    };
    let best_move = actions_to_dto(&position, analyzed_state, &actions)?;
    Ok(AnalysisUpdateDto {
        request_id: request.request_id,
        best_move: best_move.clone(),
        evaluation: evaluation_dto(analyzer.evaluation()),
        ticks,
        elapsed_ms: elapsed.max(0.0) as u64,
        recommended_line: vec![best_move],
        strategy: request.strategy,
        engine_name: request.strategy.label(),
    })
}

fn initial_state(seed: u64) -> State {
    let mut deck = [Animal::Mouse; 32];
    for (index, slot) in deck.iter_mut().enumerate() {
        *slot = animals()[index % 16];
    }
    let mut rng = seed ^ 0x9E37_79B9_7F4A_7C15;
    for index in (1..deck.len()).rev() {
        rng = splitmix64(rng);
        deck.swap(index, (rng as usize) % (index + 1));
    }
    InitialStateBuilder {
        alpha_reserve: [deck[0]],
        r1: [deck[1], deck[2]],
        r2: deck[3..15].try_into().expect("fixed slice"),
        r3: [deck[15]],
        r4: [deck[16]],
        r5: deck[17..29].try_into().expect("fixed slice"),
        r6: [deck[29], deck[30]],
        beta_reserve: [deck[31]],
    }
    .build()
    .expect("two copies of every animal")
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn dto_to_state(dto: &PositionDto) -> Result<State, JsValue> {
    let mut sets = [CardMultiset::EMPTY; 7];
    for location in LOCATIONS {
        for card in dto.locations.get(location) {
            let player: Player = card.owner.into();
            let value = if card.is_snipe {
                Card::Snipe
            } else {
                Card::Animal(animal_from_name(&card.animal)?)
            };
            let target = if matches!(location, Location::AlphaReserve | Location::BetaReserve) {
                0
            } else {
                location_index(location)
            };
            sets[target] = sets[target]
                .checked_add(CardMultiset::singleton(value, player))
                .ok_or_else(|| js_error("position contains an impossible card multiplicity"))?;
        }
    }
    let state = State {
        active_player: dto.turn.into(),
        reserves: sets[0],
        r1: sets[1],
        r2: sets[2],
        r3: sets[3],
        r4: sets[4],
        r5: sets[5],
        r6: sets[6],
        leading_action: None,
    };
    if state.winner().map(PlayerDto::from) != dto.winner {
        return Err(js_error("position winner does not match Core"));
    }
    Ok(state)
}

fn state_to_dto(
    state: &State,
    seed: u64,
    turn_number: u32,
    prior: Option<&PositionDto>,
) -> PositionDto {
    let mut locations = LocationsDto {
        alpha_reserve: Vec::new(),
        beta_reserve: Vec::new(),
        row_1: Vec::new(),
        row_2: Vec::new(),
        row_3: Vec::new(),
        row_4: Vec::new(),
        row_5: Vec::new(),
        row_6: Vec::new(),
    };
    for (animal_index, animal) in animals().into_iter().enumerate() {
        let mut ids = prior
            .map(|position| {
                LOCATIONS
                    .into_iter()
                    .flat_map(|location| position.locations.get(location))
                    .filter(|card| !card.is_snipe && card.animal == animal_name(animal))
                    .map(|card| card.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        ids.sort();
        while ids.len() < 2 {
            ids.push(format!("animal-{}", animal_index + ids.len() * 16));
        }
        let mut id_cursor = 0;
        for location in STATE_LOCATIONS {
            let cards = state_cards(state, location);
            for player in [Player::Alpha, Player::Beta] {
                for _ in 0..cards.count(Card::Animal(animal), player) {
                    locations
                        .get_mut(reserve_for(location, player))
                        .push(CardDto {
                            id: ids[id_cursor].clone(),
                            animal: animal_name(animal).to_owned(),
                            owner: player.into(),
                            is_snipe: false,
                            can_retreat: animal.is_retreater(),
                        });
                    id_cursor += 1;
                }
            }
        }
    }
    for player in [Player::Alpha, Player::Beta] {
        for location in STATE_LOCATIONS {
            if state_cards(state, location).count(Card::Snipe, player) != 0 {
                locations
                    .get_mut(reserve_for(location, player))
                    .push(CardDto {
                        id: format!("{}-snipe", player_slug(player)),
                        animal: "Snipe".to_owned(),
                        owner: player.into(),
                        is_snipe: true,
                        can_retreat: true,
                    });
            }
        }
    }
    PositionDto {
        schema_version: 1,
        seed,
        turn: state.active_player.into(),
        turn_number,
        winner: state.winner().map(Into::into),
        locations,
    }
}

fn reserve_for(location: Location, owner: Player) -> Location {
    if matches!(location, Location::AlphaReserve | Location::BetaReserve) {
        if owner == Player::Alpha {
            Location::AlphaReserve
        } else {
            Location::BetaReserve
        }
    } else {
        location
    }
}

fn state_cards(state: &State, location: Location) -> CardMultiset {
    match location {
        Location::AlphaReserve | Location::BetaReserve => state.reserves,
        Location::R1 => state.r1,
        Location::R2 => state.r2,
        Location::R3 => state.r3,
        Location::R4 => state.r4,
        Location::R5 => state.r5,
        Location::R6 => state.r6,
    }
}

fn full_turns(state: &State) -> Vec<Vec<Action>> {
    let player = state.active_player;
    let mut first = Vec::new();
    state.write_legal_actions(&mut first);
    let mut turns = Vec::new();
    for action in first {
        let Ok(after) = state.clone().apply(action) else {
            continue;
        };
        if after.active_player != player || after.winner().is_some() {
            turns.push(vec![action]);
        } else {
            let mut second = Vec::new();
            after.write_legal_actions(&mut second);
            for next in second {
                if after.clone().apply(next).is_ok() {
                    turns.push(vec![action, next]);
                }
            }
        }
    }
    turns
}

fn execute(mut state: State, actions: &[Action]) -> Result<State, JsValue> {
    for &action in actions {
        state = state
            .apply(action)
            .map_err(|error| js_error(format!("illegal action: {error:?}")))?;
    }
    Ok(state)
}

fn actions_to_dto(
    position: &PositionDto,
    state: &State,
    actions: &[Action],
) -> Result<TurnMoveDto, JsValue> {
    let player = state.active_player;
    let mut working_state = state.clone();
    let mut working_position = position.clone();
    let mut steps = Vec::new();
    for &action in actions {
        let (source, destination, animal, snipe) = action_parts(&working_state, action)?;
        let card = working_position
            .locations
            .get(source)
            .iter()
            .find(|card| {
                card.owner == PlayerDto::from(player)
                    && card.is_snipe == snipe
                    && (snipe || card.animal == animal_name(animal))
            })
            .ok_or_else(|| js_error("moving card was not found"))?;
        steps.push(MoveStepDto {
            card_id: card.id.clone(),
            from: location_name(source).to_owned(),
            to: location_name(destination).to_owned(),
        });
        working_state = working_state
            .apply(action)
            .map_err(|error| js_error(format!("generated illegal action: {error:?}")))?;
        working_position = state_to_dto(
            &working_state,
            position.seed,
            position.turn_number,
            Some(&working_position),
        );
    }
    let before_cards = all_cards(position);
    let after_cards = all_cards(&working_position);
    let captures = before_cards
        .iter()
        .filter_map(|before| {
            let after = after_cards.iter().find(|after| after.id == before.id)?;
            ((before.owner != after.owner)
                || (after.is_snipe && is_reserve_id(&card_location(&working_position, &after.id)?)))
            .then(|| before.id.clone())
        })
        .collect();
    let label = steps
        .iter()
        .map(|step| compact_label(step, player))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(TurnMoveDto {
        id: action_id(actions),
        player: player.into(),
        label,
        steps,
        captures,
    })
}

fn find_first_action(
    position: &PositionDto,
    state: &State,
    requested: &MoveStepDto,
) -> Result<Action, JsValue> {
    let mut actions = Vec::new();
    state.write_legal_actions(&mut actions);
    for action in actions {
        if !matches!(action, Action::AnimalStep(_)) {
            continue;
        }
        let dto = actions_to_dto(position, state, &[action])?;
        if dto.steps.first() == Some(requested) {
            return Ok(action);
        }
    }
    Err(js_error("first animal step is not legal"))
}

fn action_parts(
    state: &State,
    action: Action,
) -> Result<(Location, Location, Animal, bool), JsValue> {
    match action {
        Action::AnimalStep(step) => Ok((
            source_location(step.destination, state.active_player, step.direction)?,
            rank_location(step.destination),
            step.actor,
            false,
        )),
        Action::Drop(AnimalDrop { actor, destination }) => Ok((
            if state.active_player == Player::Alpha {
                Location::AlphaReserve
            } else {
                Location::BetaReserve
            },
            rank_location(destination),
            actor,
            false,
        )),
        Action::SnipeStep(SnipeStep { destination }) => {
            let source = snipe_location(state, state.active_player)
                .ok_or_else(|| js_error("snipe not found"))?;
            Ok((source, rank_location(destination), Animal::Mouse, true))
        }
    }
}

fn source_location(
    destination: Rank,
    player: Player,
    direction: StepDirection,
) -> Result<Location, JsValue> {
    let destination = rank_number(destination) as i8;
    let delta = match (player, direction) {
        (Player::Alpha, StepDirection::Advance) | (Player::Beta, StepDirection::Retreat) => -1,
        (Player::Alpha, StepDirection::Retreat) | (Player::Beta, StepDirection::Advance) => 1,
    };
    number_location(destination + delta).ok_or_else(|| js_error("action source is out of range"))
}

fn snipe_location(state: &State, player: Player) -> Option<Location> {
    [
        Location::R1,
        Location::R2,
        Location::R3,
        Location::R4,
        Location::R5,
        Location::R6,
    ]
    .into_iter()
    .find(|&location| state_cards(state, location).count(Card::Snipe, player) != 0)
}

fn action_id(actions: &[Action]) -> String {
    actions
        .iter()
        .map(|action| match action {
            Action::AnimalStep(step) => format!(
                "a{}{}{}",
                animal_index(step.actor),
                rank_number(step.destination),
                if step.direction == StepDirection::Advance {
                    "a"
                } else {
                    "r"
                }
            ),
            Action::Drop(drop) => format!(
                "d{}{}",
                animal_index(drop.actor),
                rank_number(drop.destination)
            ),
            Action::SnipeStep(step) => format!("s{}", rank_number(step.destination)),
        })
        .collect::<Vec<_>>()
        .join("-")
}

fn evaluation_dto(evaluation: Evaluation) -> EvaluationDto {
    match evaluation {
        Evaluation::MateInN { winner, plies } => EvaluationDto::Mate {
            winner: winner.into(),
            plies,
        },
        Evaluation::Estimate(value) => EvaluationDto::Estimate { value: value.raw() },
    }
}

fn all_cards(position: &PositionDto) -> Vec<&CardDto> {
    LOCATIONS
        .into_iter()
        .flat_map(|location| position.locations.get(location))
        .collect()
}

fn card_location(position: &PositionDto, id: &str) -> Option<String> {
    LOCATIONS
        .into_iter()
        .find(|&location| {
            position
                .locations
                .get(location)
                .iter()
                .any(|card| card.id == id)
        })
        .map(|location| location_name(location).to_owned())
}

fn is_reserve_id(location: &str) -> bool {
    location.ends_with("reserve")
}

fn compact_label(step: &MoveStepDto, player: Player) -> String {
    let name = if step.card_id.ends_with("snipe") {
        if player == Player::Alpha {
            "Alpha".to_owned()
        } else {
            "Beta".to_owned()
        }
    } else {
        step.card_id.clone()
    };
    format!("{name} {}", step.to.trim_start_matches("row-"))
}

fn animal_from_name(name: &str) -> Result<Animal, JsValue> {
    animals()
        .into_iter()
        .find(|animal| animal_name(*animal) == name)
        .ok_or_else(|| js_error(format!("unknown animal: {name}")))
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

fn animal_index(animal: Animal) -> usize {
    animals()
        .iter()
        .position(|&candidate| candidate == animal)
        .expect("known animal")
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

fn location_index(location: Location) -> usize {
    match location {
        Location::AlphaReserve | Location::BetaReserve => 0,
        Location::R1 => 1,
        Location::R2 => 2,
        Location::R3 => 3,
        Location::R4 => 4,
        Location::R5 => 5,
        Location::R6 => 6,
    }
}

fn rank_location(rank: Rank) -> Location {
    match rank {
        Rank::R1 => Location::R1,
        Rank::R2 => Location::R2,
        Rank::R3 => Location::R3,
        Rank::R4 => Location::R4,
        Rank::R5 => Location::R5,
        Rank::R6 => Location::R6,
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

fn number_location(number: i8) -> Option<Location> {
    match number {
        1 => Some(Location::R1),
        2 => Some(Location::R2),
        3 => Some(Location::R3),
        4 => Some(Location::R4),
        5 => Some(Location::R5),
        6 => Some(Location::R6),
        _ => None,
    }
}

fn location_name(location: Location) -> &'static str {
    match location {
        Location::AlphaReserve => "alpha-reserve",
        Location::BetaReserve => "beta-reserve",
        Location::R1 => "row-1",
        Location::R2 => "row-2",
        Location::R3 => "row-3",
        Location::R4 => "row-4",
        Location::R5 => "row-5",
        Location::R6 => "row-6",
    }
}

fn player_slug(player: Player) -> &'static str {
    if player == Player::Alpha {
        "alpha"
    } else {
        "beta"
    }
}

fn encode<T: Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|error| js_error(error.to_string()))
}

fn decode<T: for<'de> Deserialize<'de>>(json: &str) -> Result<T, JsValue> {
    serde_json::from_str(json).map_err(|error| js_error(error.to_string()))
}

fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_deals_round_trip_and_generate_applicable_moves() {
        for seed in 0..8 {
            let state = initial_state(seed);
            let dto = state_to_dto(&state, seed, 1, None);
            let rebuilt = dto_to_state(&dto).unwrap();
            assert_eq!(format!("{rebuilt:?}"), format!("{state:?}"));
            for actions in full_turns(&rebuilt) {
                assert!(actions_to_dto(&dto, &rebuilt, &actions).is_ok());
                assert!(execute(rebuilt.clone(), &actions).is_ok());
            }
        }
    }

    #[test]
    fn strategies_have_distinct_move_signatures() {
        let mut different = 0;
        for seed in 0..8 {
            let state = initial_state(seed);
            let mut avocado = AvocadoAnalyzer::new();
            avocado.set_state(state.clone());
            avocado.think(2);
            let mut avocado_line = Vec::new();
            avocado.write_optimal_lop(&mut avocado_line);

            let mut blueberry = BlueberryAnalyzer::new();
            blueberry.set_state(state);
            blueberry.think(2);
            let mut blueberry_line = Vec::new();
            blueberry.write_optimal_lop(&mut blueberry_line);
            different += usize::from(avocado_line != blueberry_line);
        }
        assert!(
            different >= 4,
            "strategies differed on only {different}/8 seeded positions"
        );
    }
}
