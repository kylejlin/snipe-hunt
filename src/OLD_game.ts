export {};

// import { option, Option, Result, result } from "rusty-ts";
// import { cardProperties } from "./cardMaps";
// import {
//   AppState,
//   Card,
//   CardType,
//   Element,
//   GameState,
//   MutCard,
//   MutGameState,
//   Player,
//   Ply,
//   PlyType,
//   Position,
//   RowNumber,
//   STATE_VERSION,
// } from "./types";

// export enum IllegalMove {
//   GameAlreadyEnded,
//   NotYourCard,
//   AlreadyMoved,
//   AttackerInReserve,
//   TargetInReserve,
//   DestinationOutOfRange,
//   InsufficientElements,
//   CapturesOwnSnipeWithoutCapturingOpponents,
//   CannotEmptyRowWithoutEndingGame,
// }

// export enum IllegalToggle {
//   GameAlreadyEnded,
//   NotYourCard,
//   CannotToggleCardInReserve,
//   SnipeIsUnpromotable,
//   AlreadyMadeSubPly,
//   CannotPromoteUntilMoveIsMade,
// }

// export enum IllegalDrop {
//   GameAlreadyEnded,
//   NotYourCard,
//   AlreadyMadeSubPly,
//   CannotEmptyReserve,
// }

// export enum IllegalUndo {
//   NothingToUndo,
//   IllegalPlyOrSubPly,
// }

// interface ElementCountTable {
//   [Element.Fire]: [boolean, boolean, boolean];
//   [Element.Water]: [boolean, boolean, boolean];
//   [Element.Earth]: [boolean, boolean, boolean];
//   [Element.Air]: [boolean, boolean, boolean];
// }

// export function getRandomState(): AppState {
//   const cards = getShuffledDeck();

//   const alpha: Position = {
//     reserve: [cards.pop()!].map(toUnpromotedAlpha),
//     backRow: [
//       CardType.AlphaSnipe,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//     ].map(toUnpromotedAlpha),
//     frontRow: [cards.pop()!].map(toUnpromotedAlpha),
//   };
//   const beta: Position = {
//     reserve: [cards.pop()!].map(toUnpromotedBeta),
//     backRow: [
//       CardType.BetaSnipe,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//       cards.pop()!,
//     ].map(toUnpromotedBeta),
//     frontRow: [cards.pop()!].map(toUnpromotedBeta),
//   };

//   return {
//     stateVersion: STATE_VERSION,
//     gameState: {
//       turn: Player.Beta,
//       alpha,
//       beta,
//       initialPositions: { alpha, beta },
//       plies: [],
//       futurePlyStack: [],
//       pendingSubPly: option.none(),
//     },
//     selectedCard: option.none(),
//   };
// }

// function getShuffledDeck(): CardType[] {
//   const cards: CardType[] = [
//     CardType.Mouse,
//     CardType.Ox,
//     CardType.Tiger,
//     CardType.Rabbit,
//     CardType.Dragon,
//     CardType.Snake,
//     CardType.Horse,
//     CardType.Ram,
//     CardType.Monkey,
//     CardType.Rooster,
//     CardType.Dog,
//     CardType.Boar,
//     CardType.Fish,
//     CardType.Elephant,
//     CardType.Squid,
//     CardType.Frog,
//   ];
//   shuffle(cards);
//   return cards;
// }

// function shuffle(arr: unknown[]): void {
//   const SHUFFLE_REPETITIONS = 512;
//   const len = arr.length;

//   for (let k = 0; k < SHUFFLE_REPETITIONS; k++) {
//     for (let i = 0; i <= len - 2; i++) {
//       const j = randInt(i, len);
//       const temp = arr[i];
//       arr[i] = arr[j];
//       arr[j] = temp;
//     }
//   }
// }

// function randInt(inclMin: number, exclMax: number): number {
//   const range = exclMax - inclMin;
//   return inclMin + Math.floor(Math.random() * range);
// }

// function toUnpromotedAlpha(cardType: CardType): Card {
//   return { cardType, allegiance: Player.Alpha, isPromoted: false };
// }

// function toUnpromotedBeta(cardType: CardType): Card {
//   return { cardType, allegiance: Player.Beta, isPromoted: false };
// }

