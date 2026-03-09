#pragma once
#include "math/constants.cuh"
#include "math/operations.cuh" // Relative to 'core/'

__device__ int wrapped_multiply(int x) {
    return deep_multiply(x, 10); 
}

// A wrapper function that uses one of the constants
__device__ float get_scaled_pi(float scale) {
    return CUDA_PI * scale;
}