import { Option, option, Result, result } from "rusty-ts";
import {
  backward,
  canMoveBackward,
  cloneGameStateAsMut,
  containsCardType,
  doAttackerElementsTrumpTargetElements,
  doesEnteringRowActivateTriplet,
  forward,
  getActiveElements,
  getCard,
  getInactiveElements,
  getMutCard,
  getMutRow,
  getPlayerKey,
  getPlayerSnipe,
  getRowNumber,
  IllegalDrop,
  IllegalMove,
  isGameOver,
  isSnipe,
  opponentOf,
  removeCardByType,
  toReserved,
  tryDrop,
  getWinner,
} from "./game";
import { Card, CardType, Drop, GameState, Player, PlyType, Row } from "./types";

export type KrakenPly = Drop | KrakenMove;

export enum KrakenPlyType {
  KrakenMove,
}

export interface KrakenMove {
  plyType: KrakenPlyType.KrakenMove;
  moved: CardType;
  destination: Row;
  capture: Option<CardType>;
  isPromoted: boolean;
}

export interface RatedPly {
  ply: KrakenPly;
  value: number;
}

const VICTORY = 1000;

export function getLegalPlies(originalState: GameState): KrakenPly[] {
  const state: GameState = { ...originalState, pendingSubPly: option.none() };

  if (isGameOver(state)) {
    return [];
  }

  const plies: KrakenPly[] = [];

  const { reserve } = state[getPlayerKey(state.turn)];
  if (reserve.length > 1) {
    for (const card of reserve) {
      for (let destination = 1; destination < 4; destination++) {
        const drop: Drop = {
          plyType: PlyType.Drop,
          dropped: card.cardType,
          destination: destination as Row,
        };
        plies.push(drop);
      }
    }
  }

  for (let i = 1; i <= 4; i++) {
    const rowNumber = i as Row;
    const friendlyRowCards = getMutRow(state, rowNumber).filter(
      (card) => card.allegiance === state.turn
    );
    const optForwardDestination = forward(rowNumber, state.turn);
    const optBackwardDestination = backward(rowNumber, state.turn);

    optForwardDestination.ifSome((forwardDestination) => {
      const couldForwardMoveBeIllegal =
        friendlyRowCards.length === 1 ||
        containsCardType(
          getMutRow(state, forwardDestination),
          getPlayerSnipe(state.turn)
        );
      const isForwardMoveDefinitelyLegal = !couldForwardMoveBeIllegal;

      for (const attacker of friendlyRowCards) {
        const unpromoted: KrakenMove = {
          plyType: KrakenPlyType.KrakenMove,
          moved: attacker.cardType,
          destination: forwardDestination,
          capture: option.none(),
          isPromoted: false,
        };

        if (isSnipe(attacker.cardType)) {
          if (friendlyRowCards.length > 1) {
            plies.push(unpromoted);
          }
          continue;
        }

        if (
          isForwardMoveDefinitelyLegal ||
          tryApplyPly(state, unpromoted).isOk()
        ) {
          plies.push(unpromoted);
        }

        const promoted: KrakenMove = {
          plyType: KrakenPlyType.KrakenMove,
          moved: attacker.cardType,
          destination: forwardDestination,
          capture: option.none(),
          isPromoted: true,
        };
        if (
          isForwardMoveDefinitelyLegal ||
          tryApplyPly(state, promoted).isOk()
        ) {
          plies.push(promoted);
        }

        const forwardRowCards = getMutRow(state, forwardDestination);
        for (const target of forwardRowCards) {
          const unpromotedCapture: KrakenMove = {
            plyType: KrakenPlyType.KrakenMove,
            moved: attacker.cardType,
            destination: forwardDestination,
            capture: option.some(target.cardType),
            isPromoted: false,
          };

          if (tryApplyPly(state, unpromotedCapture).isOk()) {
            plies.push(unpromotedCapture);
          }

          const promotedCapture: KrakenMove = {
            plyType: KrakenPlyType.KrakenMove,
            moved: attacker.cardType,
            destination: forwardDestination,
            capture: option.some(target.cardType),
            isPromoted: true,
          };

          if (tryApplyPly(state, promotedCapture).isOk()) {
            plies.push(promotedCapture);
          }
        }
      }
    });

    optBackwardDestination.ifSome((backwardDestination) => {
      const couldBackwardMoveBeIllegal =
        friendlyRowCards.length === 1 ||
        containsCardType(
          getMutRow(state, backwardDestination),
          getPlayerSnipe(state.turn)
        );
      const isBackwardMoveDefinitelyLegal = !couldBackwardMoveBeIllegal;

      for (const attacker of friendlyRowCards) {
        if (!canMoveBackward(attacker)) {
          continue;
        }

        const unpromoted: KrakenMove = {
          plyType: KrakenPlyType.KrakenMove,
          moved: attacker.cardType,
          destination: backwardDestination,
          capture: option.none(),
          isPromoted: false,
        };

        if (isSnipe(attacker.cardType)) {
          if (friendlyRowCards.length > 1) {
            plies.push(unpromoted);
          }
          continue;
        }

        if (
          isBackwardMoveDefinitelyLegal ||
          tryApplyPly(state, unpromoted).isOk()
        ) {
          plies.push(unpromoted);
        }

        const promoted: KrakenMove = {
          plyType: KrakenPlyType.KrakenMove,
          moved: attacker.cardType,
          destination: backwardDestination,
          capture: option.none(),
          isPromoted: true,
        };
        if (
          isBackwardMoveDefinitelyLegal ||
          tryApplyPly(state, promoted).isOk()
        ) {
          plies.push(promoted);
        }

        const backwardRowCards = getMutRow(state, backwardDestination);
        for (const target of backwardRowCards) {
          const unpromotedCapture: KrakenMove = {
            plyType: KrakenPlyType.KrakenMove,
            moved: attacker.cardType,
            destination: backwardDestination,
            capture: option.some(target.cardType),
            isPromoted: false,
          };

          if (tryApplyPly(state, unpromotedCapture).isOk()) {
            plies.push(unpromotedCapture);
          }

          const promotedCapture: KrakenMove = {
            plyType: KrakenPlyType.KrakenMove,
            moved: attacker.cardType,
            destination: backwardDestination,
            capture: option.some(target.cardType),
            isPromoted: true,
          };

          if (tryApplyPly(state, promotedCapture).isOk()) {
            plies.push(promotedCapture);
          }
        }
      }
    });
  }

  (window as any).lpState = state;

  return plies;
}