// export function tryCapture(
//   state: GameState,
//   attackerType: CardType,
//   targetType: CardType
// ): Result<GameState, IllegalMove> {
//   if (isGameOver(state)) {
//     return result.err(IllegalMove.GameAlreadyEnded);
//   }

//   const attacker = getCard(state, attackerType);
//   const target = getCard(state, targetType);

//   if (attacker.allegiance !== state.turn) {
//     return result.err(IllegalMove.NotYourCard);
//   }

//   if (hasAlreadyMoved(state)) {
//     return result.err(IllegalMove.AlreadyMoved);
//   }

//   const optAttackerRow = getRowNumber(state, attackerType);
//   const optTargetRow = getRowNumber(state, targetType);

//   if (optAttackerRow.isNone()) {
//     return result.err(IllegalMove.AttackerInReserve);
//   }
//   if (optTargetRow.isNone()) {
//     return result.err(IllegalMove.TargetInReserve);
//   }

//   const attackerRow = optAttackerRow.unwrap();
//   const targetRow = optTargetRow.unwrap();

//   if (
//     !(
//       forward(attackerRow, attacker.allegiance).unwrapOr(NaN) === targetRow ||
//       (canMoveBackward(attacker) &&
//         backward(attackerRow, attacker.allegiance).unwrapOr(NaN) === targetRow)
//     )
//   ) {
//     return result.err(IllegalMove.DestinationOutOfRange);
//   }

//   if (!doAttackerElementsTrumpTargetElements(attacker, target)) {
//     return result.err(IllegalMove.InsufficientElements);
//   }

//   const newState = cloneGameStateAsMut(state);
//   let capturedCards: Card[];

//   const mutAttackerRow = getMutRow(newState, attackerRow);
//   removeCardByType(mutAttackerRow, attacker.cardType);

//   const mutTargetRow = getMutRow(newState, targetRow);
//   if (doesEnteringRowActivateTriplet(attacker, mutTargetRow)) {
//     capturedCards = mutTargetRow.slice();

//     mutTargetRow.splice(0, mutTargetRow.length);
//   } else {
//     capturedCards = [target];
//     removeCardByType(mutTargetRow, target.cardType);
//   }

//   newState[getPlayerKey(attacker.allegiance)].reserve.push(
//     ...capturedCards.map(toReserved(attacker.allegiance))
//   );
//   mutTargetRow.push(attacker);

//   if (newState.pendingSubPly.isSome()) {
//     const demotion = newState.pendingSubPly.unwrap() as DemoteSubPly;
//     newState.plies.push({
//       plyType: PlyType.DemoteMove,
//       demoted: demotion.demoted,
//       moved: attacker.cardType,
//       destination: targetRow,
//       captures: capturedCards.map((c) => c.cardType),
//     });
//     newState.turn = opponentOf(newState.turn);
//     newState.pendingSubPly = option.none();
//   } else {
//     newState.pendingSubPly = option.some({
//       subPlyType: SubPlyType.Move,
//       moved: attacker.cardType,
//       destination: targetRow,
//       captures: capturedCards.map((c) => c.cardType),
//     });
//   }

//   if (
//     containsCardType(
//       newState[getPlayerKey(attacker.allegiance)].reserve,
//       getPlayerSnipe(attacker.allegiance)
//     ) &&
//     !containsCardType(
//       newState[getPlayerKey(attacker.allegiance)].reserve,
//       getPlayerSnipe(opponentOf(attacker.allegiance))
//     )
//   ) {
//     return result.err(IllegalMove.CapturesOwnSnipeWithoutCapturingOpponents);
//   }

//   if (getMutRow(newState, attackerRow).length === 0 && !isGameOver(newState)) {
//     return result.err(IllegalMove.CannotEmptyRowWithoutEndingGame);
//   }

//   newState.futurePlyStack = [];

//   return result.ok(newState);
// }

// export function isGameOver(state: GameState): boolean {
//   return getWinner(state).isSome();
// }

// export function getWinner(state: GameState): Option<Player> {
//   if (containsCardType(state.alpha.reserve, CardType.BetaSnipe)) {
//     return option.some(Player.Alpha);
//   }

//   if (containsCardType(state.beta.reserve, CardType.AlphaSnipe)) {
//     return option.some(Player.Beta);
//   }

//   if (!hasLegalPly(state)) {
//     return option.some(opponentOf(state.turn));
//   }

//   return option.none();
// }

