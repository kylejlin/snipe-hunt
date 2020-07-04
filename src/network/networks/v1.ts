import {
  MatrixMap,
  Network,
  StochasticGradientDescentHyperParameters,
  WeightedSumsAndActivations,
  WeightInitializationMethod,
} from "..";
import { AccuracyRate, LabeledImage, VectorLabeledImage } from "../../data";
import { DeepReadonly } from "../../deepReadonly";
import { Matrix } from "../../matrix";
import {
  argmax,
  divideIntoMiniBatches,
  Gradients,
  initializeWeights,
} from "../utils";

/**
 * Categorical cross-entropy cost,
 * softmax activation in last layer,
 * all other layers use sigmoid activation function,
 * L2 regularization,
 * momentum
 */
export class NetworkV1 implements Network {
  private numberOfLayers: number;
  private weights: MatrixMap;
  private biases: MatrixMap;
  private weightVelocities: MatrixMap;
  private biasVelocities: MatrixMap;
  private log: (accuracyRate: AccuracyRate, epoch: number) => void;

  private temp_totalWeightGradients: MatrixMap;
  private temp_totalBiasGradients: MatrixMap;

  private temp_weightedSums: MatrixMap;
  private temp_activations: MatrixMap;

  private temp_errors: MatrixMap;
  private temp_weightGradients: MatrixMap;
  private temp_biasGradients: MatrixMap;
  private temp_transposedActivations: MatrixMap;
  private temp_weightCosts: MatrixMap;
  private temp_transposedWeights: MatrixMap;
  private temp_sigmaPrimeOfWeightedSums: MatrixMap;

  public readonly layerSizes: number[];

  static fromWeightsAndBiases(weights: MatrixMap, biases: MatrixMap): Network {
    return new NetworkV1(weights, biases);
  }

  static fromLayerSizes(
    layerSizes: number[],
    initializationMethod: WeightInitializationMethod,
    log?: (accuracyRate: AccuracyRate, epoch: number) => void
  ): Network {
    const numberOfLayers = layerSizes.length;

    const weights: MatrixMap = new Array(numberOfLayers);
    const biases: MatrixMap = new Array(numberOfLayers);
    for (let outputLayer = 1; outputLayer < numberOfLayers; outputLayer++) {
      const inputLayer = outputLayer - 1;
      const outputLayerSize = layerSizes[outputLayer];
      const inputLayerSize = layerSizes[inputLayer];
      weights[outputLayer] = Matrix.zeros(outputLayerSize, inputLayerSize);
      biases[outputLayer] = Matrix.zeros(outputLayerSize, 1);
    }

    initializeWeights(initializationMethod, weights);

    return new NetworkV1(weights, biases, log);
  }

  private constructor(
    weights: MatrixMap,
    biases: MatrixMap,
    log?: (accuracyRate: AccuracyRate, epoch: number) => void
  ) {
    const layerSizes = [weights[1].columns];
    for (let i = 1; i < weights.length; i++) {
      layerSizes.push(weights[i].rows);
    }

    this.layerSizes = layerSizes;
    this.numberOfLayers = layerSizes.length;
    this.weights = weights;
    this.biases = biases;
    this.weightVelocities = getZeroMatrixMap(weights);
    this.biasVelocities = getZeroMatrixMap(biases);
    this.log = log || (() => {});

    this.temp_totalWeightGradients = getZeroMatrixMap(weights);
    this.temp_totalBiasGradients = getZeroMatrixMap(biases);

    {
      const weightedSums = [];
      const activations = [Matrix.zeros(weights[1].columns, 1)];

      for (
        let outputLayer = 1;
        outputLayer < this.numberOfLayers;
        outputLayer++
      ) {
        weightedSums[outputLayer] = Matrix.zeros(weights[outputLayer].rows, 1);
        activations[outputLayer] = Matrix.zeros(weights[outputLayer].rows, 1);
      }

      this.temp_weightedSums = weightedSums;
      this.temp_activations = activations;
    }

    this.temp_errors = getZeroMatrixMap(this.temp_weightedSums);
    this.temp_weightGradients = getZeroMatrixMap(weights);
    this.temp_biasGradients = getZeroMatrixMap(biases);

    {
      const activations = this.temp_activations;
      const transposedActivations: MatrixMap = new Array(activations.length);

      for (
        let outputLayer = 0;
        outputLayer < activations.length;
        outputLayer++
      ) {
        transposedActivations[outputLayer] = Matrix.zeros(
          activations[outputLayer].columns,
          activations[outputLayer].rows
        );
      }

      this.temp_transposedActivations = transposedActivations;
    }

    this.temp_weightCosts = getZeroMatrixMap(this.temp_weightGradients);

    {
      const { weights } = this;
      const transposedWeights: MatrixMap = new Array(weights.length);
      for (let i = 1; i < weights.length; i++) {
        transposedWeights[i] = Matrix.zeros(
          weights[i].columns,
          weights[i].rows
        );
      }
      this.temp_transposedWeights = transposedWeights;
    }

    this.temp_sigmaPrimeOfWeightedSums = getZeroMatrixMap(
      this.temp_weightedSums
    );
  }

