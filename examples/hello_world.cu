#include <stdio.h>

// __global__ indicates that this function is a kernel that runs on the GPU
__global__ void cuda_hello() {
    printf("Hello World from GPU!\n");
}

int main() {
    // This is the host (CPU) code

    // The syntax <<<1, 1>>> specifies the execution configuration (1 block, 1 thread)
    cuda_hello<<<1, 1>>>();

    // Synchronizes the host and device, ensuring the printf from the GPU is displayed
    cudaDeviceSynchronize();

    return 0;
}