export function getBestPly(originalState: GameState, depth: number): RatedPly {
  const state: GameState = { ...originalState, pendingSubPly: option.none() };

  const legalPlies = getLegalPlies(state);

  if (legalPlies.length === 0) {
    throw new Error("Cannot getBestPly when there are zero legal plies.");
  }

  if (depth === 0) {
    switch (state.turn) {
      case Player.Alpha: {
        let max: RatedPly = {
          ply: legalPlies[0],
          value: getValue(
            tryApplyPly(state, legalPlies[0]).unwrapOrElse((errorCode) => {
              console.log(state, legalPlies[0]);
              throw new Error(
                "Tried to apply allegedly legal ply but got: " +
                  IllegalDrop[errorCode] +
                  " or " +
                  IllegalMove[errorCode] +
                  " (we can't tell which) "
              );
            })
          ),
        };
        for (const ply of legalPlies.slice(1)) {
          const rated: RatedPly = {
            ply,
            value: getValue(
              tryApplyPly(state, ply).unwrapOrElse((errorCode) => {
                console.log(state, ply);
                throw new Error(
                  "Tried to apply allegedly legal ply but got: " +
                    IllegalDrop[errorCode] +
                    " or " +
                    IllegalMove[errorCode] +
                    " (we can't tell which) "
                );
              })
            ),
          };
          if (rated.value > max.value) {
            max = rated;
          }
        }
        return max;
      }
      case Player.Beta: {
        let min: RatedPly = {
          ply: legalPlies[0],
          value: getValue(
            tryApplyPly(state, legalPlies[0]).unwrapOrElse((errorCode) => {
              console.log(state, legalPlies[0]);
              throw new Error(
                "Tried to apply allegedly legal ply but got: " +
                  IllegalDrop[errorCode] +
                  " or " +
                  IllegalMove[errorCode] +
                  " (we can't tell which) "
              );
            })
          ),
        };
        for (const ply of legalPlies.slice(1)) {
          const rated: RatedPly = {
            ply,
            value: getValue(
              tryApplyPly(state, ply).unwrapOrElse((errorCode) => {
                console.log(state, ply);
                throw new Error(
                  "Tried to apply allegedly legal ply but got: " +
                    IllegalDrop[errorCode] +
                    " or " +
                    IllegalMove[errorCode] +
                    " (we can't tell which) "
                );
              })
            ),
          };
          if (rated.value < min.value) {
            min = rated;
          }
        }
        return min;
      }
    }
  }

  switch (state.turn) {
    case Player.Alpha: {
      let maxPly: RatedPly | undefined = undefined;
      for (const ply of legalPlies) {
        const newState = tryApplyPly(state, ply).unwrapOrElse((errorCode) => {
          console.log(state, ply);
          throw new Error(
            "Tried to apply allegedly legal ply but got: " +
              IllegalDrop[errorCode] +
              " or " +
              IllegalMove[errorCode] +
              " (we can't tell which) "
          );
        });

        if (isGameOver(newState)) {
          maxPly = { ply, value: VICTORY };
          break;
        }

        const bestBetaPly = getBestPly(newState, depth - 1);
        if (maxPly === undefined || bestBetaPly.value > maxPly.value) {
          maxPly = { ply, value: bestBetaPly.value };
        }
      }
      return maxPly!;
    }
    case Player.Beta: {
      let minPly: RatedPly | undefined = undefined;
      for (const ply of legalPlies) {
        const newState = tryApplyPly(state, ply).unwrapOrElse((errorCode) => {
          console.log(state, ply);
          throw new Error(
            "Tried to apply allegedly legal ply but got: " +
              IllegalDrop[errorCode] +
              " or " +
              IllegalMove[errorCode] +
              " (we can't tell which) "
          );
        });

        if (isGameOver(newState)) {
          minPly = { ply, value: -VICTORY };
          break;
        }

        const bestAlphaPly = getBestPly(newState, depth - 1);
        if (minPly === undefined || bestAlphaPly.value < minPly.value) {
          minPly = { ply, value: bestAlphaPly.value };
        }
      }
      return minPly!;
    }
  }
}

