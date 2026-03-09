#include <iostream>
#include "core/wrapper.cuh" // Relative to project root

__global__ void chained_kernel(int* out) {
    *out = wrapped_multiply(7); 
}

int main() {
    int *d_out, h_out = 0;
    cudaMalloc(&d_out, sizeof(int));
    chained_kernel<<<1, 1>>>(d_out);
    cudaMemcpy(&h_out, d_out, sizeof(int), cudaMemcpyDeviceToHost);
    std::cout << "Chained result (7 * 10): " << h_out << std::endl;
    cudaFree(d_out);
    return 0;
}