// function hasLegalPly(state: GameState): boolean {
//   return state.pendingSubPly.match({
//     none: () => {
//       return hasLegalDrop(state) || hasLegalDemotionMoveOrMovePromotion(state);
//     },
//     some: (subPly) => {
//       switch (subPly.subPlyType) {
//         case SubPlyType.Demote: {
//           return hasLegalMoveSubPly(state);
//         }
//         case SubPlyType.Move: {
//           // If you can move a card, you can definitely promote it.
//           return true;
//         }
//       }
//     },
//   });
// }

// function hasLegalDrop(state: GameState): boolean {
//   const { reserve } = state[getPlayerKey(state.turn)];
//   return reserve.length > 1;
// }

// function hasLegalDemotionMoveOrMovePromotion(state: GameState): boolean {
//   if (hasLegalMoveSubPly(state)) {
//     return true;
//   }

//   const allCards = [
//     ...state.alpha.backRow,
//     ...state.alpha.frontRow,
//     ...state.beta.frontRow,
//     ...state.beta.backRow,
//   ];
//   for (const card of allCards) {
//     if (card.allegiance === state.turn && card.isPromoted) {
//       const afterDemotion = unsafeForceApplyDemotion(state, card.cardType);
//       if (hasLegalMoveSubPly(afterDemotion)) {
//         return true;
//       }
//     }
//   }

//   return false;
// }

// function unsafeForceApplyDemotion(
//   state: GameState,
//   demoted: CardType
// ): GameState {
//   const newState = cloneGameStateAsMut(state);
//   const card = getMutCard(newState, demoted);
//   card.isPromoted = false;
//   return newState;
// }

// function hasLegalMoveSubPly(state: GameState): boolean {
//   for (let i = 1; i <= 4; i++) {
//     const row = i as RowNumber;
//     const rowCards = getMutRow(state, row);

//     for (const card of rowCards) {
//       if (card.allegiance === state.turn) {
//         const forwardRow = forward(row, state.turn);
//         if (forwardRow.isSome()) {
//           const destination = forwardRow.unwrap();
//           const destinationCards = getMutRow(state, destination);
//           const friendlySnipeInDesination = containsCardType(
//             destinationCards,
//             getPlayerSnipe(state.turn)
//           );
//           if (!friendlySnipeInDesination) {
//             return true;
//           }
//           if (doesEnteringRowActivateTriplet(card, destinationCards)) {
//             return true;
//           }

//           // TODO handle single captures
//         }

//         if (canMoveBackward(card)) {
//           const backwardRow = backward(row, state.turn);
//           if (backwardRow.isSome()) {
//             const destination = backwardRow.unwrap();
//             const destinationCards = getMutRow(state, destination);
//             const friendlySnipeInDesination = containsCardType(
//               destinationCards,
//               getPlayerSnipe(state.turn)
//             );
//             if (!friendlySnipeInDesination) {
//               return true;
//             }
//             if (doesEnteringRowActivateTriplet(card, destinationCards)) {
//               return true;
//             }
//           }
//         }
//       }
//     }
//   }

//   return false;
// }

// export function hasAlreadyMoved(state: GameState): boolean {
//   return state.pendingSubPly.match({
//     none: () => false,
//     some: (sub) => sub.subPlyType === SubPlyType.Move,
//   });
// }

// export function getRowNumber(
//   state: GameState,
//   cardType: CardType
// ): Option<RowNumber> {
//   if (containsCardType(state.alpha.backRow, cardType)) {
//     return option.some(1);
//   }
//   if (containsCardType(state.alpha.frontRow, cardType)) {
//     return option.some(2);
//   }
//   if (containsCardType(state.beta.frontRow, cardType)) {
//     return option.some(3);
//   }
//   if (containsCardType(state.beta.backRow, cardType)) {
//     return option.some(4);
//   }
//   return option.none();
// }

// export function getCard(state: GameState, cardType: CardType): Readonly<Card> {
//   return getMutCard(state, cardType);
// }

// export function forward(row: RowNumber, player: Player): Option<RowNumber> {
//   const delta = player === Player.Alpha ? 1 : -1;
//   const newRow = row + delta;
//   if (1 <= newRow && newRow <= 4) {
//     return option.some(newRow as RowNumber);
//   } else {
//     return option.none();
//   }
// }

// export function backward(row: RowNumber, player: Player): Option<RowNumber> {
//   return forward(row, opponentOf(player));
// }