function getValue(state: GameState): number {
  if (getWinner(state).isSome()) {
    const winner = getWinner(state).unwrap();
    if (winner === Player.Alpha) {
      return VICTORY;
    } else {
      return -VICTORY;
    }
  }

  const allCards: Card[] = [
    ...state.alpha.reserve,
    ...state.alpha.backRow,
    ...state.alpha.frontRow,
    ...state.beta.frontRow,
    ...state.beta.backRow,
    ...state.beta.reserve,
  ];

  let value = 0;
  for (const card of allCards) {
    const isTriple =
      getActiveElements(card).match({
        none: () => false,
        some: ({ double, single }) => double === single,
      }) ||
      getInactiveElements(card).match({
        none: () => false,
        some: ({ double, single }) => double === single,
      });
    const cardAbsoluteValue = isTriple ? 3 : 1;

    const allegianceMultiplier = card.allegiance === Player.Alpha ? 1 : -1;

    value += cardAbsoluteValue * allegianceMultiplier;
  }
  return value;
}

function tryApplyPly(
  state: GameState,
  ply: KrakenPly
): Result<GameState, IllegalDrop | IllegalMove> {
  switch (ply.plyType) {
    case PlyType.Drop:
      return tryDrop(state, ply.dropped, ply.destination);
    case KrakenPlyType.KrakenMove:
      return tryKrakenMove(state, ply);
  }
}

function tryKrakenMove(
  state: GameState,
  ply: KrakenMove
): Result<GameState, IllegalMove> {
  return ply.capture.match({
    none: () =>
      tryNonSingleCaptureKrakenMove(
        state,
        ply.moved,
        ply.destination,
        ply.isPromoted
      ),
    some: (capturedCardType) =>
      tryKrakenSingleCapture(
        state,
        ply.moved,
        capturedCardType,
        ply.isPromoted
      ),
  });
}

