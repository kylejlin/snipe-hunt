/** Generates a random float on the interval [-1, 1). */
export function uniformRandom(): number {
  return Math.random() * 2 - 1;
}

// https://en.wikipedia.org/wiki/Box-Muller_transform
export function normalRandom(mean: number = 0, variance: number = 1): number {
  let u1 = 0;
  let u2 = 0;

  // Convert [0,1) to (0,1)
  while (u1 === 0) {
    u1 = Math.random();
  }
  while (u2 === 0) {
    u2 = Math.random();
  }

  const z1 = Math.sqrt(-2.0 * Math.log(u1)) * Math.cos(2.0 * Math.PI * u2);
  return variance * z1 + mean;
}
