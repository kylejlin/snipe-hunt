export enum Offset {
  AlphaAnimals = 0,
  BetaAnimals = 1,
  Snipes = 2,
}

export enum Filter {
  LeastBit = 0b1,
  LeastTwoBits = 0b11,
  LeastThreeBits = 0b111,
  LeastFiveBits = 0b1_1111,
  LeastSixBits = 0b11_1111,
  LeastTenBits = 0b11_1111_1111,
  LeastSixteenBits = 0b1111_1111_1111_1111,
}

export enum PlyTag {
  SnipeStep = 0b001,
  Drop = 0b010,
  TwoAnimalSteps = 0b011,
}

// https://stackoverflow.com/a/43122214/7215455
export function bitCount(n: number): number {
  n = n - ((n >> 1) & 0x55555555);
  n = (n & 0x33333333) + ((n >> 2) & 0x33333333);
  return (((n + (n >> 4)) & 0xf0f0f0f) * 0x1010101) >> 24;
}
