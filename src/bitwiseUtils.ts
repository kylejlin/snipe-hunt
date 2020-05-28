export enum Offset {
  AlphaAnimals = 0,
  BetaAnimals = 1,
  Snipes = 2,
}

export enum Filter {
  LeastSixteenBits = 0b1111_1111_1111_1111,
  LeastBit = 0b1,
  LeastThreeBits = 0b111,
  LeastFiveBits = 0b1_1111,
}

export enum PlyTag {
  SnipeStep = 0b001,
  Drop = 0b010,
  TwoAnimalSteps = 0b011,
}