// export function canMoveBackward(card: Card): boolean {
//   const props = cardProperties[card.cardType];

//   if (card.isPromoted) {
//     return props.canPromotedMoveBackward;
//   } else {
//     return props.canUnpromotedMoveBackward;
//   }
// }

// export function doAttackerElementsTrumpTargetElements(
//   attacker: Card,
//   target: Card
// ): boolean {
//   return option
//     .all([getActiveElements(attacker), getActiveElements(target)])
//     .match({
//       none: () => false,
//       some: ([attackerElements, targetElements]) =>
//         doesTrump(attackerElements.double, targetElements.double) &&
//         doesTrump(attackerElements.single, targetElements.single),
//     });
// }

// export function getActiveElements(
//   card: Card
// ): Option<{ double: Element; single: Element }> {
//   return cardProperties[card.cardType].elements.map((elements) =>
//     card.isPromoted ? elements.promoted : elements.unpromoted
//   );
// }

// export function getInactiveElements(
//   card: Card
// ): Option<{ double: Element; single: Element }> {
//   return cardProperties[card.cardType].elements.map((elements) =>
//     card.isPromoted ? elements.unpromoted : elements.promoted
//   );
// }

// function doesTrump(a: Element, b: Element): boolean {
//   return (
//     (a === Element.Fire && b === Element.Earth) ||
//     (a === Element.Fire && b === Element.Air) ||
//     (a === Element.Water && b === Element.Fire) ||
//     (a === Element.Air && b === Element.Water) ||
//     (a === Element.Air && b === Element.Earth)
//   );
// }

// export function cloneGameStateAsMut(state: GameState): MutGameState {
//   const str = JSON.stringify(state, (_k, v) => {
//     if (v !== null && "object" === typeof v && "function" === typeof v.unwrap) {
//       return v.unwrapOr(null);
//     } else {
//       return v;
//     }
//   });
//   const parsed = JSON.parse(str);
//   return {
//     ...parsed,
//     pendingSubPly: option.fromVoidable(parsed.pendingSubPly),
//   };
// }

// export function getMutRow(state: MutGameState, row: RowNumber): Card[] {
//   return [
//     state.alpha.backRow,
//     state.alpha.frontRow,
//     state.beta.frontRow,
//     state.beta.backRow,
//   ][row - 1];
// }

// export function doesEnteringRowActivateTriplet(
//   enteringCard: Card,
//   currentCards: Card[]
// ): boolean {
//   const table: ElementCountTable = {
//     [Element.Fire]: [false, false, false],
//     [Element.Water]: [false, false, false],
//     [Element.Earth]: [false, false, false],
//     [Element.Air]: [false, false, false],
//   };

//   const newCards = currentCards.concat([enteringCard]);

//   for (const card of newCards) {
//     getActiveElements(card).ifSome((elements) => {
//       if (elements.double === elements.single) {
//         table[elements.double][2] = true;
//       } else {
//         table[elements.double][1] = true;
//         table[elements.single][0] = true;
//       }
//     });
//   }

//   const activatedElements: Element[] = getActiveElements(enteringCard).match({
//     none: () => [],
//     some: ({ double, single }) => [double, single],
//   });

//   return (
//     (table[Element.Fire][0] &&
//       table[Element.Fire][1] &&
//       table[Element.Fire][2] &&
//       activatedElements.includes(Element.Fire)) ||
//     (table[Element.Water][0] &&
//       table[Element.Water][1] &&
//       table[Element.Water][2] &&
//       activatedElements.includes(Element.Water)) ||
//     (table[Element.Earth][0] &&
//       table[Element.Earth][1] &&
//       table[Element.Earth][2] &&
//       activatedElements.includes(Element.Earth)) ||
//     (table[Element.Air][0] &&
//       table[Element.Air][1] &&
//       table[Element.Air][2] &&
//       activatedElements.includes(Element.Air))
//   );
// }

// export function removeCardByType(arr: Card[], cardType: CardType): void {
//   arr.splice(
//     arr.findIndex((card) => card.cardType === cardType),
//     1
//   );
// }

// export function getPlayerKey(player: Player): "alpha" | "beta" {
//   switch (player) {
//     case Player.Alpha:
//       return "alpha";
//     case Player.Beta:
//       return "beta";
//   }
// }

