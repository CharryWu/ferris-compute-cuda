#include <cuda_runtime.h>
#include <stdio.h>
#include <stdlib.h>
#include <cstdlib> // for rand()

/**
 * Matrix addition kernel
 *
 * @param A - first input matrix
 * @param B - second input matrix
 * @param C - output matrix
 * @param N - number of cols of A, B, and C
 * @param M - number of rows of A, B, and C
 */
__global__ void matrixAdd(float * A, float * B, float * C, unsigned int M, unsigned int N) {
  unsigned int row = threadIdx.y + blockIdx.y * blockDim.y; // NOTE: y is the row index
  unsigned int col = threadIdx.x + blockIdx.x * blockDim.x; // NOTE: x is the col index
  if (row < M && col < N) { // Bounds checking
    unsigned idx = col * M + row; // nx = number of threads per grid row = gridDim.x * blockDim.x
    C[idx] = A[idx] + B[idx];
  }
}

int main() {
  unsigned int M = 1024;
  unsigned int N = 1024;
  unsigned int BLOCK_SIZE = 32;
  unsigned int size = M * N * static_cast<unsigned int>(sizeof(float));

  // Allocate memory on the host
  float *h_A = (float*) malloc(size);
  float *h_B = (float*) malloc(size);
  float *h_C = (float*) malloc(size);

  // Initialize matrices A and B
  // Randomly initialize matrices A and B
  for (unsigned int i = 0; i < M * N; i++) {
    // NOTE: rand() returns an int (0 to RAND_MAX), also RAND_MAX is an int constant
    // Without casting to float, '/' performs integer division which almost always results in 0 
    h_A[i] = static_cast<float>(rand()) / static_cast<float>(RAND_MAX);
    h_B[i] = static_cast<float>(rand()) / static_cast<float>(RAND_MAX);
  }

  // Allocate memory on the device
  float *d_A, *d_B, *d_C;
  cudaMalloc((void**)&d_A, size);
  cudaMalloc((void**)&d_B, size);
  cudaMalloc((void**)&d_C, size);

  // Copy matrices A and B from the host to the device
  cudaMemcpy(d_A, h_A, size, cudaMemcpyHostToDevice);
  cudaMemcpy(d_B, h_B, size, cudaMemcpyHostToDevice);

  // Launch the matrixAdd kernel
  dim3 dimBlock(BLOCK_SIZE, BLOCK_SIZE);
  dim3 dimGrid((N + dimBlock.x - 1) / dimBlock.x, (M + dimBlock.y - 1) / dimBlock.y);
  matrixAdd<<<dimGrid, dimBlock>>>(d_A, d_B, d_C, M, N);

  // Copy the result from the device to the host
  cudaMemcpy(h_C, d_C, size, cudaMemcpyDeviceToHost);

  // Check a few results
  printf("A[i] + B[i] = c[i]  \n");
  printf("%f   + %f   = %f   \n", h_A[0],h_B[0],h_C[0]);
  printf("%f   + %f   = %f   \n", h_A[1],h_B[1],h_C[1]);

  // Free the memory on the device
  cudaFree(d_A);
  cudaFree(d_B);
  cudaFree(d_C);
  // Free the memory on the host
  free(h_A);
  free(h_B);
  free(h_C);

  return 0;
}