export function tryNonSingleCaptureKrakenMove(
  oState: GameState,
  attackerType: CardType,
  destination: Row,
  isPromoted: boolean
): Result<GameState, IllegalMove> {
  const newState = cloneGameStateAsMut(oState);

  if (isGameOver(newState)) {
    return result.err(IllegalMove.GameAlreadyEnded);
  }

  const attacker = getMutCard(newState, attackerType);

  if (attacker.allegiance !== newState.turn) {
    return result.err(IllegalMove.NotYourCard);
  }

  const optAttackerRow = getRowNumber(newState, attackerType);

  if (optAttackerRow.isNone()) {
    return result.err(IllegalMove.AttackerInReserve);
  }

  const attackerRow = optAttackerRow.unwrap();

  if (
    !(
      forward(attackerRow, attacker.allegiance).unwrapOr(NaN) === destination ||
      (canMoveBackward(attacker) &&
        backward(attackerRow, attacker.allegiance).unwrapOr(NaN) ===
          destination)
    )
  ) {
    return result.err(IllegalMove.DestinationOutOfRange);
  }

  let capturedCards: Card[];

  const mutAttackerRow = getMutRow(newState, attackerRow);
  removeCardByType(mutAttackerRow, attacker.cardType);

  attacker.isPromoted = isPromoted;

  const mutTargetRow = getMutRow(newState, destination);
  if (doesEnteringRowActivateTriplet(attacker, mutTargetRow)) {
    capturedCards = mutTargetRow.slice();

    mutTargetRow.splice(0, mutTargetRow.length);
  } else {
    capturedCards = [];
  }

  newState[getPlayerKey(attacker.allegiance)].reserve.push(
    ...capturedCards.map(toReserved(attacker.allegiance))
  );
  mutTargetRow.push(attacker);

  if (
    containsCardType(
      newState[getPlayerKey(attacker.allegiance)].reserve,
      getPlayerSnipe(attacker.allegiance)
    ) &&
    !containsCardType(
      newState[getPlayerKey(attacker.allegiance)].reserve,
      getPlayerSnipe(opponentOf(attacker.allegiance))
    )
  ) {
    return result.err(IllegalMove.CapturesOwnSnipeWithoutCapturingOpponents);
  }

  if (getMutRow(newState, attackerRow).length === 0 && !isGameOver(newState)) {
    return result.err(IllegalMove.CannotEmptyRowWithoutEndingGame);
  }

  newState.turn = opponentOf(newState.turn);

  return result.ok(newState);
}

export function tryKrakenSingleCapture(
  state: GameState,
  attackerType: CardType,
  targetType: CardType,
  isPromoted: boolean
): Result<GameState, IllegalMove> {
  const newState = cloneGameStateAsMut(state);

  if (isGameOver(newState)) {
    return result.err(IllegalMove.GameAlreadyEnded);
  }

  const attacker = getMutCard(newState, attackerType);
  const target = getCard(newState, targetType);

  if (attacker.allegiance !== newState.turn) {
    return result.err(IllegalMove.NotYourCard);
  }

  const optAttackerRow = getRowNumber(newState, attackerType);
  const optTargetRow = getRowNumber(newState, targetType);

  if (optAttackerRow.isNone()) {
    return result.err(IllegalMove.AttackerInReserve);
  }
  if (optTargetRow.isNone()) {
    return result.err(IllegalMove.TargetInReserve);
  }

  const attackerRow = optAttackerRow.unwrap();
  const targetRow = optTargetRow.unwrap();

  if (
    !(
      forward(attackerRow, attacker.allegiance).unwrapOr(NaN) === targetRow ||
      (canMoveBackward(attacker) &&
        backward(attackerRow, attacker.allegiance).unwrapOr(NaN) === targetRow)
    )
  ) {
    return result.err(IllegalMove.DestinationOutOfRange);
  }

  attacker.isPromoted = isPromoted;

  if (!doAttackerElementsTrumpTargetElements(attacker, target)) {
    return result.err(IllegalMove.InsufficientElements);
  }

  let capturedCards: Card[];

  const mutAttackerRow = getMutRow(newState, attackerRow);
  removeCardByType(mutAttackerRow, attacker.cardType);

  const mutTargetRow = getMutRow(newState, targetRow);
  if (doesEnteringRowActivateTriplet(attacker, mutTargetRow)) {
    capturedCards = mutTargetRow.slice();

    mutTargetRow.splice(0, mutTargetRow.length);
  } else {
    capturedCards = [target];
    removeCardByType(mutTargetRow, target.cardType);
  }

  newState[getPlayerKey(attacker.allegiance)].reserve.push(
    ...capturedCards.map(toReserved(attacker.allegiance))
  );
  mutTargetRow.push(attacker);

  if (
    containsCardType(
      newState[getPlayerKey(attacker.allegiance)].reserve,
      getPlayerSnipe(attacker.allegiance)
    ) &&
    !containsCardType(
      newState[getPlayerKey(attacker.allegiance)].reserve,
      getPlayerSnipe(opponentOf(attacker.allegiance))
    )
  ) {
    return result.err(IllegalMove.CapturesOwnSnipeWithoutCapturingOpponents);
  }

  if (getMutRow(newState, attackerRow).length === 0 && !isGameOver(newState)) {
    return result.err(IllegalMove.CannotEmptyRowWithoutEndingGame);
  }

  newState.turn = opponentOf(newState.turn);

  return result.ok(newState);
}
