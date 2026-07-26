//! Browser contract for Snipe Hunt's value-semantic Core.
//!
//! This bridge deliberately exposes no persistent physical-card identity.
//! A piece is selected by kind, allegiance, and source location. Identical
//! pieces in one location are interchangeable, exactly as they are in Core.

use agent_avocado::AvocadoAnalyzer;
use agent_blueberry::BlueberryAnalyzer;
use serde::{Deserialize, Serialize};
use snipe_core::{
    Action, Analyzer, Animal, AnimalDrop, AnimalStep, Card, CardMultiset, Evaluation,
    InitialStateBuilder, Player, Rank, SnipeStep, State, StepDirection,
};
use std::fmt::Write as _;
use wasm_bindgen::prelude::*;

const POSITION_SCHEMA: u8 = 1;
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardDto {
    piece_key: String,
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
    fn empty() -> Self {
        Self {
            alpha_reserve: Vec::new(),
            beta_reserve: Vec::new(),
            row_1: Vec::new(),
            row_2: Vec::new(),
            row_3: Vec::new(),
            row_4: Vec::new(),
            row_5: Vec::new(),
            row_6: Vec::new(),
        }
    }

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
struct LeadingActionDto {
    animal: String,
    direction: DirectionDto,
    destination: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum DirectionDto {
    Advance,
    Retreat,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionDto {
    schema_version: u8,
    position_key: String,
    seed: u64,
    turn: PlayerDto,
    turn_number: u32,
    winner: Option<PlayerDto>,
    leading_action: Option<LeadingActionDto>,
    locations: LocationsDto,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureDto {
    animals: Vec<String>,
    snipe: Option<PlayerDto>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveStepDto {
    piece_key: String,
    animal: String,
    owner: PlayerDto,
    is_snipe: bool,
    from: String,
    to: String,
    capture: CaptureDto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct TurnMoveDto {
    id: String,
    position_key: String,
    player: PlayerDto,
    label: String,
    steps: Vec<MoveStepDto>,
    captures: CaptureDto,
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
    position_key: String,
    best_move: TurnMoveDto,
    evaluation: EvaluationDto,
    ticks: u64,
    elapsed_ms: u64,
    recommended_line: Vec<TurnMoveDto>,
    strategy: Strategy,
    engine_name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    encode(&state_to_dto(&state, u64::from(seed), 1))
}

#[wasm_bindgen]
pub fn canonicalize_position(position_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let state = dto_to_state_inner(&position, false)?;
    encode(&state_to_dto(&state, position.seed, position.turn_number))
}

#[wasm_bindgen]
pub fn legal_moves(position_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let state = dto_to_state(&position)?;
    let moves = full_turns(&state)
        .into_iter()
        .map(|actions| actions_to_dto(&state, &actions))
        .collect::<Result<Vec<_>, _>>()?;
    encode(&moves)
}

#[wasm_bindgen]
pub fn preview_first_step(position_json: &str, step_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let requested: MoveStepDto = decode(step_json)?;
    let state = dto_to_state(&position)?;
    let action = find_first_action(&state, &requested)?;
    let next = state
        .apply(action)
        .map_err(|error| js_error(format!("illegal first step: {error:?}")))?;
    encode(&state_to_dto(&next, position.seed, position.turn_number))
}

#[wasm_bindgen]
pub fn apply_move(position_json: &str, move_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let requested: TurnMoveDto = decode(move_json)?;
    let state = dto_to_state(&position)?;
    if requested.position_key != position.position_key {
        return Err(js_error("move belongs to a different position"));
    }
    let actions = matching_requested_turn(&state, &requested)
        .ok_or_else(|| js_error("move does not exactly match a legal turn"))?;
    let next = execute(state, &actions)?;
    encode(&state_to_dto(
        &next,
        position.seed,
        position.turn_number.saturating_add(1),
    ))
}

fn matching_requested_turn(state: &State, requested: &TurnMoveDto) -> Option<Vec<Action>> {
    for actions in full_turns(state) {
        if actions_to_dto(state, &actions).ok()? == *requested {
            return Some(actions);
        }
    }
    None
}

#[wasm_bindgen]
pub fn analyze(request_json: &str) -> Result<String, JsValue> {
    let request: AnalysisRequestDto = decode(request_json)?;
    encode(&run_analysis(&request, None)?)
}

#[wasm_bindgen]
pub fn analyze_live(request_json: &str, on_progress: &js_sys::Function) -> Result<String, JsValue> {
    let request: AnalysisRequestDto = decode(request_json)?;
    encode(&run_analysis(&request, Some(on_progress))?)
}

fn run_analysis(
    request: &AnalysisRequestDto,
    callback: Option<&js_sys::Function>,
) -> Result<AnalysisUpdateDto, JsValue> {
    let base = dto_to_state(&request.position)?;
    let first_action = request
        .first_step
        .as_ref()
        .map(|step| find_first_action(&base, step))
        .transpose()?;
    let analyzed = if let Some(action) = first_action {
        base.clone()
            .apply(action)
            .map_err(|error| js_error(format!("illegal first step: {error:?}")))?
    } else {
        base.clone()
    };
    let mut analyzer = BrowserAnalyzer::new(request.strategy, analyzed);
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
            let update =
                analysis_update(request, &base, first_action, &analyzer, ticks, now - start)?;
            let json =
                serde_json::to_string(&update).map_err(|error| js_error(error.to_string()))?;
            callback.call1(&JsValue::NULL, &JsValue::from_str(&json))?;
            last_progress = now;
        }
    }
    analysis_update(
        request,
        &base,
        first_action,
        &analyzer,
        ticks,
        js_sys::Date::now() - start,
    )
}

fn analysis_update(
    request: &AnalysisRequestDto,
    base: &State,
    first_action: Option<Action>,
    analyzer: &BrowserAnalyzer,
    ticks: u64,
    elapsed: f64,
) -> Result<AnalysisUpdateDto, JsValue> {
    let mut actions = Vec::new();
    if let Some(action) = first_action {
        actions.push(action);
    }
    actions.extend(analyzer.line());
    let recommended_line = complete_analysis_turns(base, first_action, &actions)?;
    let best_move = recommended_line
        .first()
        .cloned()
        .ok_or_else(|| js_error("no legal moves are available"))?;
    Ok(AnalysisUpdateDto {
        request_id: request.request_id,
        position_key: request.position.position_key.clone(),
        best_move: best_move.clone(),
        evaluation: evaluation_dto(analyzer.evaluation()),
        ticks,
        elapsed_ms: elapsed.max(0.0) as u64,
        recommended_line,
        strategy: request.strategy,
        engine_name: request.strategy.label(),
    })
}

/// Converts the analyzer's action-level line of play into browser-visible,
/// fully replayed turns. A trailing half-turn is never exposed.
fn complete_analysis_turns(
    base: &State,
    required_first: Option<Action>,
    actions: &[Action],
) -> Result<Vec<TurnMoveDto>, JsValue> {
    let mut turns = Vec::new();
    let mut turn_base = base.clone();
    let mut working = base.clone();
    let mut current = Vec::new();

    for &action in actions {
        let player = turn_base.active_player;
        working = working
            .apply(action)
            .map_err(|error| js_error(format!("analyzer returned an illegal action: {error:?}")))?;
        current.push(action);
        if working.active_player != player || working.winner().is_some() {
            turns.push(actions_to_dto(&turn_base, &current)?);
            turn_base = working.clone();
            current.clear();
        }
    }

    if turns.is_empty() {
        // A time-limited analyzer may yield no line, or a line ending after
        // only the leading half of a turn. Complete that turn from Core's
        // advertised legal turns instead of inventing an invalid browser move.
        let prefix = if current.is_empty() {
            required_first.into_iter().collect::<Vec<_>>()
        } else {
            current
        };
        let completion = full_turns(base)
            .into_iter()
            .find(|candidate| candidate.starts_with(&prefix))
            .ok_or_else(|| js_error("analyzer did not provide a complete legal turn"))?;
        turns.push(actions_to_dto(base, &completion)?);
    }

    Ok(turns)
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
    dto_to_state_inner(dto, true)
}

fn dto_to_state_inner(dto: &PositionDto, validate_key: bool) -> Result<State, JsValue> {
    if dto.schema_version != POSITION_SCHEMA {
        return Err(js_error("unsupported position schema"));
    }
    let mut sets = [CardMultiset::EMPTY; 7];
    for location in LOCATIONS {
        for card in dto.locations.get(location) {
            validate_card(location, card)?;
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
    let leading_action = dto
        .leading_action
        .as_ref()
        .map(leading_action_from_dto)
        .transpose()?;
    let state = State {
        active_player: dto.turn.into(),
        reserves: sets[0],
        r1: sets[1],
        r2: sets[2],
        r3: sets[3],
        r4: sets[4],
        r5: sets[5],
        r6: sets[6],
        leading_action,
    };
    if state.winner().map(PlayerDto::from) != dto.winner {
        return Err(js_error("position winner does not match Core"));
    }
    if validate_key && position_key(&state) != dto.position_key {
        return Err(js_error("position key does not match its contents"));
    }
    Ok(state)
}

fn validate_card(location: Location, card: &CardDto) -> Result<(), JsValue> {
    let player: Player = card.owner.into();
    let expected_key = if card.is_snipe {
        piece_key(Card::Snipe, player)
    } else {
        let animal = animal_from_name(&card.animal)?;
        if card.can_retreat != animal.is_retreater() {
            return Err(js_error("card retreat property does not match Core"));
        }
        piece_key(Card::Animal(animal), player)
    };
    if card.piece_key != expected_key {
        return Err(js_error("piece key does not match card value"));
    }
    match location {
        Location::AlphaReserve if !card.is_snipe && player != Player::Alpha => {
            Err(js_error("animal is in the wrong reserve"))
        }
        Location::BetaReserve if !card.is_snipe && player != Player::Beta => {
            Err(js_error("animal is in the wrong reserve"))
        }
        Location::AlphaReserve if card.is_snipe && player != Player::Beta => {
            Err(js_error("snipe is in the wrong capture reserve"))
        }
        Location::BetaReserve if card.is_snipe && player != Player::Alpha => {
            Err(js_error("snipe is in the wrong capture reserve"))
        }
        _ => Ok(()),
    }
}

fn leading_action_from_dto(dto: &LeadingActionDto) -> Result<AnimalStep, JsValue> {
    Ok(AnimalStep {
        actor: animal_from_name(&dto.animal)?,
        direction: match dto.direction {
            DirectionDto::Advance => StepDirection::Advance,
            DirectionDto::Retreat => StepDirection::Retreat,
        },
        destination: number_rank(dto.destination)
            .ok_or_else(|| js_error("leading action destination is out of range"))?,
    })
}

fn state_to_dto(state: &State, seed: u64, turn_number: u32) -> PositionDto {
    let mut locations = LocationsDto::empty();
    for animal in animals() {
        for location in STATE_LOCATIONS {
            let cards = state_cards(state, location);
            for player in [Player::Alpha, Player::Beta] {
                for _ in 0..cards.count(Card::Animal(animal), player) {
                    locations
                        .get_mut(animal_output_location(location, player))
                        .push(card_dto(Card::Animal(animal), player));
                }
            }
        }
    }
    for player in [Player::Alpha, Player::Beta] {
        for location in STATE_LOCATIONS {
            if state_cards(state, location).count(Card::Snipe, player) != 0 {
                locations
                    .get_mut(snipe_output_location(location, player))
                    .push(card_dto(Card::Snipe, player));
            }
        }
    }
    PositionDto {
        schema_version: POSITION_SCHEMA,
        position_key: position_key(state),
        seed,
        turn: state.active_player.into(),
        turn_number,
        winner: state.winner().map(Into::into),
        leading_action: state.leading_action.map(|step| LeadingActionDto {
            animal: animal_name(step.actor).to_owned(),
            direction: if step.direction == StepDirection::Advance {
                DirectionDto::Advance
            } else {
                DirectionDto::Retreat
            },
            destination: rank_number(step.destination),
        }),
        locations,
    }
}

fn card_dto(card: Card, player: Player) -> CardDto {
    match card {
        Card::Snipe => CardDto {
            piece_key: piece_key(card, player),
            animal: "Snipe".to_owned(),
            owner: player.into(),
            is_snipe: true,
            can_retreat: true,
        },
        Card::Animal(animal) => CardDto {
            piece_key: piece_key(card, player),
            animal: animal_name(animal).to_owned(),
            owner: player.into(),
            is_snipe: false,
            can_retreat: animal.is_retreater(),
        },
    }
}

fn piece_key(card: Card, player: Player) -> String {
    let owner = if player == Player::Alpha {
        "alpha"
    } else {
        "beta"
    };
    match card {
        Card::Snipe => format!("{owner}:snipe"),
        Card::Animal(animal) => format!("{owner}:animal:{}", animal_index(animal)),
    }
}

fn animal_output_location(location: Location, owner: Player) -> Location {
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

fn snipe_output_location(location: Location, owner: Player) -> Location {
    if matches!(location, Location::AlphaReserve | Location::BetaReserve) {
        if owner == Player::Alpha {
            Location::BetaReserve
        } else {
            Location::AlphaReserve
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

fn actions_to_dto(state: &State, actions: &[Action]) -> Result<TurnMoveDto, JsValue> {
    let player = state.active_player;
    let root_key = position_key(state);
    let mut working = state.clone();
    let mut steps = Vec::new();
    let mut captures = CaptureDto::default();
    for &action in actions {
        let (source, destination, card) = action_parts(&working, action)?;
        let next = working
            .clone()
            .apply(action)
            .map_err(|error| js_error(format!("generated illegal action: {error:?}")))?;
        let capture = capture_outcome(&working, &next, action);
        captures.animals.extend(capture.animals.iter().cloned());
        captures.snipe = captures.snipe.or(capture.snipe);
        let (animal, is_snipe) = match card {
            Card::Snipe => ("Snipe".to_owned(), true),
            Card::Animal(animal) => (animal_name(animal).to_owned(), false),
        };
        steps.push(MoveStepDto {
            piece_key: piece_key(card, player),
            animal,
            owner: player.into(),
            is_snipe,
            from: location_name(source).to_owned(),
            to: location_name(destination).to_owned(),
            capture,
        });
        working = next;
    }
    let label = steps
        .iter()
        .map(|step| compact_label(step, player))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(TurnMoveDto {
        id: action_id(actions),
        position_key: root_key,
        player: player.into(),
        label,
        steps,
        captures,
    })
}

fn capture_outcome(before: &State, after: &State, action: Action) -> CaptureDto {
    let player = before.active_player;
    let dropped = match action {
        Action::Drop(drop) => Some(drop.actor),
        _ => None,
    };
    let mut captured_animals = Vec::new();
    for animal in animals() {
        let before_count = before.reserves.count(Card::Animal(animal), player) as i16;
        let after_count = after.reserves.count(Card::Animal(animal), player) as i16;
        let drop_adjustment = i16::from(dropped == Some(animal));
        let captured = after_count - before_count + drop_adjustment;
        for _ in 0..captured.max(0) {
            captured_animals.push(animal_name(animal).to_owned());
        }
    }
    let snipe = [Player::Alpha, Player::Beta]
        .into_iter()
        .find(|&owner| {
            before.reserves.count(Card::Snipe, owner) == 0
                && after.reserves.count(Card::Snipe, owner) != 0
        })
        .map(Into::into);
    CaptureDto {
        animals: captured_animals,
        snipe,
    }
}

fn find_first_action(state: &State, requested: &MoveStepDto) -> Result<Action, JsValue> {
    let mut actions = Vec::new();
    state.write_legal_actions(&mut actions);
    actions
        .into_iter()
        .filter(|action| matches!(action, Action::AnimalStep(_)))
        .find(|&action| {
            action_selector(state, action).is_ok_and(|candidate| candidate == *requested)
        })
        .ok_or_else(|| js_error("first animal step is not legal"))
}

fn action_selector(state: &State, action: Action) -> Result<MoveStepDto, JsValue> {
    let player = state.active_player;
    let (source, destination, card) = action_parts(state, action)?;
    let after = state
        .clone()
        .apply(action)
        .map_err(|error| js_error(format!("illegal first action: {error:?}")))?;
    let (animal, is_snipe) = match card {
        Card::Snipe => ("Snipe".to_owned(), true),
        Card::Animal(animal) => (animal_name(animal).to_owned(), false),
    };
    Ok(MoveStepDto {
        piece_key: piece_key(card, player),
        animal,
        owner: player.into(),
        is_snipe,
        from: location_name(source).to_owned(),
        to: location_name(destination).to_owned(),
        capture: capture_outcome(state, &after, action),
    })
}

fn action_parts(state: &State, action: Action) -> Result<(Location, Location, Card), JsValue> {
    match action {
        Action::AnimalStep(step) => Ok((
            source_location(step.destination, state.active_player, step.direction)?,
            rank_location(step.destination),
            Card::Animal(step.actor),
        )),
        Action::Drop(AnimalDrop { actor, destination }) => Ok((
            if state.active_player == Player::Alpha {
                Location::AlphaReserve
            } else {
                Location::BetaReserve
            },
            rank_location(destination),
            Card::Animal(actor),
        )),
        Action::SnipeStep(SnipeStep { destination }) => Ok((
            snipe_location(state, state.active_player)
                .ok_or_else(|| js_error("snipe not found"))?,
            rank_location(destination),
            Card::Snipe,
        )),
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

fn compact_label(step: &MoveStepDto, player: Player) -> String {
    let destination = step.to.trim_start_matches("row-");
    if step.from.ends_with("reserve") {
        return format!("{} &{destination}", step.animal);
    }
    let source = step
        .from
        .strip_prefix("row-")
        .and_then(|rank| rank.parse::<u8>().ok())
        .unwrap_or_default();
    let target = destination.parse::<u8>().unwrap_or_default();
    let advances = if player == Player::Alpha {
        target > source
    } else {
        target < source
    };
    let name = if step.is_snipe {
        if player == Player::Alpha {
            "Alpha"
        } else {
            "Beta"
        }
    } else {
        &step.animal
    };
    format!("{name} {}{destination}", if advances { "" } else { "*" })
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

fn position_key(state: &State) -> String {
    // This is deliberately a canonical encoding, not a hash. Position
    // identity participates in stale-action rejection, so even a theoretical
    // hash collision would be a correctness bug.
    let mut key = if state.active_player == Player::Alpha {
        "p1:A".to_owned()
    } else {
        "p1:B".to_owned()
    };
    for cards in [
        state.reserves,
        state.r1,
        state.r2,
        state.r3,
        state.r4,
        state.r5,
        state.r6,
    ]
    .into_iter()
    {
        key.push('|');
        for card in animals().into_iter().map(Card::Animal).chain([Card::Snipe]) {
            for player in [Player::Alpha, Player::Beta] {
                write!(&mut key, "{},", cards.count(card, player))
                    .expect("writing to a String cannot fail");
            }
        }
    }
    if let Some(step) = state.leading_action {
        write!(
            &mut key,
            "|L{},{},{}",
            animal_index(step.actor),
            rank_number(step.destination),
            if step.direction == StepDirection::Advance {
                "a"
            } else {
                "r"
            }
        )
        .expect("writing to a String cannot fail");
    } else {
        key.push_str("|L-");
    }
    key
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

fn number_rank(number: u8) -> Option<Rank> {
    match number {
        1 => Some(Rank::R1),
        2 => Some(Rank::R2),
        3 => Some(Rank::R3),
        4 => Some(Rank::R4),
        5 => Some(Rank::R5),
        6 => Some(Rank::R6),
        _ => None,
    }
}

fn number_location(number: i8) -> Option<Location> {
    number_rank(number.try_into().ok()?).map(rank_location)
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
    fn positions_round_trip_without_physical_card_identity() {
        for seed in 0..32 {
            let state = initial_state(seed);
            let dto = state_to_dto(&state, seed, 1);
            let rebuilt = dto_to_state(&dto).unwrap();
            assert_eq!(format!("{rebuilt:?}"), format!("{state:?}"));
            assert!(
                LOCATIONS
                    .into_iter()
                    .flat_map(|l| dto.locations.get(l))
                    .all(|card| !card.piece_key.contains("occurrence")
                        && !card.piece_key.contains('@'))
            );
        }
    }

    #[test]
    fn every_advertised_turn_is_applicable_and_scoped_to_its_position() {
        for seed in 0..16 {
            let state = initial_state(seed);
            for actions in full_turns(&state) {
                let dto = actions_to_dto(&state, &actions).unwrap();
                assert_eq!(dto.position_key, position_key(&state));
                assert_eq!(dto.steps.len(), actions.len());
                assert!(execute(state.clone(), &actions).is_ok());
            }
        }
    }

    #[test]
    fn value_contract_survives_randomized_multi_ply_play() {
        for seed in 0..32_u64 {
            let mut state = initial_state(seed);
            for ply in 0..40_usize {
                let dto = state_to_dto(&state, seed, (ply + 1) as u32);
                let rebuilt = dto_to_state(&dto).unwrap();
                assert_eq!(position_key(&rebuilt), dto.position_key);

                let turns = full_turns(&state);
                if turns.is_empty() {
                    assert!(state.winner().is_some());
                    break;
                }
                for actions in &turns {
                    let advertised = actions_to_dto(&state, actions).unwrap();
                    assert_eq!(advertised.position_key, dto.position_key);
                    assert!(execute(state.clone(), actions).is_ok());
                }
                let choice = ((seed as usize).wrapping_mul(31) + ply * 17) % turns.len();
                state = execute(state, &turns[choice]).unwrap();
            }
        }
    }

    #[test]
    fn midpoint_analysis_returns_a_complete_turn_for_the_base_position() {
        let state = initial_state(7_071);
        let actions = full_turns(&state)
            .into_iter()
            .find(|actions| actions.len() == 2)
            .expect("initial state has a two-action turn");
        let first = actions[0];
        let after_first = state.clone().apply(first).unwrap();
        let request = AnalysisRequestDto {
            position: state_to_dto(&state, 7_071, 1),
            time_limit_ms: 1,
            request_id: 1,
            strategy: Strategy::Blueberry,
            first_step: Some(action_selector(&state, first).unwrap()),
        };
        let mut analyzer = BrowserAnalyzer::new(Strategy::Blueberry, after_first);
        analyzer.think(1);

        let update = analysis_update(&request, &state, Some(first), &analyzer, 1, 0.0).unwrap();

        assert_eq!(update.best_move.position_key, position_key(&state));
        assert_eq!(update.best_move.steps.len(), 2);
        assert!(
            full_turns(&state)
                .iter()
                .any(|turn| action_id(turn) == update.best_move.id)
        );
        assert_eq!(update.recommended_line[0].id, update.best_move.id);
    }

    #[test]
    fn analyzer_lines_are_split_into_complete_scoped_turns() {
        let state = initial_state(9);
        let (first, after_first, second) = full_turns(&state)
            .into_iter()
            .find_map(|first| {
                let after = execute(state.clone(), &first).ok()?;
                let second = full_turns(&after).into_iter().next()?;
                Some((first, after, second))
            })
            .expect("initial position has a two-turn continuation");
        let raw = first
            .iter()
            .chain(second.iter())
            .copied()
            .collect::<Vec<_>>();

        let line = complete_analysis_turns(&state, None, &raw).unwrap();

        assert_eq!(line.len(), 2);
        assert_eq!(line[0].position_key, position_key(&state));
        assert_eq!(line[0].id, action_id(&first));
        assert_eq!(line[1].position_key, position_key(&after_first));
        assert_eq!(line[1].id, action_id(&second));
    }

    #[test]
    fn duplicate_pieces_share_a_semantic_key() {
        let state = initial_state(4);
        let dto = state_to_dto(&state, 4, 1);
        for location in LOCATIONS {
            for left in dto.locations.get(location) {
                for right in dto.locations.get(location) {
                    if left.animal == right.animal && left.owner == right.owner {
                        assert_eq!(left.piece_key, right.piece_key);
                    }
                }
            }
        }
    }

    #[test]
    fn a_move_with_forged_derived_fields_is_rejected() {
        let state = initial_state(2);
        let actions = full_turns(&state).into_iter().next().unwrap();
        let mut advertised = actions_to_dto(&state, &actions).unwrap();
        advertised.label.push_str(" (forged)");

        assert!(matching_requested_turn(&state, &advertised).is_none());
    }
}
