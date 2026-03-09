#include "include/utils.cuh"

__global__ void hello_kernel() {
    print_id();
}

int main() {
    hello_kernel<<<1, 2>>>();
    cudaDeviceSynchronize();
    return 0;
}