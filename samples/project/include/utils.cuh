#pragma once
#include <cstdio>

__device__ void print_id() {
    printf("Hello from thread %d inside the include folder!\n", threadIdx.x);
}