// export function toReserved(owner: Player): (card: Card) => Card {
//   return function toReserved(card: Card): Card {
//     return { ...card, allegiance: owner, isPromoted: false };
//   };
// }

// export function containsCardType(cards: Card[], cardType: CardType): boolean {
//   return cards.some((card) => card.cardType === cardType);
// }

// export function getPlayerSnipe(
//   player: Player
// ): CardType.AlphaSnipe | CardType.BetaSnipe {
//   switch (player) {
//     case Player.Alpha:
//       return CardType.AlphaSnipe;
//     case Player.Beta:
//       return CardType.BetaSnipe;
//   }
// }

// export function tryMove(
//   state: GameState,
//   attackerType: CardType,
//   destination: RowNumber
// ): Result<GameState, IllegalMove> {
//   if (isGameOver(state)) {
//     return result.err(IllegalMove.GameAlreadyEnded);
//   }

//   const attacker = getCard(state, attackerType);

//   if (attacker.allegiance !== state.turn) {
//     return result.err(IllegalMove.NotYourCard);
//   }

//   if (hasAlreadyMoved(state)) {
//     return result.err(IllegalMove.AlreadyMoved);
//   }

//   const optAttackerRow = getRowNumber(state, attackerType);

//   if (optAttackerRow.isNone()) {
//     return result.err(IllegalMove.AttackerInReserve);
//   }

//   const attackerRow = optAttackerRow.unwrap();

//   if (
//     !(
//       forward(attackerRow, attacker.allegiance).unwrapOr(NaN) === destination ||
//       (canMoveBackward(attacker) &&
//         backward(attackerRow, attacker.allegiance).unwrapOr(NaN) ===
//           destination)
//     )
//   ) {
//     return result.err(IllegalMove.DestinationOutOfRange);
//   }

//   const newState = cloneGameStateAsMut(state);
//   let capturedCards: Card[];

//   const mutAttackerRow = getMutRow(newState, attackerRow);
//   removeCardByType(mutAttackerRow, attacker.cardType);

//   const mutTargetRow = getMutRow(newState, destination);
//   if (doesEnteringRowActivateTriplet(attacker, mutTargetRow)) {
//     capturedCards = mutTargetRow.slice();

//     mutTargetRow.splice(0, mutTargetRow.length);
//   } else {
//     capturedCards = [];
//   }

//   newState[getPlayerKey(attacker.allegiance)].reserve.push(
//     ...capturedCards.map(toReserved(attacker.allegiance))
//   );
//   mutTargetRow.push(attacker);

//   if (newState.pendingSubPly.isSome()) {
//     const demotion = newState.pendingSubPly.unwrap() as DemoteSubPly;
//     newState.plies.push({
//       plyType: PlyType.DemoteMove,
//       demoted: demotion.demoted,
//       moved: attacker.cardType,
//       destination,
//       captures: capturedCards.map((c) => c.cardType),
//     });
//     newState.turn = opponentOf(newState.turn);
//     newState.pendingSubPly = option.none();
//   } else {
//     newState.pendingSubPly = option.some({
//       subPlyType: SubPlyType.Move,
//       moved: attacker.cardType,
//       destination,
//       captures: capturedCards.map((c) => c.cardType),
//     });
//   }

//   if (
//     containsCardType(
//       newState[getPlayerKey(attacker.allegiance)].reserve,
//       getPlayerSnipe(attacker.allegiance)
//     ) &&
//     !containsCardType(
//       newState[getPlayerKey(attacker.allegiance)].reserve,
//       getPlayerSnipe(opponentOf(attacker.allegiance))
//     )
//   ) {
//     return result.err(IllegalMove.CapturesOwnSnipeWithoutCapturingOpponents);
//   }

//   if (getMutRow(newState, attackerRow).length === 0 && !isGameOver(newState)) {
//     return result.err(IllegalMove.CannotEmptyRowWithoutEndingGame);
//   }

//   newState.futurePlyStack = [];

//   return result.ok(newState);
// }

// export function tryToggle(
//   state: GameState,
//   cardType: CardType
// ): Result<GameState, IllegalToggle> {
//   if (isGameOver(state)) {
//     return result.err(IllegalToggle.GameAlreadyEnded);
//   }

//   const card = getCard(state, cardType);

//   if (state.turn !== card.allegiance) {
//     return result.err(IllegalToggle.NotYourCard);
//   }

