//! String-based WASM bridge for the browser UI.
//!
//! JSON at this boundary is intentional: browser history remains ordinary
//! serializable data, while every rules/search operation is authoritative Rust.

use serde::{Deserialize, Serialize};
use snipe_ai::{evaluate_state, SearchConfig, SearchEngine, MATE_SCORE};
use snipe_core::{Animal, AtomicMove, Card as CoreCard, Location as CoreLocation};
use snipe_core::{Move, Player, State};
use std::time::Duration;
use wasm_bindgen::prelude::*;

const ENGINE_NAME: &str = "Snipe Hunt Rust alpha-beta";
const MIN_ANALYSIS_TIME_MS: u64 = 1;
const MAX_ANALYSIS_TIME_MS: u64 = 60_000;
const BROWSER_MAX_DEPTH: u8 = 64;
const MAX_LIVE_ANALYSIS_DEPTH: u8 = 10;
const BROWSER_DEADLINE_CHECK_INTERVAL: u64 = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardDto {
    id: String,
    animal: String,
    owner: PlayerDto,
    is_snipe: bool,
    can_retreat: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    fn get(&self, location: CoreLocation) -> &[CardDto] {
        match location {
            CoreLocation::AlphaReserve => &self.alpha_reserve,
            CoreLocation::Row1 => &self.row_1,
            CoreLocation::Row2 => &self.row_2,
            CoreLocation::Row3 => &self.row_3,
            CoreLocation::Row4 => &self.row_4,
            CoreLocation::Row5 => &self.row_5,
            CoreLocation::Row6 => &self.row_6,
            CoreLocation::BetaReserve => &self.beta_reserve,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PositionDto {
    schema_version: u8,
    seed: u64,
    turn: PlayerDto,
    turn_number: u32,
    winner: Option<PlayerDto>,
    locations: LocationsDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct MoveStepDto {
    card_id: String,
    from: String,
    to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TurnMoveDto {
    id: String,
    player: PlayerDto,
    label: String,
    steps: Vec<MoveStepDto>,
    captures: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisRequestDto {
    position: PositionDto,
    time_limit_ms: u64,
    request_id: u64,
    #[serde(default)]
    history: Vec<PositionDto>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveAnalysisRequestDto {
    position: PositionDto,
    max_depth: u8,
    request_id: u64,
    #[serde(default)]
    history: Vec<PositionDto>,
    #[serde(default)]
    first_step: Option<MoveStepDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateLineDto {
    r#move: TurnMoveDto,
    score: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisResultDto {
    request_id: u64,
    best_move: TurnMoveDto,
    score: i32,
    depth: u8,
    nodes: u64,
    elapsed_ms: u64,
    principal_variation: Vec<String>,
    candidates: Vec<CandidateLineDto>,
    engine_name: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveAnalysisUpdateDto {
    request_id: u64,
    best_move: TurnMoveDto,
    score: i32,
    depth: u8,
    principal_variation: Vec<TurnMoveDto>,
}

#[wasm_bindgen]
pub fn create_game(seed: u32) -> Result<String, JsValue> {
    encode(&position_to_dto(
        State::initial(seed as u64),
        seed as u64,
        1,
    ))
}

#[wasm_bindgen]
pub fn legal_moves(position_json: &str) -> Result<String, JsValue> {
    let dto: PositionDto = decode(position_json)?;
    let state = dto_to_state(&dto)?;
    let moves = state
        .legal_moves()
        .into_iter()
        .map(|mv| move_to_dto(state, mv))
        .collect::<Result<Vec<_>, _>>()?;
    encode(&moves)
}

#[wasm_bindgen]
pub fn preview_first_step(position_json: &str, step_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let requested: MoveStepDto = decode(step_json)?;
    let state = dto_to_state(&position)?;
    let first = find_first_animal_step(state, &requested)?;
    let preview = state
        .apply_atomic(AtomicMove::Animal(first))
        .map_err(|error| js_error(format!("illegal first animal step: {error}")))?;
    encode(&position_to_dto(
        preview,
        position.seed,
        position.turn_number,
    ))
}

#[wasm_bindgen]
pub fn apply_move(position_json: &str, move_json: &str) -> Result<String, JsValue> {
    let position: PositionDto = decode(position_json)?;
    let requested: TurnMoveDto = decode(move_json)?;
    let state = dto_to_state(&position)?;
    let mv = find_move(state, &requested.id)?;
    let next = state
        .apply_move(mv)
        .map_err(|error| js_error(format!("illegal move: {error}")))?;
    encode(&position_to_dto(
        next,
        position.seed,
        position.turn_number.saturating_add(1),
    ))
}

#[wasm_bindgen]
pub fn analyze(request_json: &str) -> Result<String, JsValue> {
    let request: AnalysisRequestDto = decode(request_json)?;
    let state = dto_to_state(&request.position)?;
    let (repetition_hashes, convergence_hashes) = history_context(&request.history)?;
    let config = browser_search_config(request.time_limit_ms);
    let mut engine = SearchEngine::<State>::new(config);
    let result = engine.search_with_context(&state, &repetition_hashes, &convergence_hashes);
    let best = result
        .best_move
        .ok_or_else(|| js_error("no legal moves are available"))?;

    let mut candidates = state
        .legal_moves()
        .into_iter()
        .map(|mv| {
            let child = state
                .apply_move(mv)
                .map_err(|error| js_error(format!("generated illegal move: {error}")))?;
            let score = if child.winner() == Some(state.side_to_move()) {
                MATE_SCORE
            } else {
                -evaluate_state(child)
            };
            Ok((mv, score))
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    candidates.sort_unstable_by(|(left_move, left_score), (right_move, right_score)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_move.cmp(right_move))
    });
    if let Some(index) = candidates.iter().position(|(mv, _)| *mv == best) {
        let selected = candidates.remove(index);
        candidates.insert(0, selected);
    }

    let candidate_dtos = candidates
        .into_iter()
        .take(4)
        .map(|(mv, score)| {
            Ok(CandidateLineDto {
                r#move: move_to_dto(state, mv)?,
                score: if mv == best { result.score } else { score },
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    let mut pv_state = state;
    let mut pv = Vec::new();
    for mv in result.principal_variation {
        let dto = move_to_dto(pv_state, mv)?;
        pv.push(dto.label);
        pv_state = pv_state
            .apply_move(mv)
            .map_err(|error| js_error(format!("invalid principal variation: {error}")))?;
    }
    let response = AnalysisResultDto {
        request_id: request.request_id,
        best_move: move_to_dto(state, best)?,
        score: result.score,
        depth: result.depth,
        nodes: result.stats.nodes.saturating_add(result.stats.qnodes),
        elapsed_ms: result.stats.elapsed.as_millis().min(u64::MAX as u128) as u64,
        principal_variation: pv,
        candidates: candidate_dtos,
        engine_name: ENGINE_NAME,
    };
    encode(&response)
}

#[wasm_bindgen]
pub fn analyze_live(request_json: &str, on_progress: &js_sys::Function) -> Result<String, JsValue> {
    let request: LiveAnalysisRequestDto = decode(request_json)?;
    let state = dto_to_state(&request.position)?;
    let (repetition_hashes, convergence_hashes) = history_context(&request.history)?;
    let constrained_moves = request
        .first_step
        .as_ref()
        .map(|step| constrained_root_moves(state, step))
        .transpose()?;
    let config = SearchConfig {
        max_depth: request.max_depth.clamp(1, MAX_LIVE_ANALYSIS_DEPTH),
        deadline_check_interval: BROWSER_DEADLINE_CHECK_INTERVAL,
        ..SearchConfig::default()
    };
    let mut engine = SearchEngine::<State>::new(config);
    let result = engine.search_to_depth_with_context_and_progress(
        &state,
        &repetition_hashes,
        &convergence_hashes,
        constrained_moves.as_deref(),
        request.max_depth.clamp(1, MAX_LIVE_ANALYSIS_DEPTH),
        |progress| {
            let Some(best) = progress.best_move else {
                return;
            };
            let Ok(best_move) = move_to_dto(state, best) else {
                return;
            };
            let Ok(principal_variation) =
                principal_variation_to_dtos(state, &progress.principal_variation)
            else {
                return;
            };
            let update = LiveAnalysisUpdateDto {
                request_id: request.request_id,
                best_move,
                score: progress.score,
                depth: progress.depth,
                principal_variation,
            };
            if let Ok(json) = serde_json::to_string(&update) {
                let _ = on_progress.call1(&JsValue::NULL, &JsValue::from_str(&json));
            }
        },
    );
    let best = result
        .best_move
        .ok_or_else(|| js_error("no legal moves are available"))?;
    let principal_variation = principal_variation_to_dtos(state, &result.principal_variation)?;
    encode(&LiveAnalysisUpdateDto {
        request_id: request.request_id,
        best_move: move_to_dto(state, best)?,
        score: result.score,
        depth: result.depth,
        principal_variation,
    })
}

fn principal_variation_to_dtos(
    mut state: State,
    moves: &[Move],
) -> Result<Vec<TurnMoveDto>, JsValue> {
    let mut principal_variation = Vec::with_capacity(moves.len());
    for &mv in moves {
        principal_variation.push(move_to_dto(state, mv)?);
        state = state
            .apply_move(mv)
            .map_err(|error| js_error(format!("invalid principal variation: {error}")))?;
    }
    Ok(principal_variation)
}

fn constrained_root_moves(state: State, requested: &MoveStepDto) -> Result<Vec<Move>, JsValue> {
    let first = find_first_animal_step(state, requested)?;
    let moves = state
        .legal_moves()
        .into_iter()
        .filter(|mv| matches!(mv, Move::Animals { first: candidate, .. } if *candidate == first))
        .collect::<Vec<_>>();
    if moves.is_empty() {
        Err(js_error("no legal continuation matches the first subply"))
    } else {
        Ok(moves)
    }
}

fn history_context(history: &[PositionDto]) -> Result<(Vec<u64>, Vec<u64>), JsValue> {
    let states = history
        .iter()
        .map(dto_to_state)
        .collect::<Result<Vec<_>, _>>()?;
    let repetition_hashes = states.iter().map(|state| state.repetition_hash()).collect();
    let convergence_hashes = states
        .iter()
        .map(|state| state.convergence_hash())
        .collect();
    Ok((repetition_hashes, convergence_hashes))
}

fn browser_search_config(requested_time_ms: u64) -> SearchConfig {
    SearchConfig {
        time_limit: Duration::from_millis(
            requested_time_ms.clamp(MIN_ANALYSIS_TIME_MS, MAX_ANALYSIS_TIME_MS),
        ),
        // Iterative deepening, rather than an artificial shallow ceiling,
        // decides how far the engine gets inside the user's selected budget.
        max_depth: BROWSER_MAX_DEPTH,
        // Check more frequently in WASM so a completed iteration cannot run
        // materially beyond the UI's deadline.
        deadline_check_interval: BROWSER_DEADLINE_CHECK_INTERVAL,
        ..SearchConfig::default()
    }
}

fn decode<T: for<'de> Deserialize<'de>>(json: &str) -> Result<T, JsValue> {
    serde_json::from_str(json).map_err(|error| js_error(format!("invalid bridge JSON: {error}")))
}

fn encode<T: Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value)
        .map_err(|error| js_error(format!("cannot encode bridge JSON: {error}")))
}

fn js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

fn dto_to_state(position: &PositionDto) -> Result<State, JsValue> {
    if position.schema_version != 1 {
        return Err(js_error("unsupported position schema"));
    }
    let mut state = State::empty(position.turn.into());
    for location in CoreLocation::ALL {
        for card in position.locations.get(location) {
            let core_card = if card.is_snipe {
                CoreCard::Snipe(card.owner.into())
            } else {
                CoreCard::Animal(animal_from_card(card)?)
            };
            state = state.with_card(location, core_card, card.owner.into());
        }
    }
    // Force the rules engine to validate duplicate cards and malformed snipe bits.
    State::from_data(state.to_data())
        .map_err(|error| js_error(format!("invalid position: {error:?}")))
}

fn position_to_dto(state: State, seed: u64, turn_number: u32) -> PositionDto {
    let cards = |location| cards_at(state, location);
    PositionDto {
        schema_version: 1,
        seed,
        turn: state.side_to_move().into(),
        turn_number,
        winner: state.winner().map(Into::into),
        locations: LocationsDto {
            alpha_reserve: cards(CoreLocation::AlphaReserve),
            beta_reserve: cards(CoreLocation::BetaReserve),
            row_1: cards(CoreLocation::Row1),
            row_2: cards(CoreLocation::Row2),
            row_3: cards(CoreLocation::Row3),
            row_4: cards(CoreLocation::Row4),
            row_5: cards(CoreLocation::Row5),
            row_6: cards(CoreLocation::Row6),
        },
    }
}

fn cards_at(state: State, location: CoreLocation) -> Vec<CardDto> {
    let mut cards = Vec::new();
    for player in [Player::Alpha, Player::Beta] {
        for animal in Animal::ALL {
            if state.animal_bits(location, player) & animal.bit() != 0 {
                cards.push(CardDto {
                    id: animal_id(animal),
                    animal: animal_name(animal).to_owned(),
                    owner: player.into(),
                    is_snipe: false,
                    can_retreat: animal.can_retreat(),
                });
            }
        }
        if state.cell(location).has_snipe(player) {
            cards.push(CardDto {
                id: format!("{}-snipe", player_slug(player)),
                animal: "Snipe".to_owned(),
                owner: player.into(),
                is_snipe: true,
                can_retreat: true,
            });
        }
    }
    cards
}

fn move_to_dto(state: State, mv: Move) -> Result<TurnMoveDto, JsValue> {
    let player = state.side_to_move();
    let is_drop = matches!(mv, Move::Drop { .. });
    let mut steps = Vec::new();
    match mv {
        Move::Snipe { destination } => {
            let from = state
                .snipe_location(player)
                .ok_or_else(|| js_error("moving snipe was not found"))?;
            steps.push(step_dto(
                format!("{}-snipe", player_slug(player)),
                from,
                destination.location(),
            ));
        }
        Move::Drop {
            animal,
            destination,
        } => {
            let from = state
                .location_of_animal(animal)
                .ok_or_else(|| js_error("dropped animal was not found"))?;
            steps.push(step_dto(animal_id(animal), from, destination.location()));
        }
        Move::Animals { first, second } => {
            let from = state
                .location_of_animal(first.moved)
                .ok_or_else(|| js_error("first animal was not found"))?;
            steps.push(step_dto(
                animal_id(first.moved),
                from,
                first.destination.location(),
            ));
            if let Some(second) = second {
                let after_first = state
                    .apply_atomic(AtomicMove::Animal(first))
                    .map_err(|error| js_error(format!("invalid first step: {error}")))?;
                let from = after_first
                    .location_of_animal(second.moved)
                    .ok_or_else(|| js_error("second animal was not found"))?;
                steps.push(step_dto(
                    animal_id(second.moved),
                    from,
                    second.destination.location(),
                ));
            }
        }
    }
    let outcome = state
        .apply_move_with_outcome(mv)
        .map_err(|error| js_error(format!("invalid move: {error}")))?;
    let mut captures = Animal::ALL
        .into_iter()
        .filter(|animal| outcome.captured_animals & animal.bit() != 0)
        .map(animal_id)
        .collect::<Vec<_>>();
    for captured in [Player::Alpha, Player::Beta] {
        if outcome.captured_snipes & (1 << captured as u8) != 0 {
            captures.push(format!("{}-snipe", player_slug(captured)));
        }
    }
    let label = steps
        .iter()
        .map(|step| compact_step_label(step, player, is_drop))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(TurnMoveDto {
        id: move_id(mv),
        player: player.into(),
        label,
        steps,
        captures,
    })
}

fn find_move(state: State, id: &str) -> Result<Move, JsValue> {
    state
        .legal_moves()
        .into_iter()
        .find(|mv| move_id(*mv) == id)
        .ok_or_else(|| js_error("move is not legal in this position"))
}

fn find_first_animal_step(
    state: State,
    requested: &MoveStepDto,
) -> Result<snipe_core::AnimalStep, JsValue> {
    state
        .legal_moves()
        .into_iter()
        .find_map(|mv| {
            let Move::Animals { first, .. } = mv else {
                return None;
            };
            let from = state.location_of_animal(first.moved)?;
            let step = step_dto(animal_id(first.moved), from, first.destination.location());
            (step == *requested).then_some(first)
        })
        .ok_or_else(|| js_error("first animal step is not legal in this position"))
}

fn move_id(mv: Move) -> String {
    match mv {
        Move::Snipe { destination } => format!("s:{}", destination.number()),
        Move::Drop {
            animal,
            destination,
        } => {
            format!("d:{}:{}", animal.index(), destination.number())
        }
        Move::Animals { first, second } => match second {
            Some(second) => format!(
                "a:{}:{}:{}:{}",
                first.moved.index(),
                first.destination.number(),
                second.moved.index(),
                second.destination.number()
            ),
            None => format!(
                "a:{}:{}:win",
                first.moved.index(),
                first.destination.number()
            ),
        },
    }
}

fn step_dto(card_id: String, from: CoreLocation, to: CoreLocation) -> MoveStepDto {
    MoveStepDto {
        card_id,
        from: location_id(from).to_owned(),
        to: location_id(to).to_owned(),
    }
}

fn animal_id(animal: Animal) -> String {
    format!("animal-{}", animal.index())
}

fn animal_from_id(id: &str) -> Result<Animal, JsValue> {
    let index = id
        .strip_prefix("animal-")
        .and_then(|value| value.parse::<u8>().ok())
        .and_then(Animal::from_index)
        .ok_or_else(|| js_error(format!("invalid animal card id: {id}")))?;
    Ok(index)
}

fn animal_from_card(card: &CardDto) -> Result<Animal, JsValue> {
    if let Ok(animal) = animal_from_id(&card.id) {
        return Ok(animal);
    }
    // Migration path for games persisted by the pre-WASM preview engine.
    let base = [
        "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Ram", "Monkey", "Rooster",
        "Dog", "Boar", "Fish", "Elephant", "Squid", "Frog",
    ]
    .iter()
    .position(|name| *name == card.animal)
    .ok_or_else(|| js_error(format!("unknown animal card: {}", card.animal)))?;
    let copy = if card.owner == PlayerDto::Alpha {
        0
    } else {
        16
    };
    Animal::from_index((base + copy) as u8)
        .ok_or_else(|| js_error(format!("invalid animal card: {}", card.id)))
}

fn animal_name(animal: Animal) -> &'static str {
    const NAMES: [&str; 16] = [
        "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Ram", "Monkey", "Rooster",
        "Dog", "Boar", "Fish", "Elephant", "Squid", "Frog",
    ];
    NAMES[animal.index() & 15]
}

fn card_label(id: &str) -> String {
    if id == "alpha-snipe" {
        "Alpha".to_owned()
    } else if id == "beta-snipe" {
        "Beta".to_owned()
    } else {
        animal_from_id(id)
            .map(|animal| animal_name(animal).to_owned())
            .unwrap_or_else(|_| id.to_owned())
    }
}

fn compact_step_label(step: &MoveStepDto, player: Player, is_drop: bool) -> String {
    let destination = step.to.trim_start_matches("row-");
    let suffix = if is_drop {
        "!"
    } else {
        let source = step
            .from
            .trim_start_matches("row-")
            .parse::<u8>()
            .expect("non-drop move source is a row");
        let destination_number = destination
            .parse::<u8>()
            .expect("move destination is a row");
        let advances = match player {
            Player::Alpha => destination_number > source,
            Player::Beta => destination_number < source,
        };
        if advances {
            ""
        } else {
            "R"
        }
    };
    format!("{} {destination}{suffix}", card_label(&step.card_id))
}

fn player_slug(player: Player) -> &'static str {
    match player {
        Player::Alpha => "alpha",
        Player::Beta => "beta",
    }
}

fn location_id(location: CoreLocation) -> &'static str {
    match location {
        CoreLocation::AlphaReserve => "alpha-reserve",
        CoreLocation::Row1 => "row-1",
        CoreLocation::Row2 => "row-2",
        CoreLocation::Row3 => "row-3",
        CoreLocation::Row4 => "row-4",
        CoreLocation::Row5 => "row-5",
        CoreLocation::Row6 => "row-6",
        CoreLocation::BetaReserve => "beta-reserve",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_position_round_trips_and_all_moves_apply() {
        for seed in 0..8 {
            let state = State::initial(seed);
            let dto = position_to_dto(state, seed, 1);
            let rebuilt = dto_to_state(&dto).unwrap();
            assert_eq!(rebuilt, state);
            for mv in rebuilt.legal_moves() {
                let dto = move_to_dto(rebuilt, mv).unwrap();
                assert_eq!(find_move(rebuilt, &dto.id).unwrap(), mv);
                rebuilt.apply_move(mv).unwrap();
            }
        }
    }

    #[test]
    fn dto_uses_browser_card_names_and_stable_ids() {
        let dto = position_to_dto(State::initial(7_071), 7_071, 1);
        let cards = CoreLocation::ALL
            .into_iter()
            .flat_map(|location| dto.locations.get(location))
            .collect::<Vec<_>>();
        assert_eq!(cards.len(), 34);
        assert!(cards.iter().any(|card| card.animal == "Rat"));
        assert!(cards.iter().any(|card| card.id == "alpha-snipe"));
        assert!(cards
            .iter()
            .all(|card| card.is_snipe || card.id.starts_with("animal-")));
    }

    #[test]
    fn move_labels_use_compact_ascii_notation() {
        let alpha_advance = MoveStepDto {
            card_id: "animal-0".to_owned(),
            from: "row-2".to_owned(),
            to: "row-3".to_owned(),
        };
        let beta_retreat = MoveStepDto {
            card_id: "animal-3".to_owned(),
            from: "row-5".to_owned(),
            to: "row-6".to_owned(),
        };
        let beta_snipe = MoveStepDto {
            card_id: "beta-snipe".to_owned(),
            from: "row-6".to_owned(),
            to: "row-5".to_owned(),
        };
        assert_eq!(
            compact_step_label(&alpha_advance, Player::Alpha, false),
            "Rat 3"
        );
        assert_eq!(
            compact_step_label(&beta_retreat, Player::Beta, false),
            "Rabbit 6R"
        );
        assert_eq!(
            compact_step_label(&beta_retreat, Player::Beta, true),
            "Rabbit 6!"
        );
        assert_eq!(
            compact_step_label(&beta_snipe, Player::Beta, false),
            "Beta 5"
        );

        let state = State::initial(7_071);
        let two_step = state
            .legal_moves()
            .into_iter()
            .find(|mv| {
                matches!(
                    mv,
                    Move::Animals {
                        second: Some(_),
                        ..
                    }
                )
            })
            .expect("initial position has a two-animal move");
        assert!(move_to_dto(state, two_step).unwrap().label.contains(", "));
    }

    #[test]
    fn live_analysis_root_constraint_keeps_only_matching_first_steps() {
        let state = State::initial(7_071);
        let selected_move = state
            .legal_moves()
            .into_iter()
            .find(|mv| {
                matches!(
                    mv,
                    Move::Animals {
                        second: Some(_),
                        ..
                    }
                )
            })
            .expect("initial position has a two-animal move");
        let Move::Animals {
            first: selected, ..
        } = selected_move
        else {
            unreachable!()
        };
        let requested = move_to_dto(state, selected_move).unwrap().steps.remove(0);

        let constrained = constrained_root_moves(state, &requested).unwrap();

        assert!(!constrained.is_empty());
        assert!(constrained
            .into_iter()
            .all(|mv| matches!(mv, Move::Animals { first, .. } if first == selected)));
    }

    #[test]
    fn principal_variation_dtos_follow_each_resulting_position() {
        let mut state = State::initial(7_071);
        let mut moves = Vec::new();
        let mut expected_ids = Vec::new();
        for _ in 0..3 {
            let mv = state.legal_moves()[0];
            expected_ids.push(move_to_dto(state, mv).unwrap().id);
            moves.push(mv);
            state = state.apply_move(mv).unwrap();
            if state.winner().is_some() {
                break;
            }
        }

        let dtos = principal_variation_to_dtos(State::initial(7_071), &moves).unwrap();

        assert_eq!(dtos.len(), moves.len());
        assert_eq!(
            dtos.into_iter().map(|dto| dto.id).collect::<Vec<_>>(),
            expected_ids
        );
    }

    #[test]
    fn first_step_preview_preserves_turn_and_shows_captures() {
        let state = State::empty(Player::Alpha)
            .with_card(
                CoreLocation::Row1,
                CoreCard::Snipe(Player::Alpha),
                Player::Alpha,
            )
            .with_card(
                CoreLocation::Row6,
                CoreCard::Snipe(Player::Beta),
                Player::Beta,
            )
            .with_card(
                CoreLocation::Row3,
                CoreCard::Animal(Animal::Rooster1),
                Player::Alpha,
            )
            .with_card(
                CoreLocation::Row3,
                CoreCard::Animal(Animal::Dog1),
                Player::Alpha,
            )
            .with_card(
                CoreLocation::Row3,
                CoreCard::Animal(Animal::Horse1),
                Player::Alpha,
            )
            .with_card(
                CoreLocation::Row4,
                CoreCard::Animal(Animal::Mouse2),
                Player::Beta,
            )
            .with_card(
                CoreLocation::Row4,
                CoreCard::Animal(Animal::Tiger2),
                Player::Beta,
            );
        let requested = MoveStepDto {
            card_id: animal_id(Animal::Rooster1),
            from: "row-3".to_owned(),
            to: "row-4".to_owned(),
        };
        let first = find_first_animal_step(state, &requested).unwrap();
        let preview = state.apply_atomic(AtomicMove::Animal(first)).unwrap();
        let dto = position_to_dto(preview, 77, 12);

        assert_eq!(dto.turn, PlayerDto::Alpha);
        assert_eq!(dto.turn_number, 12);
        assert!(dto
            .locations
            .alpha_reserve
            .iter()
            .any(|card| card.id == animal_id(Animal::Mouse2)));
        assert!(dto
            .locations
            .alpha_reserve
            .iter()
            .any(|card| card.id == animal_id(Animal::Tiger2)));
        assert!(dto
            .locations
            .row_4
            .iter()
            .any(|card| card.id == animal_id(Animal::Rooster1)));
    }

    #[test]
    fn browser_search_uses_the_requested_budget_without_a_shallow_depth_cap() {
        let config = browser_search_config(5_000);
        assert_eq!(config.time_limit, Duration::from_secs(5));
        assert_eq!(config.max_depth, BROWSER_MAX_DEPTH);
        assert!(config.max_depth > 8);
        assert_eq!(
            config.deadline_check_interval,
            BROWSER_DEADLINE_CHECK_INTERVAL
        );
    }

    #[test]
    fn browser_search_clamps_only_extreme_time_budgets() {
        assert_eq!(
            browser_search_config(0).time_limit,
            Duration::from_millis(MIN_ANALYSIS_TIME_MS)
        );
        assert_eq!(
            browser_search_config(u64::MAX).time_limit,
            Duration::from_millis(MAX_ANALYSIS_TIME_MS)
        );
    }

    #[test]
    fn analysis_request_history_is_backward_compatible() {
        let request = AnalysisRequestDto {
            position: position_to_dto(State::initial(99), 99, 1),
            time_limit_ms: 1_000,
            request_id: 7,
            history: Vec::new(),
        };
        let mut value = serde_json::to_value(request).unwrap();
        value
            .as_object_mut()
            .expect("analysis request is an object")
            .remove("history");

        let decoded: AnalysisRequestDto = serde_json::from_value(value).unwrap();
        assert!(decoded.history.is_empty());
    }

    #[test]
    fn analysis_request_accepts_prior_positions() {
        let first = State::initial(101);
        let second = first.apply_move(first.legal_moves()[0]).unwrap();
        let prior = vec![
            position_to_dto(first, 101, 1),
            position_to_dto(second, 101, 2),
        ];
        let request = AnalysisRequestDto {
            position: position_to_dto(second, 101, 2),
            time_limit_ms: 5_000,
            request_id: 8,
            history: prior,
        };

        let decoded: AnalysisRequestDto =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(decoded.history.len(), 2);
        let (repetition_hashes, convergence_hashes) = history_context(&decoded.history).unwrap();
        assert_eq!(
            repetition_hashes,
            vec![first.repetition_hash(), second.repetition_hash()]
        );
        assert_eq!(
            convergence_hashes,
            vec![first.convergence_hash(), second.convergence_hash()]
        );
    }
}
