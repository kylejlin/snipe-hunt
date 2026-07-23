import { MatrixMap, WeightInitializationMethod } from ".";
import { uniformRandom, normalRandom } from "../random";

export interface Gradients {
  weightGradients: MatrixMap;
  biasGradients: MatrixMap;
}

export function divideIntoMiniBatches<T>(
  trainingData: T[],
  miniBatchSize: number
): T[][] {
  shuffle(trainingData);
  const miniBatches: T[][] = [];
  for (let i = 0; i < trainingData.length; i += miniBatchSize) {
    miniBatches.push(trainingData.slice(i, i + miniBatchSize));
  }
  return miniBatches;
}

function shuffle(arr: unknown[]): void {
  const SHUFFLE_TIMES = 512;

  for (let n = 0; n < SHUFFLE_TIMES; n++) {
    for (let i = arr.length - 1; i >= 1; i--) {
      let j = randInt(i + 1);
      const temp = arr[i];
      arr[i] = arr[j];
      arr[j] = temp;
    }
  }
}

function randInt(exclMax: number): number {
  return Math.floor(Math.random() * exclMax);
}

export function argmax(arr: ArrayLike<number>): number {
  let maxIndex = 0;
  let max = arr[maxIndex];
  for (let i = 1; i < arr.length; i++) {
    const value = arr[i];
    if (value > max) {
      max = value;
      maxIndex = i;
    }
  }
  return maxIndex;
}

export function initializeWeights(
  method: WeightInitializationMethod,
  weights: MatrixMap
): void {
  for (let i = 1; i < weights.length; i++) {
    const matrix = weights[i];
    const initializer: () => number = (() => {
      switch (method) {
        case WeightInitializationMethod.Uniform:
          return uniformRandom;
        case WeightInitializationMethod.LargeGaussian:
          return () => normalRandom(0, 1);
        case WeightInitializationMethod.SmallGaussian:
          return () => normalRandom(0, 1 / Math.sqrt(matrix.columns));
      }
    })();
    matrix.applyElementwiseInto(initializer, matrix);
  }
}
