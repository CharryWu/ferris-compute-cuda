# 🦀 rust-cuda Deep Dive: Compilation Pipeline, Runtime, and cuDNN

> A walkthrough of the [rust-cuda](https://github.com/Rust-GPU/Rust-CUDA) repository for newcomers.  
> Covers: Rust → LLVM IR → PTX compilation, the runtime host API, and the cuDNN backend API with its operation graph.

---

## 📁 Repository Overview

```
rust-cuda/
├── crates/
│   │
│   │  ── COMPILATION PIPELINE (GPU kernel dev) ──────────────────────────
│   ├── cuda_builder/          🔨 Build-script helper: invokes rustc + nvvm
│   ├── rustc_codegen_nvvm/    🧠 Custom rustc codegen backend (Rust → LLVM IR → NVVM bitcode)
│   ├── nvvm/                  🔗 Safe Rust wrappers for libnvvm (NVVM IR → PTX)
│   ├── cuda_std/              📦 GPU-side std (no_std, #[kernel] macros, address spaces)
│   ├── cuda_std_macros/       🪄 Proc-macros for cuda_std
│   ├── ptx/                   📄 PTX utilities
│   ├── ptx_compiler/          ⚙️  Bindings to nvPTXCompiler (PTX → cubin at runtime)
│   │
│   │  ── HOST RUNTIME API ─────────────────────────────────────────────────
│   ├── cust/                  🖥️  High-level safe host API (context, memory, streams, modules)
│   ├── cust_core/             🔷 DeviceCopy trait (shared between host & device)
│   ├── cust_raw/              ⚡ Raw FFI bindings (bindgen) to CUDA driver, runtime, cuBLAS, libnvvm
│   ├── cust_derive/           🪄 #[derive(DeviceCopy)] proc-macro
│   │
│   │  ── DEEP LEARNING LIBRARY INTERFACE ───────────────────────────────────
│   ├── cudnn/                 🧮 High-level safe cuDNN API (operation graph, execution plans)
│   ├── cudnn-sys/             ⚡ Raw FFI bindings (bindgen) to libcudnn
│   │
│   │  ── OTHER ───────────────────────────────────────────────────────────
│   ├── blastoff/              🚀 High-level cuBLAS wrapper
│   ├── optix/                 🌅 NVIDIA OptiX ray-tracing API
│   ├── gpu_rand/              🎲 GPU-side random number generation
│   └── ...
│
├── examples/
│   ├── vecadd/                ➕ Classic vector-add kernel demo
│   ├── gemm/                  🔢 Matrix multiply using cuBLAS
│   └── ...
├── rust-toolchain.toml        🔧 Pins a specific nightly rustc (required for custom codegen)
└── .cargo/config.toml         ⚙️  Cargo config (codegen backend plumbing)
```

---

## 🔄 Part 1 — The Compilation Pipeline: Rust → LLVM IR → PTX

This is the core innovation of the repo. The goal is to write GPU kernels in pure Rust and have them compiled to PTX (Parallel Thread Execution) assembly that NVIDIA GPUs can run.

### 1.1 The Big Picture

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │                  COMPILE TIME  (build.rs on host)                   │
  │                                                                     │
  │  Your GPU crate (no_std Rust)                                       │
  │       │                                                             │
  │       │ cargo build --target nvptx64-nvidia-cuda                   │
  │       ▼                                                            │
  │  ┌──────────────────────────────────────────────────────────┐      │
  │  │           rustc  (custom codegen backend)                │      │
  │  │                                                          │      │
  │  │  Rust HIR/MIR  ──►  rustc_codegen_nvvm  ──►  LLVM IR     │      │
  │  │                         │                    (bitcode)   │      │
  │  │                         │ via LLVM C API                 │      │
  │  └──────────────────────────────────────────────────────────┘      │
  │       │                                                            │
  │       │  LLVM bitcode (.bc) per CGU (codegen unit)                 │
  │       ▼                                                            │
  │  ┌─────────────────────────────────────────────────────────────┐   │
  │  │              nvvm crate  (libnvvm FFI)                      │   │
  │  │                                                             │   │
  │  │  Merge all .bc  ──►  Internalize  ──►  DCE  ──►  libnvvm    │   │
  │  │  + libdevice.bc (math intrinsics)                           │   │
  │  │  + libintrinsics.bc (custom helpers)                        │   │
  │  └─────────────────────────────────────────────────────────────┘   │
  │       │                                                            │
  │       │  PTX text file  (e.g. my_kernel.ptx)                       │
  │       ▼                                                            │
  │  cuda_builder::CudaBuilder::build() returns PathBuf to .ptx        │
  └──────────────────────────────────────────────────────�┬────────────┘
                                                          │ include_str!("my_kernel.ptx")
                                                          ▼
  ┌─────────────────────────────────────────────────────────────────────┐
  │                  RUNTIME  (host executable)                         │
  │                                                                     │
  │  cust::Module::from_ptx(ptx_string, &[])                            │
  │       │                                                             │
  │       │  CUDA Driver API: cuModuleLoadDataEx()                      │
  │       │  (JIT compiles PTX → cubin for the current GPU)             │
  │       ▼                                                             │
  │  module.get_function("my_kernel_name")  ──►  Function handle        │
  │  launch!(function<<<grid, block, shared_mem, stream>>>(args))       │
  │       │                                                             │
  │       ▼                                                             │
  │  🖥️  GPU executes cubin                                             │
  └─────────────────────────────────────────────────────────────────────┘
```

---

### 1.2 Crate: `rustc_codegen_nvvm` — The Custom Rustc Backend

**Location:** `crates/rustc_codegen_nvvm/`

This is the heart of the system. It is a `rustc` *codegen backend* — a dynamic library (`.so`/`.dll`) that replaces LLVM's standard code emission. Instead of emitting native machine code, it emits **LLVM bitcode** in a form that NVIDIA's `libnvvm` can accept.

#### How a custom codegen backend works

```
  rustc internals
  ┌──────────────────────────────────────────────────────────────────┐
  │   Parser → HIR → typechecking → MIR → ... → codegen trait API   │
  │                                                    │             │
  │                          implements CodegenBackend │             │
  │                                    ▼               │             │
  │                    rustc_codegen_nvvm.so  ◄────────┘             │
  │                    (this crate, loaded dynamically)              │
  └──────────────────────────────────────────────────────────────────┘
```

#### Key source files

```
rustc_codegen_nvvm/src/
├── lib.rs              — NvvmCodegenBackend: main entry point, implements CodegenBackend
├── back.rs             — compile_codegen_unit(): MIR → LLVM module → bitcode
├── nvvm.rs             — codegen_bitcode_modules(): merge bitcode, feed to libnvvm → PTX
├── builder.rs          — Builder: implements BuilderMethods (generates LLVM instructions)
├── context.rs          — CodegenCx: per-CGU codegen context, holds LLVM context/module
├── llvm.rs             — Raw FFI to LLVM C API (LLVMBuildAdd, LLVMInt32Type, etc.)
├── ptxgen.rs           — PTX-specific passes (replace illegal types, etc.)
├── int_replace.rs      — Emulate i128 (not natively supported by nvvm)
├── intrinsic.rs        — Map Rust intrinsics → NVVM intrinsics (threadIdx, blockIdx, etc.)
├── override_fns.rs     — Override libm functions with libdevice equivalents
├── attributes.rs       — Apply NVVM-specific LLVM attributes (kernel metadata, etc.)
├── debug_info/         — DWARF debug info generation
└── rustc_llvm_wrapper/ — Thin C++ shims over LLVM API not exposed via C
    ├── RustWrapper.cpp
    └── PassWrapper.cpp
```

#### Step-by-step in `back.rs → compile_codegen_unit()`

```
  TyCtxt (Rust compiler context)
      │
      ▼
  CodegenCx::new()          — create LLVM Context + Module for this CGU
      │
      ▼
  mono_items (monomorphized Rust items)
      │
      ├──► mono_item.predefine()   — declare functions/statics in LLVM module (no bodies yet)
      │
      └──► mono_item.define()      — emit LLVM IR instructions for each item body
               │
               ▼
           Builder (wraps LLVMBuilderRef)
           • Every Rust operation maps to LLVM instructions
           • Special handling: no i128, no f16/f128, PTX address spaces
      │
      ▼
  ModuleCodegen<LlvmMod>    — LLVM module object (in-memory bitcode)
      │
      ▼
  ModuleBuffer::new()       — serialize to LLVM bitcode bytes (Vec<u8>)
      │
      ▼
  write to temp .bc file    — one file per CGU
```

#### Step-by-step in `nvvm.rs → codegen_bitcode_modules()`

```
  all .bc files (one per CGU)
      │
      ▼
  merge_llvm_modules()
  • LLVMRustParseBitcodeForLTO() — parse each .bc
  • LLVMLinkModules2()            — link into single "merged_modules" module
      │
      ▼
  internalize_pass()
  • Read nvvm.annotations metadata — find all #[kernel] functions
  • Non-kernel, non-extern functions → InternalLinkage (allows DCE)
      │
      ▼
  dce_pass()
  • LLVMAddGlobalDCEPass()        — remove dead functions and globals
      │
      ▼
  Annotate NVVM IR version metadata  (nvvmir.version)
      │
      ▼
  NvvmProgram::new()
  prog.add_module(merged_bitcode)
  prog.add_lazy_module(libdevice.bc)     — NVIDIA math library (~2000 GPU math fns)
  prog.add_lazy_module(libintrinsics.bc) — custom helpers (warp ops, etc.)
      │
      ▼
  prog.verify()     — catch malformed IR early (libnvvm can segfault on bad IR!)
      │
      ▼
  prog.compile(&nvvm_options)
  • Calls nvvmCompileProgram() from libnvvm.so
  • libnvvm optimizes LLVM IR for the target arch
  • libnvvm emits PTX text
      │
      ▼
  Vec<u8>  (PTX bytes, written to .ptx file)
```

#### LLVM C++ wrappers

The file `rustc_llvm_wrapper/RustWrapper.cpp` adds thin C wrappers around LLVM APIs that don't have a C interface. For example:
- `LLVMRustParseBitcodeForLTO` — parse bitcode with LTO context
- `LLVMRustSetLinkage` / `LLVMRustSetVisibility` — set symbol visibility
- `LLVMRustPrintModule` — dump module to `.ll` text for debugging
- `LLVMRustCreateTargetMachine` — create NVPTX target machine

---

### 1.3 Crate: `cuda_builder` — The Build Script Utility

**Location:** `crates/cuda_builder/`

Users call this from their GPU crate's host `build.rs` script.

```rust
// In host_crate/build.rs
use cuda_builder::CudaBuilder;

fn main() {
    CudaBuilder::new("../gpu_crate")
        .arch(NvvmArch::Compute75)   // RTX Turing
        .release(true)
        .copy_to("ptx/gpu_kernels.ptx")
        .build()
        .unwrap();
}
```

#### What `CudaBuilder::build()` does

```
  CudaBuilder::build()
      │
      ▼
  find_rustc_codegen_nvvm()
  • Search DEP_RUSTC_CODEGEN_NVVM_OUT_DIR
  • Search workspace target/ dirs  (debug, release, deps)
  • If not found: auto-build via   cargo build -p rustc_codegen_nvvm
      │
      ▼
  Build RUSTFLAGS:
  ┌────────────────────────────────────────────────────────────────────┐
  │  -Zcodegen-backend=<path/to/rustc_codegen_nvvm.so>                │
  │  -Zunstable-options                                                │
  │  -Zcrate-attr=no_std          (GPU crates must be no_std)         │
  │  -Cpanic=immediate-abort      (no unwinding on GPU)               │
  │  -Cllvm-args="-arch=compute_75 -opt=3 ..."  (nvvm options)        │
  └────────────────────────────────────────────────────────────────────┘
      │
      ▼
  cargo build --lib
              --target nvptx64-nvidia-cuda   ← the magic target triple
              -Zbuild-std=core,alloc         ← rebuild std for GPU
              --message-format=json-render-diagnostics
      │
      ▼
  Parse JSON output for compiler-artifact with .ptx extension
      │
      ▼
  Return PathBuf to the .ptx file
```

#### `NvvmArch` — Target Architecture Enum

The `nvvm` crate defines `NvvmArch`, an enum representing CUDA compute capabilities. It controls which PTX instructions are available.

```
  NvvmArch::Compute75   →  "-arch=compute_75"   (Turing: RTX 20xx, T4)
  NvvmArch::Compute80   →  "-arch=compute_80"   (Ampere: A100)
  NvvmArch::Compute90   →  "-arch=compute_90"   (Hopper: H100)
  NvvmArch::Compute90a  →  "-arch=compute_90a"  (Hopper arch-specific features)
  NvvmArch::Compute100f →  "-arch=compute_100f" (Blackwell family features)
  ...

  Feature hierarchy (each arch is a superset of lower archs):

  compute_50  ⊂  compute_52  ⊂  ...  ⊂  compute_75  ⊂  compute_80  ⊂  ...
                                             ▲
                                         default
```

Suffix meanings (new in CUDA 12.9):
- **No suffix** (e.g. `compute_90`) — forward-compatible baseline
- **`a` suffix** (e.g. `compute_90a`) — architecture-specific (e.g. Tensor Core ops, only runs on that exact GPU)
- **`f` suffix** (e.g. `compute_100f`) — family-specific, works within same major version

---

### 1.4 Crate: `nvvm` — Safe libnvvm Bindings

**Location:** `crates/nvvm/`

Thin, safe wrappers around the raw `cust_raw::nvvm_sys` bindings.

Key types:

```
  NvvmProgram
  ├── ::new()                     → nvvmCreateProgram()
  ├── .add_module(bc, name)       → nvvmAddModuleToProgram()       (regular)
  ├── .add_lazy_module(bc, name)  → nvvmLazyAddModuleToProgram()   (only-if-used)
  ├── .verify()                   → nvvmVerifyProgram()
  ├── .compile(&[NvvmOption])     → nvvmCompileProgram() → PTX bytes
  └── .compiler_log()             → nvvmGetProgramLog()  (error details)

  NvvmOption  (maps to libnvvm CLI flags)
  ├── Arch(NvvmArch)              → "-arch=compute_XX"
  ├── NoOpts                      → "-opt=0"
  ├── Ftz                         → "-ftz=1"  (flush denormals)
  ├── FastSqrt                    → "-prec-sqrt=0"
  ├── FastDiv                     → "-prec-div=0"
  └── NoFmaContraction            → "-fma=0"
```

---

### 1.5 Crate: `cust_raw` — Raw FFI Bindings via bindgen

**Location:** `crates/cust_raw/`

Generated at build time by `bindgen`, reading CUDA header files.

```
  cust_raw/
  ├── build/
  │   ├── main.rs               — build script: finds CUDA SDK, runs bindgen
  │   ├── cuda_sdk.rs           — CudaSdk struct: SDK auto-detection logic
  │   │   • reads env: CUDA_PATH | CUDA_ROOT | CUDA_TOOLKIT_ROOT_DIR
  │   │   • falls back to: /usr/local/cuda, /usr/cuda, etc.
  │   │   • finds libdevice.10.bc at <cuda_root>/nvvm/libdevice/
  │   ├── callbacks.rs          — bindgen callbacks (rename cu* → Cu*, etc.)
  │   ├── driver_wrapper.h      → includes <cuda.h>         → driver_sys.rs
  │   ├── runtime_wrapper.h     → includes <cuda_runtime.h> → runtime_sys.rs
  │   ├── nvvm_wrapper.h        → includes <nvvm.h>         → nvvm_sys.rs
  │   ├── cublas_wrapper.h      → includes <cublas_v2.h>    → cublas_sys.rs
  │   └── nvptx_compiler_wrapper.h → <nvPTXCompiler.h>     → nvptx_compiler_sys.rs
  └── src/
      ├── lib.rs
      ├── driver_sys.rs         — Generated: CUcontext, CUmodule, cuLaunchKernel, ...
      ├── runtime_sys.rs        — Generated: cudaMalloc, cudaMemcpy, ...
      ├── nvvm_sys.rs           — Generated: nvvmProgram, nvvmCompileProgram, ...
      │                           Also: LIBDEVICE_BITCODE (embedded via include_bytes!)
      ├── cublas_sys.rs         — Generated: cublasHandle_t, cublasSgemm, ...
      └── nvptx_compiler_sys.rs — Generated: nvPTXCompilerHandle, nvPTXCompilerCompile, ...
```

**CUDA SDK auto-detection** (in `cuda_sdk.rs`):
```
  Priority order:
  1. Env var: CUDA_PATH  or  CUDA_ROOT  or  CUDA_TOOLKIT_ROOT_DIR
  2. CUDA_LIBRARY_PATH env var hints
  3. Default platform locations:
     Linux:   /usr/local/cuda, /usr/cuda, /opt/cuda
     Windows: C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v*
     macOS:   (limited; CUDA dropped Mac support)
```

---

## 🖥️ Part 2 — The Host Runtime API: `cust`, `cust_core`

Once the PTX file is compiled, the host program (normal Rust, not GPU code) uses `cust` to run kernels.

### 2.1 Crate: `cust` — High-Level CUDA Host API

**Location:** `crates/cust/`

```
  cust/src/
  ├── context/          — CudaContext (primary / legacy), device context management
  ├── device.rs         — Device query: name, memory, compute capability
  ├── module.rs         — Module: load PTX/cubin/fatbin, get_function(), get_global()
  ├── function.rs       — Function: kernel handle, occupancy queries
  ├── stream.rs         — CudaStream: async execution ordering
  ├── event.rs          — CudaEvent: timing, synchronization
  ├── memory/
  │   ├── device/
  │   │   ├── device_box.rs      — DeviceBox<T>: single value on GPU
  │   │   ├── device_buffer.rs   — DeviceBuffer<T>: array on GPU
  │   │   └── device_slice.rs    — DeviceSlice<T>: slice view of GPU memory
  │   ├── locked.rs     — LockedBuffer: page-locked host memory (faster DMA)
  │   ├── unified.rs    — UnifiedBuffer: managed memory (auto-migrate host↔device)
  │   └── pointer.rs    — DevicePointer<T>: typed raw GPU pointer
  ├── graph.rs          — CUDA Graph API: capture streams into replayable DAGs
  ├── link.rs           — Linker: PTX → cubin JIT linking
  └── compile.rs        — Inline PTX compilation helpers
```

#### Loading a module and launching a kernel

```
  cust::quick_init()?          — init CUDA, create context
       │
       ▼
  Module::from_ptx(ptx_str, &[])
       │  cuModuleLoadDataEx()   ← CUDA driver JIT-compiles PTX → cubin
       ▼
  module.get_function("kernel_name")
       │  cuModuleGetFunction()
       ▼
  Function handle
       │
       ▼
  launch!(func<<<grid, block, shared, stream>>>(arg1, arg2, ...))
       │  cuLaunchKernel()
       ▼
  stream.synchronize()          — wait for kernel to finish
```

#### Memory type hierarchy

```
  ┌─────────────────────────────────────────────────────────────────┐
  │  Host (CPU) Memory                                              │
  │  ┌──────────────┐   ┌──────────────────┐                       │
  │  │  Vec<T>      │   │  LockedBuffer<T>  │  (page-locked, fast  │
  │  │  (normal)    │   │  (DMA-friendly)   │   for transfers)     │
  │  └──────┬───────┘   └────────┬─────────┘                       │
  └─────────┼───────────────────┼─────────────────────────────────┘
            │ copy_from / copy_to │
  ┌─────────▼───────────────────▼─────────────────────────────────┐
  │  Device (GPU) Memory                                           │
  │  ┌─────────────────┐  ┌────────────────────┐                  │
  │  │  DeviceBuffer<T> │  │  UnifiedBuffer<T>   │  (managed;     │
  │  │  DeviceBox<T>    │  │                    │   auto-migrate) │
  │  │  DeviceSlice<T>  │  └────────────────────┘                  │
  │  └─────────────────┘                                           │
  └────────────────────────────────────────────────────────────────┘
```

### 2.2 Crate: `cust_core` — The `DeviceCopy` Safety Trait

**Location:** `crates/cust_core/`

A tiny `no_std` crate that defines a single critical safety marker trait:

```rust
pub unsafe trait DeviceCopy: Copy {}
```

**Purpose:** Types that implement `DeviceCopy` can be safely bit-copied to the GPU. This rules out types like `Vec<T>`, `Box<T>`, `String`, or any type with a raw pointer to heap memory (which would be invalid on the device).

```
  DeviceCopy is NOT the same as Copy!
  ┌────────────────────────────────────────────────────────────────┐
  │  Copy:        can be implicitly duplicated by the compiler     │
  │  DeviceCopy:  safe to bitwise-copy TO the GPU device          │
  │               (no host-only pointers, no destructors)          │
  └────────────────────────────────────────────────────────────────┘

  Blanket impls:
  u8, u16, u32, u64, u128, usize, i8, ... f32, f64, bool, char ✅
  *const T, *mut T  ✅ (raw pointers — user's responsibility)
  [T; N] where T: DeviceCopy              ✅
  (A, B, ...) where all: DeviceCopy       ✅ (up to 8-tuples)
  Option<T>, Result<L,R>                  ✅ (if T/L/R: DeviceCopy)
  Vec<T>, String, Box<T>                  ❌ (contain heap pointers)
```

The `cust_core` crate is intentionally `no_std` so that GPU-side code (which is `no_std`) can use the same trait definition as the host side.

---

## 🧮 Part 3 — The cuDNN Interface: `cudnn-sys` and `cudnn`

cuDNN is NVIDIA's GPU-accelerated deep learning primitive library (convolutions, RNNs, attention, etc.). The repo is building a safe Rust wrapper.

### 3.1 Crate: `cudnn-sys` — Raw FFI Bindings

**Location:** `crates/cudnn-sys/`

```
  cudnn-sys/
  ├── build/
  │   ├── main.rs           — build script
  │   ├── cudnn_sdk.rs      — CudnnSdk: auto-detect cuDNN installation
  │   └── wrapper.h         — #include <cudnn.h>   (all cuDNN headers)
  └── src/
      └── lib.rs            — re-exports generated cudnn_sys.rs (from OUT_DIR)
```

**cuDNN SDK auto-detection** (in `cudnn_sdk.rs`):
```
  Priority:
  1. Env var: CUDNN_INCLUDE_DIR
  2. Default paths (Linux):
     /usr/include
     /usr/local/include
     /usr/include/x86_64-linux-gnu     ← CUDA 13 arch-specific
     /usr/include/aarch64-linux-gnu
  3. Default paths (Windows):
     C:\Program Files\NVIDIA\CUDNN\v9.x\include
     C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v*\include

  Validates by checking for: cudnn.h AND cudnn_version.h
  Parses version from: #define CUDNN_MAJOR / CUDNN_MINOR / CUDNN_PATCHLEVEL
```

The `cudnn-sys` build script also receives **transitive metadata** from `cust_raw` via `DEP_CUDA_INCLUDES`, so it can pass CUDA include directories to `bindgen` when parsing cuDNN headers (which themselves include CUDA headers).

```
  cust_raw (build.rs)
      │  cargo::metadata=includes=<cuda_include_paths>
      ▼
  cudnn-sys (build.rs)
      │  env: DEP_CUDA_INCLUDES=<cuda_include_paths>
      │  Used as clang_args to bindgen
      ▼
  Generated: cudnn_sys.rs (all cudnn* types and functions)
  Link:      libcudnn.so (dynamically linked)
```

### 3.2 Crate: `cudnn` — High-Level Safe API

**Location:** `crates/cudnn/`

This wraps the cuDNN C API in safe Rust. The API surface is divided into two layers:

```
  cudnn/src/
  ├── context.rs                 — CudnnContext (cudnnHandle_t wrapper)
  ├── data_type.rs               — DataType enum (Float, Double, Half, Int8, ...)
  ├── tensor/                    — TensorDescriptor (shape, strides, format)
  ├── convolution/               — ConvolutionDescriptor, FilterDescriptor, algos
  ├── activation/                — ActivationDescriptor (ReLU, Sigmoid, ...)
  ├── pooling/                   — PoolingDescriptor (max, avg)
  ├── dropout/                   — DropoutDescriptor
  ├── rnn/                       — RNN: GRU, LSTM, RNN descriptors + layouts
  ├── softmax/                   — SoftmaxAlgo, SoftmaxMode
  ├── op/                        — Element-wise tensor ops
  ├── reduction/                 — ReductionDescriptor
  ├── attention/                 — AttentionDescriptor (multi-head attention)
  └── backend/                   ← The new Backend API (cuDNN v8+)
      ├── descriptor.rs          — Descriptor (generic backend descriptor handle)
      ├── tensor.rs              — Tensor (virtual tensor for op graph)
      ├── operation.rs           — Operation enum (ConvFwd, MatMul, Pointwise, ...)
      ├── graph.rs               — Graph / GraphBuilder (operation graph)
      ├── engine.rs              — Engine (algorithm handle)
      ├── engine_cfg.rs          — EngineCfg (engine + knob configuration)
      ├── engine_heuristic.rs    — EngineHeuristic (auto-select best engine)
      ├── execution_plan.rs      — ExecutionPlan (ready-to-run plan)
      ├── conv_fwd.rs            — ConvFwd operation builder
      ├── conv_bwd_data.rs       — ConvBwdData operation builder
      ├── conv_bwd_filter.rs     — ConvBwdFilter operation builder
      ├── matmul.rs              — MatMul operation builder
      ├── pointwise.rs           — Pointwise operation builder
      ├── reduction.rs           — Reduction operation builder
      └── mod.rs
```

---

### 3.3 The cuDNN Operation Graph (Backend API)

The most architecturally interesting part. cuDNN v8+ introduced a **graph-based API** where you compose computational operations into a DAG (directed acyclic graph), and cuDNN selects an optimized fused kernel to execute the whole graph in one pass.

#### Mental model

```
  Traditional cuDNN (v7 style, per-operation):
  ┌─────────┐    ┌──────────┐    ┌───────────┐
  │  Conv   │ →  │  Bias    │ →  │  ReLU     │   (3 separate kernel launches)
  └─────────┘    └──────────┘    └───────────┘

  cuDNN v8 Backend API (graph-based, fused):
  ┌────────────────────────────────────────────────────────────┐
  │                   OperationGraph                           │
  │                                                            │
  │  Tensor(x) ─►┌──────────┐                                 │
  │               │  ConvFwd │──► Tensor(conv_out)             │
  │  Tensor(w) ─►└──────────┘         │                       │
  │                                   ▼                        │
  │  Tensor(b) ─►┌──────────┐   Tensor(bias_out)              │
  │               │ Pointwise│◄──────────────────              │
  │               │  (Add)   │                                 │
  │               └──────────┘──► Tensor(act_in)              │
  │                                   │                        │
  │               ┌──────────┐        ▼                        │
  │               │ Pointwise│◄── Tensor(act_in)              │
  │               │ (ReLU)   │──► Tensor(y)                   │
  │               └──────────┘                                 │
  │                                                            │
  │   (one fused kernel! saves memory bandwidth & latency)     │
  └────────────────────────────────────────────────────────────┘
```

#### Data structures

```
  Descriptor  (crates/cudnn/src/backend/descriptor.rs)
  ┌────────────────────────────────────────────────────────────┐
  │  Descriptor(Rc<Inner>)                                     │
  │    Inner { raw: cudnnBackendDescriptor_t }                 │
  │                                                            │
  │  Methods:                                                  │
  │  • new(type)         → cudnnBackendCreateDescriptor()      │
  │  • set_attribute()   → cudnnBackendSetAttribute()          │
  │  • get_attribute()   → cudnnBackendGetAttribute()          │
  │  • finalize()        → cudnnBackendFinalize()              │
  │  • drop()            → cudnnBackendDestroyDescriptor()     │
  └────────────────────────────────────────────────────────────┘

  Tensor  (backend/tensor.rs)
  ┌─────────────────────────────────────────────────────┐
  │  Tensor { descriptor: Descriptor }                  │
  │  • Represents a VIRTUAL tensor in the graph         │
  │  • Has: unique_id, data_type, dimensions, strides   │
  │  • Not bound to actual GPU memory yet               │
  └─────────────────────────────────────────────────────┘

  Operation  (backend/operation.rs)
  ┌─────────────────────────────────────────────────────────────────┐
  │  enum Operation {                                               │
  │    ConvFwd    { raw: Descriptor, cfg: ConvCfg,                 │
  │                 alpha, beta, w: Tensor, x: Tensor, y: Tensor } │
  │    ConvBwdData { ... }                                          │
  │    ConvBwdFilter { ... }                                        │
  │    MatMul     { raw: Descriptor, cfg: MatMulCfg,               │
  │                 a: Tensor, b: Tensor, c: Tensor }               │
  │    Pointwise  { raw: Descriptor, cfg: PointwiseCfg,            │
  │                 x, y: Tensor, b: Option<Tensor>, ... }         │
  │    Reduction  { raw: Descriptor, cfg: ReductionCfg,            │
  │                 x: Tensor, y: Tensor }                         │
  │  }                                                              │
  └─────────────────────────────────────────────────────────────────┘

  Graph  (backend/graph.rs)
  ┌────────────────────────────────────────────────────────────────┐
  │  struct Graph {                                                │
  │    descriptor: Descriptor,   // CUDNN_BACKEND_OPERATIONGRAPH  │
  │    context: CudnnContext,     // cudnnHandle_t                 │
  │    operations: Vec<Operation>                                  │
  │  }                                                             │
  │                                                                │
  │  GraphBuilder                                                  │
  │  • .set_context(ctx)                                           │
  │  • .set_operations(ops)                                        │
  │  • .build() → Graph    [calls cudnnBackendFinalize]            │
  └────────────────────────────────────────────────────────────────┘

  Engine  (backend/engine.rs)
  ┌────────────────────────────────────────────────────────────────┐
  │  A specific algorithm implementation for executing a graph.    │
  │  cuDNN may provide multiple engines per graph with different   │
  │  performance/accuracy/workspace tradeoffs.                     │
  └────────────────────────────────────────────────────────────────┘

  EngineCfg  (backend/engine_cfg.rs)
  ┌────────────────────────────────────────────────────────────────┐
  │  Engine + knob settings (tile sizes, split-k, etc.)           │
  └────────────────────────────────────────────────────────────────┘

  ExecutionPlan  (backend/execution_plan.rs)
  ┌────────────────────────────────────────────────────────────────┐
  │  struct ExecutionPlan {                                        │
  │    descriptor: Descriptor, // CUDNN_BACKEND_EXECUTION_PLAN    │
  │    engine_cfg: EngineCfg                                       │
  │  }                                                             │
  │  ExecutionPlanBuilder                                          │
  │  • .set_engine_cfg(cfg)                                        │
  │  • .build() → ExecutionPlan                                    │
  └────────────────────────────────────────────────────────────────┘
```

#### Full Backend API workflow (builder pattern)

```
  Step 1: Define virtual tensors
  ┌─────────────────────────────────────────┐
  │  let x = TensorBuilder::new()           │
  │    .set_data_type(DataType::Float)      │
  │    .set_dimensions(&[N, C, H, W])       │
  │    .set_unique_id(1)                    │
  │    .build()?;                           │
  └─────────────────────────────────────────┘

  Step 2: Define operations (nodes in the graph)
  ┌─────────────────────────────────────────┐
  │  let conv = ConvFwdBuilder::new()       │
  │    .set_conv_cfg(conv_cfg)              │
  │    .set_x(x).set_w(w).set_y(y)         │
  │    .build()?;                           │
  └─────────────────────────────────────────┘

  Step 3: Build the graph
  ┌─────────────────────────────────────────┐
  │  let graph = GraphBuilder::new()        │
  │    .set_context(ctx)                    │
  │    .set_operations(vec![conv, bias, act])│
  │    .build()?;   ← cudnnBackendFinalize  │
  └─────────────────────────────────────────┘

  Step 4: Select an engine
  ┌─────────────────────────────────────────┐
  │  let heuristic = EngineHeuristicBuilder │
  │    .set_graph(&graph)                   │
  │    .set_mode(HeuristicMode::A)          │
  │    .build()?;                           │
  │  let engine_cfgs = heuristic.get_engine_configs()?;│
  └─────────────────────────────────────────┘

  Step 5: Build execution plan
  ┌─────────────────────────────────────────┐
  │  let plan = ExecutionPlanBuilder::new() │
  │    .set_engine_cfg(engine_cfgs[0])      │
  │    .build()?;                           │
  └─────────────────────────────────────────┘

  Step 6: Execute
  ┌─────────────────────────────────────────┐
  │  ctx.execute(&plan, &variant_pack)?;    │
  │  (variant_pack binds virtual tensors    │
  │   to actual GPU device pointers)        │
  └─────────────────────────────────────────┘
```

#### Internal descriptor lifecycle

Every cuDNN backend object follows the same lifecycle, managed by the `Descriptor` struct:

```
  cudnnBackendCreateDescriptor(type)     // allocate
       │
       ▼
  cudnnBackendSetAttribute(...)          // configure (multiple calls)
       │
       ▼
  cudnnBackendFinalize()                 // validate + freeze
       │
       ▼
  cudnnBackendGetAttribute(...)          // query after finalization (optional)
       │
       ▼
  cudnnBackendDestroyDescriptor()        // Rust Drop impl, automatic
```

The `Descriptor` uses `Rc<Inner>` for cheap cloning (reference counting), allowing tensors and operations to share descriptors without copies.

---

## 📦 Part 4 — External Dependencies

### 4.1 LLVM

| What | Where | How |
|---|---|---|
| LLVM C API (`LLVMBuildAdd`, `LLVMInt32Type`, etc.) | `rustc_codegen_nvvm/src/llvm.rs` | FFI declarations; linked against rustc's bundled LLVM |
| LLVM C++ shims | `rustc_codegen_nvvm/rustc_llvm_wrapper/RustWrapper.cpp` | Compiled as a C++ static lib, linked into the codegen dylib |
| LLVM pass manager | `rustc_codegen_nvvm/src/back.rs` → `optimize()` | `LLVMCreatePassManager`, `LLVMRunPassManager` |
| LLVM bitcode parsing | `rustc_codegen_nvvm/src/nvvm.rs` | `LLVMRustParseBitcodeForLTO` |
| LLVM module linking | `rustc_codegen_nvvm/src/nvvm.rs` | `LLVMLinkModules2` |
| LLVM DCE pass | `rustc_codegen_nvvm/src/nvvm.rs` | `LLVMAddGlobalDCEPass` |
| LLVM IR emission | `rustc_codegen_nvvm/src/back.rs` | `LLVMRustPrintModule` |

> **Note:** The codegen uses rustc's own bundled LLVM (pinned nightly), NOT a system LLVM. The `rust-toolchain.toml` pins the exact nightly to ensure LLVM compatibility.

### 4.2 CUDA Toolkit

| Component | Where invoked | Purpose |
|---|---|---|
| `libcuda.so` (CUDA Driver API) | `cust_raw/build/main.rs`, `cust/src/module.rs`, `cust/src/memory/` | Load modules, launch kernels, GPU memory allocation |
| `libcudart.so` (CUDA Runtime API) | `cust_raw` with `runtime` feature | Higher-level runtime utilities |
| `libnvvm.so` | `nvvm/src/lib.rs`, `cust_raw/build/main.rs` | Compile LLVM IR → PTX |
| `libdevice.10.bc` | `cust_raw/build/main.rs` → embedded in `nvvm_sys.rs` | GPU math library (sin, cos, exp, etc.) |
| `nvPTXCompiler` | `cust_raw` with `nvptx-compiler` feature | PTX → cubin compilation at runtime |
| `libcublas.so` | `cust_raw` with `cublas` feature | Dense linear algebra |
| `libcudnn.so` | `cudnn-sys/build/main.rs` | Deep learning primitives |
| CUDA headers (`cuda.h`, `nvvm.h`, etc.) | bindgen in `cust_raw/build/` | Generate Rust FFI bindings |

### 4.3 Rust / Cargo Crates Used

| Crate | Used By | Purpose |
|---|---|---|
| `bindgen` | `cust_raw`, `cudnn-sys` | Generate FFI bindings from C headers |
| `serde`, `serde_json` | `cuda_builder` | Parse `cargo --message-format=json` output |
| `strum` | `nvvm` | `NvvmArch::iter()` over all variants |
| `libc` | `rustc_codegen_nvvm` | C types for FFI |
| `tracing` | `rustc_codegen_nvvm` | Debug logging |
| `rustc_codegen_ssa` | `rustc_codegen_nvvm` | The rustc codegen SSA traits to implement |
| `rustc_middle`, `rustc_session`, etc. | `rustc_codegen_nvvm` | Access to rustc's internal compiler types |

---

## 🗺️ Part 5 — Putting It All Together: Full Data Flow

```
╔══════════════════════════════════════════════════════════════════════════════╗
║  USER WRITES GPU KERNEL                                                      ║
║                                                                              ║
║  // gpu_crate/src/lib.rs                                                     ║
║  #[no_mangle]                                                                ║
║  pub unsafe extern "C" fn vector_add(a: *const f32, b: *const f32,          ║
║                                       c: *mut f32, len: i32) { ... }        ║
╚════════════════════════════╦═════════════════════════════════════════════════╝
                             ║ cargo build (via cuda_builder in host build.rs)
                             ▼
╔═══════════════════════════════════════════════════════════════╗
║  rustc + rustc_codegen_nvvm.so                                ║
║                                                               ║
║  Rust source                                                  ║
║    → HIR → typechecking → MIR                                 ║
║    → CodegenCx + Builder (LLVM IR construction)               ║
║    → LLVM Module (in-memory)                                  ║
║    → LLVM bitcode (.bc file per CGU)                          ║
╚════════════════════╦══════════════════════════════════════════╝
                     ║ Vec<Vec<u8>> (bitcode bytes)
                     ▼
╔═══════════════════════════════════════════════════════════════╗
║  codegen_bitcode_modules()  (nvvm.rs)                         ║
║                                                               ║
║  merge_llvm_modules()  → single LLVM module                   ║
║  internalize_pass()    → mark non-kernels internal            ║
║  dce_pass()            → remove dead code                     ║
║                                                               ║
║  NvvmProgram::new()                                           ║
║  .add_module(merged.bc)                                       ║
║  .add_lazy_module(libdevice.bc)   ← CUDA math lib             ║
║  .add_lazy_module(libintrinsics.bc)                           ║
║  .compile(&[Arch(Compute75), ...])                            ║
║     → nvvmCompileProgram()  [libnvvm.so]                      ║
╚════════════════════╦══════════════════════════════════════════╝
                     ║ PTX text (Vec<u8>)
                     ▼
╔═══════════════════════════════════════════════════════════════╗
║  my_kernel.ptx   (written to disk by cuda_builder)            ║
╚════════════════════╦══════════════════════════════════════════╝
                     ║ include_str!("my_kernel.ptx")
                     ▼
╔═══════════════════════════════════════════════════════════════╗
║  HOST RUNTIME  (cust)                                         ║
║                                                               ║
║  cust::quick_init()                                           ║
║  Module::from_ptx(ptx, &[])                                   ║
║    → cuModuleLoadDataEx()                                     ║
║    → CUDA Driver JIT: PTX → cubin (for current GPU)          ║
║  module.get_function("vector_add")                            ║
║    → cuModuleGetFunction()                                    ║
║  DeviceBuffer::from_slice(&host_data)  → GPU memory          ║
║    → cuMemAlloc + cuMemcpyHtoD                                ║
║  launch!(func<<<grid, block, 0, stream>>>(a, b, c, n))        ║
║    → cuLaunchKernel()                                         ║
║  stream.synchronize()                                         ║
╚════════════════════╦══════════════════════════════════════════╝
                     ║
                     ▼
╔═══════════════════════════════════════════════════════════════╗
║  🖥️  GPU executes the kernel                                  ║
╚═══════════════════════════════════════════════════════════════╝
```

---

## 📎 Quick Reference Card

```
┌──────────────────┬────────────────────────────────────────────────────────┐
│ Crate            │ One-liner                                               │
├──────────────────┼────────────────────────────────────────────────────────┤
│ cuda_builder     │ Build script helper: cargo → PTX                        │
│ rustc_codegen_nvvm│ Custom rustc backend: Rust MIR → LLVM IR → bitcode    │
│ nvvm             │ Safe wrappers for libnvvm: bitcode → PTX               │
│ cust_raw         │ bindgen FFI to CUDA driver/runtime/nvvm/cublas         │
│ cust             │ High-level host API: context, memory, modules, streams │
│ cust_core        │ DeviceCopy trait: type-safe host↔device transfers      │
│ cudnn-sys        │ bindgen FFI to libcudnn                                 │
│ cudnn            │ Safe cuDNN: tensors, conv, RNN, attention, op-graph    │
└──────────────────┴────────────────────────────────────────────────────────┘

Key external libraries:
┌──────────────────┬────────────────────────────────────────────────────────┐
│ libnvvm.so       │ NVIDIA's LLVM IR → PTX compiler (part of CUDA Toolkit) │
│ libdevice.10.bc  │ NVIDIA GPU math intrinsics (LLVM bitcode)              │
│ libcuda.so       │ CUDA Driver API                                         │
│ libcudnn.so      │ cuDNN deep learning primitives                          │
│ LLVM             │ Bundled with rustc (not system LLVM)                   │
└──────────────────┴────────────────────────────────────────────────────────┘
```

---

*Generated from source exploration of [Rust-GPU/Rust-CUDA](https://github.com/Rust-GPU/Rust-CUDA) — April 2026.*
