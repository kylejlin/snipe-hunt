import { LabeledImage, VectorLabeledImage, AccuracyRate } from "../data";
import { Matrix } from "../matrix";
import { DeepReadonly } from "../deepReadonly";

export interface Network {
  readonly layerSizes: number[];

  stochasticGradientDescent(
    trainingData: VectorLabeledImage[],
    hyperparams: StochasticGradientDescentHyperParameters,
    evaluationData?: LabeledImage[]
  ): void;

  performForwardPass(inputColumnVector: Matrix): WeightedSumsAndActivations;

  test(testData: LabeledImage[]): AccuracyRate;

  getWeights(): DeepReadonly<MatrixMap>;

  getBiases(): DeepReadonly<MatrixMap>;
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