  stochasticGradientDescent(
    trainingData: VectorLabeledImage[],
    hyperParams: StochasticGradientDescentHyperParameters,
    testData?: LabeledImage[]
  ): void {
    const {
      batchSize,
      epochs,
      learningRate,
      momentumCoefficient,
    } = hyperParams;
    const trainingDataSize = trainingData.length;

    for (let epoch = 0; epoch < epochs; epoch++) {
      const miniBatches = divideIntoMiniBatches(trainingData, batchSize);
      for (const miniBatch of miniBatches) {
        const { weightGradients, biasGradients } = this.getTotalGradients(
          miniBatch,
          hyperParams.regularizationRate,
          trainingDataSize
        );

        for (let i = 1; i < this.numberOfLayers; i++) {
          weightGradients[i].mutMultiplyScalar(learningRate / miniBatch.length);
          biasGradients[i].mutMultiplyScalar(learningRate / miniBatch.length);

          this.weightVelocities[i]
            .mutMultiplyScalar(momentumCoefficient)
            .mutSubtract(weightGradients[i]);
          this.biasVelocities[i]
            .mutMultiplyScalar(momentumCoefficient)
            .mutSubtract(biasGradients[i]);

          this.weights[i].mutAdd(this.weightVelocities[i]);
          this.biases[i].mutAdd(this.biasVelocities[i]);

          //   this.weights[i].mutSubtract(weightGradients[i]);
          //   this.biases[i].mutSubtract(biasGradients[i]);
        }
      }

      if (testData !== undefined) {
        const accuracyRate = this.test(testData);
        this.log(accuracyRate, epoch);
      }
    }
  }

  private getTotalGradients(
    miniBatch: VectorLabeledImage[],
    regularizationRate: number,
    trainingDataSize: number
  ): Gradients {
    const {
      weightGradients: totalWeightGradients,
      biasGradients: totalBiasGradients,
    } = this.resetTotalGradientTemps();

    for (const image of miniBatch) {
      const { weightGradients, biasGradients } = this.getGradients(
        image,
        regularizationRate,
        trainingDataSize
      );
      for (let i = 1; i < this.numberOfLayers; i++) {
        totalWeightGradients[i].mutAdd(weightGradients[i]);
        totalBiasGradients[i].mutAdd(biasGradients[i]);
      }
    }

    return {
      weightGradients: totalWeightGradients,
      biasGradients: totalBiasGradients,
    };
  }

  private resetTotalGradientTemps(): Gradients {
    const numberOfLayers = this.layerSizes.length;
    const weightGradients = this.temp_totalWeightGradients;
    const biasGradients = this.temp_totalBiasGradients;
    for (let i = 1; i < numberOfLayers; i++) {
      weightGradients[i].setToZero();
      biasGradients[i].setToZero();
    }
    return { weightGradients, biasGradients };
  }

