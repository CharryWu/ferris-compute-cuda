#include <iostream>
#include "math_constants.cuh"

__global__ void compute_area(float* r, float* area) {
    int idx = threadIdx.x;
    area[idx] = PI * r[idx] * r[idx];
}

int main() {
    float h_r[1] = {5.0f};
    float h_area[1] = {0.0f};
    float *d_r, *d_area;

    cudaMalloc(&d_r, sizeof(float));
    cudaMalloc(&d_area, sizeof(float));

    cudaMemcpy(d_r, h_r, sizeof(float), cudaMemcpyHostToDevice);
    compute_area<<<1, 1>>>(d_r, d_area);
    cudaMemcpy(h_area, d_area, sizeof(float), cudaMemcpyDeviceToHost);

    std::cout << "Area of circle (r=5): " << h_area[0] << std::endl;

    cudaFree(d_r);
    cudaFree(d_area);
    return 0;
}