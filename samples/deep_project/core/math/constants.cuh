#ifndef CONSTANTS_CUH
#define CONSTANTS_CUH

// Use __constant__ or a simple __device__ variable
__device__ const float CUDA_PI = 3.1415926535f;
__device__ const int MAX_THREADS_PER_BLOCK = 1024;

#endif
