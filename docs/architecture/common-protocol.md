# 📄 Architecture: Protocol Definitions

**File:** [crates/common/proto/compute.proto](/crates/common/proto/compute.proto)

This file serves as the "source of truth" for the entire project. Because it is written in Language-Agnostic Protobuf, it ensures that your macOS client and Windows/Linux host can communicate even if they are built with different toolchains.

### The Service: `CUDAExecutor`

This defines the interface of our Remote Procedure Call (RPC).

* **Method:** `ExecuteCode`
* **Pattern:** **Request-Stream**. The client sends one single batch of data (the code), but the server responds with a continuous "stream" of messages. This is vital so the user sees compiler warnings and `printf` outputs as they happen, rather than waiting for the entire job to finish.

### The Message: `ComputeRequest`

This represents the payload sent from your local machine to the remote GPU server.

1. **`source_code`**: The raw string content of the `.cu` file.
2. **`file_name`**: Allows the Host to save the file with the correct name (e.g., `vector_add.cu`) so that error messages from the compiler point to the correct filename.
3. **`compiler_flags`**: A list of strings (e.g., `["-O3", "-arch=sm_80"]`). This gives the user control over the `nvcc` compilation process from their local CLI.

### The Message: `ComputeResponse`

This is the data packet pushed from the Server back to the Client.

1. **`output`**: A single line or chunk of text. This could be a compiler warning, a status update ("Compiling..."), or the actual output of the executed program.
2. **`is_error`**: A boolean flag. If `true`, the client can choose to render the text in **red** in the terminal to signify `stderr` or a crash.

### Why use stream?

If you didn't use a stream, the client would send the code and then sit in silence for 10 seconds while the server compiles and runs it. With a stream, as soon as nvcc prints its first line of output, the Host can push that line to the Client immediately. This makes the CLI feel much more responsive.
