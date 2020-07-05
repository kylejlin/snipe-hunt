import { Matrix } from "../matrix";
import { DeepReadonly } from "../deepReadonly";
import { Player } from "../types";

export interface Network {
  readonly layerSizes: number[];

  stochasticGradientDescent(
    trainingData: GamePosition[],
    hyperparams: StochasticGradientDescentHyperParameters
  ): void;

  performForwardPass(position: GamePosition): WeightedSumsAndActivations;

  getWeights(): DeepReadonly<MatrixMap>;

  getBiases(): DeepReadonly<MatrixMap>;
}

export interface GamePosition {
  gameState: Matrix;
  legalActions: ArrayLike<number | boolean>;
  actionProbabilitiesAndValue: Matrix;
}

export interface WeightedSumsAndActivations {
  weightedSums: MatrixMap;
  activations: MatrixMap;
}

/**
 * `MatrixMap` differs from `ArrayLike<Matrix>` in that
 * indices start at `1`. Because the input layer (layer
 * `0`) does not have a weight matrix or bias matrix,
 * `MatrixMap[0]` will be undefined.
 */
export interface MatrixMap {
  [layer: number]: Matrix;
  length: number;
}

export enum WeightInitializationMethod {
  Uniform = "Uniform",
  LargeGaussian = "LargeGaussian",
  SmallGaussian = "SmallGaussian",
}

export interface StochasticGradientDescentHyperParameters {
  batchSize: number;
  epochs: number;
  learningRate: number;
  regularizationRate: number;
  momentumCoefficient: number;
}