//   if (getRowNumber(state, card.cardType).isNone()) {
//     return result.err(IllegalToggle.CannotToggleCardInReserve);
//   }

//   if (isSnipe(card.cardType)) {
//     return result.err(IllegalToggle.SnipeIsUnpromotable);
//   }

//   if (card.isPromoted) {
//     if (state.pendingSubPly.isSome()) {
//       return result.err(IllegalToggle.AlreadyMadeSubPly);
//     }

//     const newState = cloneGameStateAsMut(state);
//     getMutCard(newState, cardType).isPromoted = false;
//     newState.pendingSubPly = option.some({
//       subPlyType: SubPlyType.Demote,
//       demoted: card.cardType,
//     });
//     return result.ok(newState);
//   } else {
//     if (!hasAlreadyMoved(state)) {
//       return result.err(IllegalToggle.CannotPromoteUntilMoveIsMade);
//     }

//     const newState = cloneGameStateAsMut(state);
//     getMutCard(newState, cardType).isPromoted = true;
//     const move = newState.pendingSubPly.unwrap() as MoveSubPly;
//     newState.plies.push({
//       plyType: PlyType.MovePromote,
//       moved: move.moved,
//       destination: move.destination,
//       captures: move.captures,
//       promoted: card.cardType,
//     });
//     newState.turn = opponentOf(newState.turn);
//     newState.pendingSubPly = option.none();
//     return result.ok(newState);
//   }
// }

// export function isSnipe(cardType: CardType): boolean {
//   return cardType === CardType.AlphaSnipe || cardType === CardType.BetaSnipe;
// }

// export function getMutCard(state: GameState, cardType: CardType): MutCard {
//   function matchesType(card: Card): boolean {
//     return card.cardType === cardType;
//   }

//   return (state.alpha.reserve.find(matchesType) ||
//     state.alpha.backRow.find(matchesType) ||
//     state.alpha.frontRow.find(matchesType) ||
//     state.beta.frontRow.find(matchesType) ||
//     state.beta.backRow.find(matchesType) ||
//     state.beta.reserve.find(matchesType))!;
// }

// export function opponentOf(player: Player): Player {
//   switch (player) {
//     case Player.Alpha:
//       return Player.Beta;
//     case Player.Beta:
//       return Player.Alpha;
//   }
// }

// export function tryDrop(
//   state: GameState,
//   cardType: CardType,
//   destination: RowNumber
// ): Result<GameState, IllegalDrop> {
//   if (isGameOver(state)) {
//     return result.err(IllegalDrop.GameAlreadyEnded);
//   }

//   const card = getCard(state, cardType);

//   if (state.turn !== card.allegiance) {
//     return result.err(IllegalDrop.NotYourCard);
//   }

//   if (state.pendingSubPly.isSome()) {
//     return result.err(IllegalDrop.AlreadyMadeSubPly);
//   }

//   if (state[getPlayerKey(card.allegiance)].reserve.length === 1) {
//     return result.err(IllegalDrop.CannotEmptyReserve);
//   }

//   const newState = cloneGameStateAsMut(state);
//   removeCardByType(
//     newState[getPlayerKey(newState.turn)].reserve,
//     card.cardType
//   );
//   getMutRow(newState, destination).push(card);
//   newState.plies.push({
//     plyType: PlyType.Drop,
//     dropped: card.cardType,
//     destination,
//   });
//   newState.turn = opponentOf(newState.turn);
//   newState.futurePlyStack = [];
//   return result.ok(newState);
// }

// export function getCardsWithActiveElements(
//   cards: Card[],
//   amount: 1 | 2 | 3,
//   element: Element
// ): Card[] {
//   return cards.filter((card) =>
//     getActiveElements(card).match({
//       none: () => false,
//       some: ({ double, single }) => {
//         if (double === single) {
//           return amount === 3 && double === element;
//         }

//         return (
//           (double === element && amount === 2) ||
//           (single === element && amount === 1)
//         );
//       },
//     })
//   );
// }

// export function getCardsWithInactiveElements(
//   cards: Card[],
//   amount: 1 | 2 | 3,
//   element: Element
// ): Card[] {
//   return cards.filter((card) =>
//     getInactiveElements(card).match({
//       none: () => false,
//       some: ({ double, single }) => {
//         if (double === single) {
//           return amount === 3 && double === element;
//         }

