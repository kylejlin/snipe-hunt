import { MatrixMap, Network, WeightInitializationMethod } from ".";
import { NetworkV1 } from "./networks/v1";

export interface NetworkFactory {
  fromLayerSizes(
    layerSizes: number[],
    initializationMethod: WeightInitializationMethod
  ): Network;
  fromWeightsAndBiases(weights: MatrixMap, biases: MatrixMap): Network;
}

/**
 * At any given time, the entire web app uses one and only
 * one neural network implementation.
 *
 * All other files will depend on `networkFactory`, which will
 * allow me to easily change which network I'm using by changing
 * only one variable.
 * Otherwise, if I wanted to change from using `NetworkX`
 * to `NetworkY`, I would have to go through the entire src
 * folder and replace every occurrence of `NetworkX` with
 * `NetworkY`.
 */
export const networkFactory: NetworkFactory = {
  fromLayerSizes(
    layerSizes: number[],
    initializationMethod: WeightInitializationMethod
  ): Network {
    return NetworkV1.fromLayerSizes(layerSizes, initializationMethod);
  },
  fromWeightsAndBiases(weights: MatrixMap, biases: MatrixMap): Network {
    return NetworkV1.fromWeightsAndBiases(weights, biases);
  },
};