  private getGradients(
    image: VectorLabeledImage,
    regularizationRate: number,
    trainingDataSize: number
  ): Gradients {
    const { numberOfLayers } = this;

    const { weightedSums, activations } = this.performForwardPass(image.inputs);
    const errors = this.temp_errors;
    const weightGradients = this.temp_weightGradients;
    const biasGradients = this.temp_biasGradients;

    const lastLayerError = activations[this.numberOfLayers - 1].subtractInto(
      image.outputs,
      errors[numberOfLayers - 1]
    );

    lastLayerError
      .multiplyInto(
        activations[numberOfLayers - 2].transposeInto(
          this.temp_transposedActivations[numberOfLayers - 2]
        ),
        weightGradients[numberOfLayers - 1]
      )
      .mutAdd(
        this.weights[numberOfLayers - 1].multiplyScalarInto(
          regularizationRate / trainingDataSize,
          this.temp_weightCosts[numberOfLayers - 1]
        )
      );

    lastLayerError.copyInto(biasGradients[numberOfLayers - 1]);

    for (let i = this.numberOfLayers - 2; i >= 1; i--) {
      const error = this.weights[i + 1]
        .transposeInto(this.temp_transposedWeights[i + 1])
        .multiplyInto(errors[i + 1], errors[i])
        .mutHadamard(
          weightedSums[i].applyElementwiseInto(
            sigmaPrime,
            this.temp_sigmaPrimeOfWeightedSums[i]
          )
        );

      error
        .multiplyInto(
          activations[i - 1].transposeInto(
            this.temp_transposedActivations[i - 1]
          ),
          weightGradients[i]
        )
        .mutAdd(
          this.weights[i].multiplyScalarInto(
            regularizationRate / trainingDataSize,
            this.temp_weightCosts[i]
          )
        );

      error.copyInto(biasGradients[i]);
    }

    return { weightGradients, biasGradients };
  }

  performForwardPass(inputs: Matrix): WeightedSumsAndActivations {
    const lastLayer = this.numberOfLayers - 1;

    const weightedSums = this.temp_weightedSums;
    const activations = this.temp_activations;

    activations[0] = inputs;

    for (let outputLayer = 1; outputLayer < lastLayer; outputLayer++) {
      const inputLayer = outputLayer - 1;
      const weightedSum = this.weights[outputLayer]
        .multiplyInto(activations[inputLayer], weightedSums[outputLayer])
        .mutAdd(this.biases[outputLayer]);
      weightedSum.applyElementwiseInto(sigma, activations[outputLayer]);
    }

    {
      const inputLayer = lastLayer - 1;
      const weightedSum = this.weights[lastLayer]
        .multiplyInto(activations[inputLayer], weightedSums[lastLayer])
        .mutAdd(this.biases[lastLayer]);
      const lastActivation = weightedSum
        .subtractScalarInto(weightedSum.maxEntry(), activations[lastLayer])
        .applyElementwiseInto(Math.exp, activations[lastLayer]);
      lastActivation.mutMultiplyScalar(1 / lastActivation.sumOfEntries());
    }

    return { weightedSums, activations };
  }

  test(testData: LabeledImage[]): AccuracyRate {
    let correctClassifications = 0;
    for (const image of testData) {
      const { activations } = this.performForwardPass(image.inputs);
      const prediction = argmax(
        activations[this.numberOfLayers - 1].rowMajorOrderEntries()
      );
      if (prediction === image.label) {
        correctClassifications++;
      }
    }
    return { correct: correctClassifications, total: testData.length };
  }

  getWeights(): DeepReadonly<MatrixMap> {
    return this.weights;
  }

  getBiases(): DeepReadonly<MatrixMap> {
    return this.biases;
  }
}

function sigma(z: number): number {
  return 1 / (1 + Math.exp(-z));
}

function sigmaPrime(z: number): number {
  const sigmaZ = sigma(z);
  return sigmaZ * (1 - sigmaZ);
}

function getZeroMatrixMap(map: MatrixMap): MatrixMap {
  const zeroMatrices: MatrixMap = [];
  for (let i = 1; i < map.length; i++) {
    const matrix = map[i];
    zeroMatrices[i] = Matrix.zeros(matrix.rows, matrix.columns);
  }
  return zeroMatrices;
}
