public enum IllegalGameStateUpdate {
    SnipeAlreadyCaptured, AlreadyMovedAnimal, StepDestinationOutOfRange, CannotEmptyRowWithoutImmediatelyWinning,
    DroppedAnimalNotInReserve, CannotEmptyReserve, CannotDropRetreaterOnEnemysBackTwoRanks, MovedCardInReserve,
    NotYourAnimal, CannotMoveSameAnimalTwice, CannotCaptureOwnSnipeWithoutAlsoCapturingOpponents,

    NothingToUndo;
}