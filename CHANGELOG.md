## [4.0.7]
- fix advancing the buffer size while handling massive data income 

## [4.0.6] - 2026-08-21
- updating macros and adding flexible functions to make the work more easier 
- Enhanced performance and added new features to improve HTTP request handling

## [4.0.4] - 2026-07-08
- adding support for advance headers interceptor

## [4.0.4] - 2026-07-08
- removed canceling the connection when there is unpredicted route
- much stable and widley tested version
- adding a lot of flexible methods like get_route for getting named routes
## [4.0.3] - 2026-06-25

- Updated file-sending macros to the optimized versions.
- Exported the `smallbox` crate dependency so users won't encounter local missing crate errors.
- Added recursive swallowing support for dynamic paths (e.g., catching trailing slashes/wildcards).

## [4.0.1] - 2026-06-13

### 🚀 Added
- **`fast_build!` Engine Feature:** Introduced a unified macro abstraction layer that automates server boilerplate, root initialization, and controller configurations. Features seamless signature flexibility supporting raw properties, multi-threaded state sharing (`thread_shared_struct`), nested controller trees (`children`), and interceptor middleware blocks.
- **Zero-Overhead `mini` Module:** Added bare-metal server building support meticulously optimized for tiny edge applications. Operates with strictly zero heap allocations and zero routing overhead via stack-allocated const generics and raw socket ring-buffer interaction through `CtxPtr`.
- **`LazyResponse` Pipeline Integration:** Added a high-performance deferred response layer (`LazyResponse`) to delay data assembly until the final microsecond of the connection cycle, maximizing throughput efficiency.
- **Advanced Interceptor Hierarchy:** Introduced parent interceptor cascading blocks (`apply_parents_interceptors`). Response interceptors are fully compatible and natively synchronized with the new `LazyResponse` architecture.

### ⚡ Optimized & Upgraded
- **`water_buffer` Core Overhaul:** Re-engineered internal buffers to scale raw throughput aggressively. Optimized single-byte iteration loops and per-connection allocation reuse to guarantee ultra-high performance and a stable workload baseline across major OS environments.
- **Parsing Velocity:** Enhanced internal memory layouts to keep HTTP/1 payload parsing speeds sustained down to a raw **1 microsecond runtime baseline**.

## 3.1.1 - 3.1.0
- Major performance optimizations and internal updates to ensure extremely high efficiency and speed, targeting top-tier benchmark results and aligning with the highest industry standards.
## 3.0.6
- splitting tls support to make the crate much easier to run in different environments so when you need to configure your server with tls you could add feature [support_tls]()
## 3.0.4
- fixes content length reading and increasing the size of parsing bytes to valid rust data type
- update packages 
## 3.0.2 
 - updates packages to the latest versions
## 3.0.1
 - breaking new records for speed and stability
 - adding more secure logics inside parsing http request and handling
 - new function implementations for controllers and capsules of the framework ( refers the way writing server application using water_http )
 - adding new facilities to writing responses using powerful rust macros
 - adding more styles for writing http requests and functions inside controllers
## 2.0.11
 - new send json method to context to make it much easier to use
 - new send status code method to send http status code as final response to the client inside context
 - multipart form data now support all arbitrary field headers 

## 2.0.10 
 - fixing linux kernel system call implementation in one of crates