//         return (
//           (double === element && amount === 2) ||
//           (single === element && amount === 1)
//         );
//       },
//     })
//   );
// }

// export function tryUndoPlyOrSubPly(
//   state: GameState
// ): Result<GameState, IllegalUndo> {
//   const newState = cloneGameStateAsMut(state);

//   if (newState.pendingSubPly.isSome()) {
//     newState.pendingSubPly = option.none();
//     return recalculateOutOfSyncGameState(newState).mapErr(
//       () => IllegalUndo.IllegalPlyOrSubPly
//     );
//   }

//   if (newState.plies.length === 0) {
//     return result.err(IllegalUndo.NothingToUndo);
//   }

//   const removedPly = newState.plies.pop()!;
//   newState.futurePlyStack.push(removedPly);
//   return recalculateOutOfSyncGameState(newState).mapErr(
//     () => IllegalUndo.IllegalPlyOrSubPly
//   );
// }

// export function recalculateOutOfSyncGameState(
//   state: GameState
// ): Result<GameState, IllegalToggle | IllegalMove | IllegalDrop> {
//   const initState: GameState = {
//     turn: Player.Beta,
//     alpha: state.initialPositions.alpha,
//     beta: state.initialPositions.beta,
//     initialPositions: state.initialPositions,
//     plies: [],
//     futurePlyStack: [],
//     pendingSubPly: option.none(),
//   };

//   const withFullPliesApplied = state.plies.reduce(
//     (
//       currentStateOrError: Result<
//         GameState,
//         IllegalToggle | IllegalMove | IllegalDrop
//       >,
//       ply
//     ) =>
//       currentStateOrError.andThen((currentState) =>
//         tryApplyPly(currentState, ply)
//       ),
//     result.ok(initState)
//   );
//   const withPendingSubPlyApplied = withFullPliesApplied.andThen(
//     (currentState) =>
//       currentState.pendingSubPly.match({
//         none: () => result.ok(currentState),
//         some: (subPly) => tryApplySubPly(currentState, subPly),
//       })
//   );

//   return withPendingSubPlyApplied.map((currentState) => {
//     return {
//       ...currentState,
//       futurePlyStack: state.futurePlyStack,
//     };
//   });
// }

// function tryApplyPly(
//   state: GameState,
//   ply: Ply
// ): Result<GameState, IllegalToggle | IllegalMove | IllegalDrop> {
//   switch (ply.plyType) {
//     case PlyType.DemoteMove:
//       return tryToggle(state, ply.demoted).andThen((afterDemotion) =>
//         tryMoveOrCapture(afterDemotion, ply)
//       );
//     case PlyType.MovePromote:
//       return tryMoveOrCapture(state, ply).andThen((afterMoveOrCapture) =>
//         tryToggle(afterMoveOrCapture, ply.promoted)
//       );
//     case PlyType.Drop:
//       return tryDrop(state, ply.dropped, ply.destination);
//   }
// }

// function tryMoveOrCapture(
//   state: GameState,
//   ply: { moved: CardType; destination: RowNumber; captures: CardType[] }
// ): Result<GameState, IllegalMove> {
//   if (ply.captures.length === 1) {
//     return tryCapture(state, ply.moved, ply.captures[0]);
//   } else {
//     return tryMove(state, ply.moved, ply.destination);
//   }
// }

// function tryApplySubPly(
//   state: GameState,
//   subPly: SubPly
// ): Result<GameState, IllegalToggle | IllegalMove> {
//   switch (subPly.subPlyType) {
//     case SubPlyType.Demote:
//       return tryToggle(state, subPly.demoted);
//     case SubPlyType.Move:
//       return tryMoveOrCapture(state, subPly);
//   }
// }

// export function canCapture(
//   state: GameState,
//   attackerType: CardType,
//   targetType: CardType
// ): boolean {
//   const attacker = getCard(state, attackerType);
//   const target = getCard(state, targetType);
//   return doAttackerElementsTrumpTargetElements(attacker, target);
// }

// export function backRowNumber(player: Player): 1 | 4 {
//   switch (player) {
//     case Player.Alpha:
//       return 1;
//     case Player.Beta:
//       return 4;
//   }
// }

// export function frontRowNumber(player: Player): 2 | 3 {
//   switch (player) {
//     case Player.Alpha:
//       return 2;
//     case Player.Beta:
//       return 3;
//   }
// }
