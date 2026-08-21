# Water_HTTP Crate Documentation

Welcome to the official developer documentation for `water_http`, a hyper-performance, micro-second HTTP web framework designed for Rust. `water_http` combines the type safety of Rust with advanced systems programming concepts to deliver near bare-metal throughput while maintaining a friendly, macro-driven developer experience.

---

## Table of Contents
1. [Core Philosophy & Architecture](#1-core-philosophy--architecture)
2. [Installation & Requirements](#2-installation--requirements)
3. [Feature Flags](#3-feature-flags)
4. [Startup & Configurations](#4-startup--configurations)
5. [Controller & Routing System](#5-controller--routing-system)
6. [Request Lifecycle & Handling Context](#6-request-lifecycle--handling-context)
7. [Writing Responses](#7-writing-responses)
8. [Middlewares & Interceptors](#8-middlewares--interceptors)
9. [Advanced Architectures](#9-advanced-architectures)
    - [LazyResponse](#lazyresponse-deferred-payload-pipeline)
    - [Mini Engine](#the-mini-engine-zero-overhead)
    - [io_uring Platform Backend](#io_uring-platform-backend)
10. [RESTful Design Guidelines & Router Rules](#10-restful-design-guidelines--router-rules)
11. [Troubleshooting & System Setup](#11-troubleshooting--system-setup)

---

## 1. Core Philosophy & Architecture

The ultimate performance bottleneck in web applications is memory allocation and system calls. `water_http` optimizes for hardware alignment using the following strategies:

* **Zero-Allocation Parsing:** When an HTTP/1.x request is read from the OS socket into the socket ring-buffer, `water_http` parses standard headers, HTTP verbs, paths, and queries using direct slices from that buffer. No new memory allocations occur in the hot path.
* **Siloed Thread-Per-Core (Shared-Nothing):** When `use_tokio_send` is disabled, the framework binds worker threads directly to physical CPU cores (`cpu_affinity`) and uses `SO_REUSEPORT` at the socket level. Each thread acts as a siloed reactor, eliminating cross-thread CPU scheduling, context switching, and lock contention.
* **Persistent Connection Buffers:** Instead of allocating memory buffers per request, `water_http` allocates a dedicated connection buffer upon socket acceptance, recycling it for the entire duration of the keep-alive connection.

---

## 2. Installation & Requirements

Add `water_http` and its runtime dependency `tokio` to your `Cargo.toml`:

```toml
[dependencies]
water_http = "4.0.4"
tokio = { version = "1.48.0", features = ["full"] }
```

### Compiler Requirements
> [!IMPORTANT]
> The optional TLS support depends on crates that link against native C++ libraries. When compiling with TLS support, ensure `clang` / LLVM and `cmake` are installed on your build machine.
>
> On Ubuntu/Debian:
> ```shell
> sudo apt-get install build-essential cmake clang
> ```

---

## 3. Feature Flags

`water_http` exposes highly granular feature flags to optimize compilation size and run-time architectures:

| Feature Flag | Description | Dependencies |
| :--- | :--- | :--- |
| `debugging` | Enables detailed console tracing for routing and request parsing. | `tracing`, `tracing-subscriber` |
| `support_tls` | Enables secure SSL/TLS listeners. | `rustls`, `rustls-pemfile`, `tokio-rustls` |
| `use_only_http1` | Disables HTTP/2 support to reduce binary footprint and routing complexity. | None |
| `use_io_uring` | Replaces standard Epoll/Reactor loop with Linux asynchronous I/O (`io_uring`). | `tokio-uring`, `water_buffer/uring` |
| `cpu_affinity` | Pin worker threads to specific CPU cores for cache locality. | `core_affinity` |
| `thread_shared_struct` | Allows injecting state/factory structs shared across connection threads. | None |
| `lazy_response` | Defers writing payloads to the socket until handlers/interceptors conclude. | None |
| `mini` | Bypasses macro-routers and controller structures for direct, raw socket handler loops. | None |
| `auto_encode_response` | Compresses responses automatically (gzip, brotli, deflate, zstd, snappy, lz4, bzip2). | None |
| `accept_transfer_chunked`| Enables incoming HTTP chunked transfer encoding processing. | None |

---

## 4. Startup & Configurations

The framework is configured using `ServerConfigurations` and started using the `RunServer!` macro.

### Configuration Methods

```rust
use water_http::server::ServerConfigurations;

fn setup_server() -> ServerConfigurations {
    // Bind to localhost on port 8084
    let mut config = ServerConfigurations::bind("127.0.0.1", 8084);
    
    // Customize configuration options
    config.set_listeners_count(2);
    
    // Enable core affinity for performance
    #[cfg(feature = "cpu_affinity")]
    config.enable_core_affinity();
    
    config
}
```

### Setup TLS Configuration
To enable SSL/TLS listeners, ensure the `support_tls` feature is enabled:

```rust
use water_http::server::ServerConfigurations;

fn enable_tls(config: &mut ServerConfigurations) {
    config.tls_ports = vec![443];
    config.set_tls_certificate(
        "./ssl/certificate.crt",
        "./ssl/private.key",
        None // CA Bundle path (optional)
    );
}
```

---

## 5. Controller & Routing System

Routing in `water_http` is defined in a hierarchical structure using the **Controller Tree** approach:

```
         MAIN_ROOT (InitControllersRoot)
                |
          RootController (WaterController)
           /          \
  UserController   ProductController
```

### Initializing the Root Context

Use the `InitControllersRoot!` macro to define your generic storage type (state/holder) and stack size limits. Stack limits prevent malicious users from performing buffer overflow/DoS attacks.

```rust
use std::collections::HashMap;
use water_http::InitControllersRoot;

// Holder type acts as request context storage
type MainHolderType = HashMap<String, String>;

InitControllersRoot! {
    /// Identifier for your route root
    name: MAIN_ROOT,
    /// Holder storage type
    holder_type: MainHolderType,
    /// Max headers to read from single request (default: 16)
    headers_length: 16,
    /// Max queries to parse in URI query string (default: 16)
    queries_length: 16
}
```

### Defining Controllers

Use the `WaterController!` macro to build endpoints, declare child controllers, register local middlewares, or attach interceptors.

```rust
use water_http::WaterController;

WaterController! {
    holder -> crate::MainHolderType,
    name -> RootController,
    functions -> {
        // HTTP verb => Path => Handler Function (inlined or mapped)
        GET => / => index_handler(context) async {
            let _ = context.send_str("Welcome to Water HTTP").await;
        }
        
        // Dynamic path variables binding
        GET -> api/users/{id} -> get_user(context) async {
            let id = context.get_from_path_params("id");
            if let Some(user_id) = id {
                let _ = context.send_string_slice(&format!("User ID: {}", user_id)).await;
            } else {
                context.send_status_code_as_final_response(water_http::http::status_code::HttpStatusCode::BAD_REQUEST).await;
            }
        }
    },
    children -> ([
        UserController
    ])
}

WaterController! {
    holder -> crate::MainHolderType,
    name -> UserController,
    functions -> {
        GET => profile => profile_handler(context) async {
            let _ = context.send_str("User Profile Page").await;
        }
    }
}
```

### Nested Controller Architecture

The real power of the controller system is the ability to **nest controllers into deep trees**. Each controller can declare any number of child controllers using the `children` key. This nesting:

- **Automatically composes URL prefixes** — a child controller's path is prefixed with all ancestor prefixes up the chain.
- **Inherits middlewares transitively** — a child route will execute every middleware registered on every ancestor, ordered from root to leaf.
- **Composes interceptors** — works identically with response interceptors (requires `lazy_response` feature).
- **Allows scoped middleware overrides** — a child can opt out of ancestor middleware inheritance.

#### How Prefix Composition Works

During server startup, the framework traverses the controller tree recursively. Each level's `prefix` string is concatenated onto the path accumulated from all ancestors above it. This means you only declare **local path segments** at each controller, and the full absolute path is assembled automatically.

```
MAIN_ROOT
  └── ApiController (prefix: "api")
        └── V1Controller (prefix: "v1")
              └── UsersController (prefix: "users")
                    ├── GET  /              → resolves to GET  /api/v1/users
                    ├── GET  /{id}          → resolves to GET  /api/v1/users/{id}
                    └── POST /              → resolves to POST /api/v1/users
```

All three routes are registered automatically — no need to manually type `/api/v1/users` in every handler definition.

#### How Middleware Inheritance Works

By default every controller **inherits and runs** the middlewares of all its ancestors, from the root down to itself, before the handler executes:

```
Root MW → Api MW → UsersController MW → [HANDLER]
```

This is controlled by the `apply_parents_middlewares` flag:

| Value | Behaviour |
| :--- | :--- |
| `true` *(default — no need to declare it)* | The controller checks and runs all ancestor middlewares before its own, then calls the handler. |
| `false` | Ancestor middlewares are **ignored**. Only this controller's own middleware (if any) runs. |

> [!IMPORTANT]
> You only need to write `apply_parents_middlewares -> false` when you explicitly want to **opt out**. If you do nothing, inheritance is already active.



#### Full Nested Example

```rust
use water_http::WaterController;

// ─── Root controller ──────────────────────────────────────────────────────────
WaterController! {
    holder  -> crate::MainHolderType,
    name    -> RootController,
    functions -> {
        GET => / => index(context) async {
            let _ = context.send_str("Welcome").await;
        }
    },
    // Attach two child controllers
    children -> ([ApiController])
}

// ─── /api level ───────────────────────────────────────────────────────────────
WaterController! {
    holder  -> crate::MainHolderType,
    name    -> ApiController,
    prefix  -> ("api"),             // all routes here start with /api/...

    // A middleware that runs for every route in ApiController AND its children.
    // Performs authentication token validation.
    middleware -> (context {
        let token = context.get_from_headers("Authorization");
        if token.is_none() {
            context.send_status_code_as_final_response(
                water_http::http::status_code::HttpStatusCode::UNAUTHORIZED
            ).await;
            return server::MiddlewareResult::Stop;
        }
        server::MiddlewareResult::Pass
    }),

    functions -> {
        GET => health => health_check(context) async {
            // resolves to: GET /api/health
            let _ = context.send_str("OK").await;
        }
    },

    children -> ([UsersController, ProductsController])
}

// ─── /api/users level ─────────────────────────────────────────────────────────
WaterController! {
    holder  -> crate::MainHolderType,
    name    -> UsersController,
    prefix  -> ("users"),           // routes here become /api/users/...

    // Additional middleware that only applies to this controller and its children.
    // Checks that the requesting user has the "users" permission scope.
    middleware -> (context {
        // scope check ...
        server::MiddlewareResult::Pass
    }),

    functions -> {
        GET  => /        => list_users(context)   async { /* GET /api/users   */ }
        POST => /        => create_user(context)  async { /* POST /api/users  */ }
        GET  => {id}     => get_user(context)     async {
            // GET /api/users/{id}
            let id = context.get_from_path_params("id");
            let _ = context.send_string_slice(&format!("User: {:?}", id)).await;
        }
        PUT  => {id}     => update_user(context)  async { /* PUT /api/users/{id} */ }
    }
}

// ─── /api/products level ──────────────────────────────────────────────────────
WaterController! {
    holder  -> crate::MainHolderType,
    name    -> ProductsController,
    prefix  -> ("products"),        // routes here become /api/products/...

    // By default apply_parents_middlewares is true, which means this controller
    // would inherit and run ApiController's auth middleware.
    // Setting it to false stops the ancestor walk here — only this controller's
    // own middleware (if any) runs, making these routes publicly accessible.
    apply_parents_middlewares -> false,

    functions -> {
        GET  => /        => list_products(context) async { /* GET /api/products */ }
        GET  => {id}     => get_product(context)   async { /* GET /api/products/{id} */ }
    }
}
```

#### Key Benefits at a Glance

| Benefit | How it Works |
| :--- | :--- |
| **Zero path repetition** | Prefix is declared once per level; full path is composed automatically. |
| **Layered authentication** | Middleware at `ApiController` enforces auth for all `/api/...` routes without touching each handler. |
| **Scoped middleware** | Middleware at `UsersController` only gates `/api/users/...` routes. |
| **`apply_parents_middlewares` (default `true`)** | When `true` (the default), the framework walks all ancestor controllers and executes their middlewares before the handler. Set to `false` on any controller to stop the walk at that point — only that controller's own middleware (if any) runs, useful for exposing a public sub-tree inside an otherwise protected namespace. |
| **Deep nesting supported** | The tree can be as deep as needed — the framework walks the entire ancestor chain at startup, not at runtime, so nesting depth adds zero per-request overhead. |
| **Compile-time routing** | All paths, prefixes, and middleware chains are resolved once at server start. At request time the framework simply does a hash-map lookup for static routes or a flat segment-count vec scan for dynamic routes. |

---

## 6. Request Lifecycle & Handling Context

The `HttpContext` struct is the payload wrapper passed to handlers, containing details of the TCP connection, parsed request slices, path variables, query items, and response methods.

### HttpContext Key Methods

| Signature | Description |
| :--- | :--- |
| `path(&self) -> &str` | Returns the raw request path (e.g. `/api/users/12`). |
| `method(&self) -> &str` | Returns the HTTP request verb (e.g., `GET`, `POST`). |
| `get_peer_socket(&self) -> &SocketAddr` | Returns the peer socket address of the incoming TCP link. |
| `get_from_headers(&self, key: &str) -> Option<Cow<'_, str>>` | Retrieve a specific header value. |
| `get_from_path_params(&self, key: &str) -> Option<&String>` | Retrieve a path variable bound inside `{}` brackets. |
| `get_from_path_query(&self, key: &str) -> Option<Cow<'_, str>>` | Retrieve a query string value (e.g. `?id=12`). |
| `get_body_full_bytes(&mut self) -> Result<Option<&Vec<u8>>, WaterErrors>` | Read and return the entire incoming body buffer in heap memory. |
| `get_body_as_json<V: Deserialize>(&mut self) -> Result<V, serde_json::Error>` | Parse the request body automatically as a JSON structure. |
| `get_body_as_multipart(&mut self) -> Result<FormDataAll, WaterErrors>` | Parse and extract multipart form data fields and files. |
| `get_body_map(&mut self) -> Result<DynamicBodyMap, WaterErrors>` | Extract request body as a dynamic map regardless of whether it's multipart or URL-encoded. |
| `set_headers_interceptor(&mut self, status: HeaderInterceptorApplyFor, interceptor: HeaderInterceptorFunction)` | Dynamically attach a hook function to intercept and mutate headers. |
| `set_lazy_response(&mut self, res: LazyResponse)` | Register a `LazyResponse` to defer output writing. |

### Accessing Request Payload Example

```rust
use serde::Deserialize;
use water_http::server::HttpContext;

#[derive(Deserialize)]
struct RegistrationPayload {
    username: String,
    email: String,
}

async fn handle_registration(context: &mut HttpContext<'_, crate::MainHolderType, 16, 16>) {
    // 1. Get path query param
    let debug_mode = context.get_from_path_query("debug");
    
    // 2. Read headers
    let user_agent = context.get_from_headers("User-Agent");
    
    // 3. Deserialize JSON request body
    match context.get_body_as_json::<RegistrationPayload>().await {
        Ok(payload) => {
            println!("Registered user: {} with email: {}", payload.username, payload.email);
            let _ = context.send_str("Registration successful").await;
        }
        Err(_) => {
            context.send_status_code_as_final_response(
                water_http::http::status_code::HttpStatusCode::BAD_REQUEST
            ).await;
        }
    }
}
```

### 6.1. Extracting Request Bodies (JSON, Multipart, and Dynamic Maps)

`water_http` offers precise structures to interact with different body payloads.

#### A. JSON Parsing
Use `context.get_body_as_json::<T>()` to automatically deserialize raw body bytes into any struct implementing `serde::Deserialize`.

#### B. Multipart Form Data (`FormDataAll` & `HeapFormField`)
When parsing file uploads or boundary-delimited payloads, call `context.get_body_as_multipart()`. It parses the buffer and constructs a `FormDataAll` instance on the heap:

```rust
pub struct FormDataAll {
    pub fields: Vec<HeapFormField>,
}

pub struct HeapFormField {
    pub multipart: KeyValueMap, // Key-value metadata of form boundary headers (e.g. name, filename)
    pub data: Vec<u8>,          // Raw binary content of the form field
}
```

Key methods on `HeapFormField`:
* `data(&self) -> &[u8]` : Retrieve raw binary payload.
* `is_file(&self) -> bool` : Returns `true` if this field contains a `filename` header.
* `content_type(&self) -> Option<&[u8]>` : Returns the optional MIME type (e.g., `image/jpeg`).

Key methods on `FormDataAll`:
* `get_field(&self, key: &str) -> Option<&HeapFormField>` : Find field by field name.
* `get_mut(&mut self, key: &str) -> Option<&mut HeapFormField>` : Find mutable reference to field.

##### Multipart Processing Example (File Upload)
```rust
use water_http::server::HttpContext;

async fn upload_handler(context: &mut HttpContext<'_, crate::MainHolderType, 16, 16>) {
    if let Ok(form_data) = context.get_body_as_multipart().await {
        if let Some(field) = form_data.get_field("avatar") {
            let file_data = field.data(); // Read binary payload
            let is_image = field.is_file();
            
            if is_image {
                // Write the raw upload bytes to disk
                if let Ok(mut file) = tokio::fs::File::create("./public/uploads/avatar.png").await {
                    use tokio::io::AsyncWriteExt;
                    let _ = file.write_all(file_data).await;
                    let _ = context.send_str("Upload successful!").await;
                    return;
                }
            }
        }
    }
    context.send_status_code_as_final_response(water_http::http::status_code::HttpStatusCode::BAD_REQUEST).await;
}
```

#### C. Unified Dynamic Parsing (`DynamicBodyMap` & `DynamicBodyMapTrait`)
If your route handles requests that can be either `multipart/form-data` or `application/x-www-form-urlencoded`, use `context.get_body_map()`. It returns a unified wrapper enum `DynamicBodyMap`:

```rust
pub enum DynamicBodyMap {
    FormField(FormDataAll),
    Xww(HeapXWWWFormUrlEncoded)
}
```

Both `FormDataAll` and `HeapXWWWFormUrlEncoded` implement the `DynamicBodyMapTrait`, allowing you to fetch fields uniformly using standard keys:

```rust
pub trait DynamicBodyMapTrait {
    fn get_as_bytes(&self, key: &str) -> Option<&[u8]>;
    fn get(&self, key: &str) -> Option<Cow<'_, str>>;
    fn all(&self) -> HashMap<String, Bytes>;
    fn get_as_encoded_string(&self, key: &str) -> Option<String>; // Decodes URL-encoded field strings automatically
}
```

##### Dynamic Body Map Querying Example
```rust
use water_http::server::HttpContext;
use water_http::http::request::DynamicBodyMapTrait;

async fn process_dynamic_form(context: &mut HttpContext<'_, crate::MainHolderType, 16, 16>) {
    if let Ok(body_map) = context.get_body_map().await {
        // Query fields using the trait methods uniformly
        let username = body_map.get("username");
        let age_bytes = body_map.get_as_bytes("age");
        let encoded_desc = body_map.get_as_encoded_string("description");
        
        if let Some(user) = username {
            let _ = context.send_string_slice(&format!("Hello, {}!", user)).await;
            return;
        }
    }
    context.send_status_code_as_final_response(water_http::http::status_code::HttpStatusCode::BAD_REQUEST).await;
}
```

### 6.2. Dynamic Header Interceptors

`water_http` supports registering raw header hooks inside middlewares or endpoint handlers via the `context.set_headers_interceptor(...)` method. These interceptors run automatically just after the response status code is written but *before* headers are flushed, providing a perfect opportunity to dynamically modify outgoing headers (e.g. for CORS configuration, custom caching policies, or security banners).

```rust
pub fn set_headers_interceptor(
    &mut self,
    status: HeaderInterceptorApplyFor,
    interceptor: HeaderInterceptorFunction
)
```

Where:
* `HeaderInterceptorApplyFor` defines when to fire:
  * `HeaderInterceptorApplyFor::All` : Run for all responses.
  * `HeaderInterceptorApplyFor::Specific(HttpStatusCode)` : Run only when the status matches.
* `HeaderInterceptorFunction` is a function pointer callback:
  * `for<'a,'context> fn (&mut HttpSender<'a,'context,HEADER_SIZE,QUERY_SIZE>)`

##### Example: CORS and Global Security Headers Hook
```rust
use water_http::server::{HttpContext, MiddlewareResult, HeaderInterceptorApplyFor};

async fn global_cors_middleware(context: &mut HttpContext<'_, crate::MainHolderType, 16, 16>) -> MiddlewareResult {
    // Register header interceptor for all status codes
    context.set_headers_interceptor(
        HeaderInterceptorApplyFor::All,
        |sender| {
            // Write CORS headers efficiently
            sender.set_header_ef("Access-Control-Allow-Origin", "*");
            sender.set_header_ef("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
            sender.set_header_ef("Access-Control-Allow-Headers", "Content-Type, Authorization");
            sender.set_header_ef("Server", "water_http");
        }
    );
    MiddlewareResult::Pass
}
```

---

## 7. Writing Responses

Responses can be written in three distinct ways depending on the requirements for control and efficiency:

### A. Direct context methods (Easiest)
Methods directly on `HttpContext` send the status code and final body in a single call.

```rust
// Write raw text
context.send_str("Hello World").await;

// Write String slice
context.send_string_slice(&format!("Server Time: {:?}", std::time::SystemTime::now())).await;

// Write JSON payload
context.send_json(&serde_json::json!({ "status": "active" })).await;

// Redirect 
context.redirect("/new-path").await;
```

### B. High-performance `response!` macro
The `response!` macro simplifies response writing by packaging status codes, headers, and body writes into single-line macros.

#### All Supported `response!` Macro Syntax Variants:

* **Plain Static String Response:**
  ```rust
  response!(context -> "Hello World");
  ```
  Sends status code `200 OK` with a plaintext payload.

* **Format / Dynamic String Response:**
  ```rust
  response!(context string -> "User ID: {}", user_id);
  response!(context string -> my_string_variable);
  ```
  Format-prints strings dynamically on-the-fly and writes them to the response buffer.

* **JSON Serialization Response:**
  ```rust
  response!(context json -> serde_json::json!({ "success": true, "code": 200 }));
  ```
  Automatically sets the header `"Content-Type: application/json"` and serializes the given structure.

* **Serving Static Files (Streaming / Video ranges supported):**
  ```rust
  response!(context file -> "./public/images/logo.png");
  ```
  Streams files from the local filesystem. Supports browser range requests (useful for streaming media). Automatically responds with `404 Not Found` if the file doesn't exist.

* **Serving Static Files with Binary Modification (Chunk Callback):**
  ```rust
  response!(context file -> "./public/text/secret.txt", |chunk| {
      for byte in chunk {
          *byte ^= 0x55; // Apply basic XOR encryption on-the-fly to each chunk before streaming
      }
  });
  ```

* **Triggering File Downloads (Attachment):**
  ```rust
  response!(context download -> "./public/documents/report.pdf");
  ```
  Sets the headers `"Content-Disposition: attachment; filename=report.pdf"`, forcing browser download prompts rather than inline loading.

* **Triggering File Downloads with Binary Modification (Chunk Callback):**
  ```rust
  response!(context download -> "./public/raw_data.bin", |chunk| {
      // Modify each chunk on-the-fly before download transmission
  });
  ```

### C. Low-level `HttpSender` structure
Allows step-by-step writing of headers, status codes, and manual buffer mutations.

```rust
use water_http::http::HttpSenderTrait;
use water_http::http::status_code::HttpStatusCode;

async fn manual_response(context: &mut HttpContext<'_, crate::MainHolderType, 16, 16>) {
    let mut sender = context.sender();
    sender.send_status_code(HttpStatusCode::CREATED);
    sender.set_header("Custom-Header", "Water-Powered");
    sender.set_header_ef("Cache-Control", "no-cache");
    
    let _ = sender.send_str("Created Resource").await;
}
```

---

## 8. Middlewares & Interceptors

Middlewares and Interceptors allow hook logic execution before a request reaches the final router endpoint, or before the output gets sent to the socket.

* **Middleware:** Runs *before* the handler. Can pass request variables, validate auth tokens, or abort processing completely by returning `MiddlewareResult::Stop`.
* **Interceptor (LazyResponse only):** Runs *after* the endpoint handler, allowing parent modules to overwrite headers or rewrite body slices before serialization.

### Implementing Middleware

```rust
use water_http::server::{MiddlewareResult, HttpContext};
use water_http::response;

async fn auth_middleware(context: &mut HttpContext<'_, crate::MainHolderType, 16, 16>) -> MiddlewareResult {
    let auth_token = context.get_from_headers("Authorization");
    
    if let Some(token) = auth_token {
        if token == "Bearer secret-token-key" {
            return MiddlewareResult::Pass; // Let request traverse down-tree
        }
    }
    
    // Stop the request, write 401 Unauthorized, block handler execution
    context.send_status_code_as_final_response(water_http::http::status_code::HttpStatusCode::UNAUTHORIZED).await;
    MiddlewareResult::Stop
}
```

Attach a middleware to your controller tree inside the `WaterController!` definition:

```rust
WaterController! {
    holder -> crate::MainHolderType,
    name -> ProtectedController,
    functions -> {
        GET => secret => secret_handler(context) async {
            let _ = context.send_str("Secret data accessed").await;
        }
    },
    middleware -> (context {
        crate::auth_middleware(context).await
    })
}
```

---

## 9. Advanced Architectures

### `LazyResponse` (Deferred Payload Pipeline)

Normally, writing to `HttpContext` sends chunks immediately down to the socket stream. By enabling the `lazy_response` feature, handlers write payloads into a deferred staging area. This allows upstream parents or controllers to rewrite body details or headers before the network transfer commits.

```rust
use water_http::http::LazyResponse;

WaterController! {
    holder -> crate::MainHolderType,
    name -> SharedController,
    functions -> {
        GET => data => get_data(context) async {
            let mut response = LazyResponse::new();
            response.set_text_response("Endpoint response payload");
            context.set_lazy_response(response);
        }
    },
    // The parent interceptor can override the payload
    interceptor -> (context {
        if let Some(ref mut lazy) = context.lazy_response {
            lazy.set_header("X-Override", "True");
            lazy.set_text_response("Intercepted and modified response!");
        }
    })
}
```

### The `mini` Engine (Zero-Overhead)

For hyper-performance microservices requiring zero-heap allocations and minimal CPU footprint, the `mini` engine runs handlers on a raw loop bypassing routing, macros, or controller trees.

```rust
use water_http::server::mini::{CtxPtr, HandlerFn, serve};
use water_http::server::ServerConfigurations;

fn main() {
    let config = ServerConfigurations::bind("0.0.0.0", 8084);
    let no_init = || async {};
    
    // Start raw server listening on 8084
    serve::<16, 10, _, _, _>(
        config, 
        HandlerFn(|ctx: CtxPtr<16, 10>| mini_handler(ctx)), 
        Some(no_init)
    );
}

async fn mini_handler(mut ctx: CtxPtr<16, 10>) {
    let context = ctx.get();
    context.set_header("Content-Length", "11");
    context.write_body_bytes(b"Hello World");
}
```

### `io_uring` Platform Backend

Linux distributions supporting `io_uring` can compile the crate with the `use_io_uring` feature flag. The runtime will replace epoll events with queue submission rings directly in the kernel space.

> [!CAUTION]
> **Limitations of `io_uring`:**
> 1. **No SSL/TLS:** Because SSL/TLS requires payload decoding in user-space buffers (via OpenSSL/Rustls), system call reduction using kernel-space queues loses its performance boost. The runtime blocks TLS when running `io_uring`.
> 2. **Plaintext HTTP/1.x only:** HTTP/2 streaming multiplexes frames over a single TCP stream. Parsing asynchronous frame completions requires epoll synchronization, meaning HTTP/2 multiplexing is not supported on the standard `io_uring` loop.

---

## 10. RESTful Design Guidelines & Router Rules

To keep path lookups at near-constant time complexity, the `water_http` router resolves endpoints strictly mapping path keys to handler function pointers.

> [!WARNING]
> **Strict Path-Based Routing (No Method Multiplexing):**
> You cannot bind multiple HTTP verbs (such as `GET` and `POST`) to the exact same path string (e.g. `/api/data`). If paths overlap, the last compiled endpoint will overwrite previous mappings in the lookup table.

### Recommended Workarounds:
Instead of overloading a single path:
* **Explicit Endpoint Actions:**
  * Use `GET => /api/data_get`
  * Use `POST => /api/data_post`
* **Route Prefixes:** Add explicit verbs or CRUD indicators to path definitions (e.g. `/api/users/get/{id}`, `/api/users/update/{id}`).

---

## 11. Troubleshooting & System Setup

### SSL/TLS Compilation Failures
* **Error:** `openssl-sys` or `rustls` compilation failed because compiler cannot locate headers.
* **Resolution:** Ensure `pkg-config`, `libssl-dev` (Linux), or `openssl` (Homebrew on macOS) is installed.
  ```shell
  # MacOS
  brew install openssl pkg-config
  
  # Ubuntu/Debian
  sudo apt-get install pkg-config libssl-dev
  ```

### Linux Socket Options for Thread-per-Core
If launching multiple reactors bound using `SO_REUSEPORT`, verify your system allows socket reuse:
```shell
sysctl net.ipv4.ip_unprivileged_port_start
```

---

## 12. Code Examples Index

The crate includes 19 functional examples in the [examples/](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples) directory demonstrating every configuration and feature combination:

| Example File | Description / Key Focus | Relevant Feature Flags |
| :--- | :--- | :--- |
| [all_post_requests.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/all_post_requests.rs) | General routing of POST requests, body extraction, and basic JSON mapping. | None |
| [all_post_requests_shared.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/all_post_requests_shared.rs) | Handling POST payloads while sharing state across local execution sets. | `thread_shared_struct` |
| [all_post_requests_shared_send_tls.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/all_post_requests_shared_send_tls.rs) | Fully multi-threaded TLS server sharing context state across cores. | `thread_shared_struct`, `support_tls`, `use_tokio_send` |
| [cross_middlewares.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/cross_middlewares.rs) | Passing/sharing middleware interceptors across nested controller boundaries. | None |
| [cross_redirect.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/cross_redirect.rs) | Route mapping and link direction between child and parent controllers. | None |
| [default.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/default.rs) | Baseline minimum setup for starting the server. | None |
| [dynamic_path_with_slashes.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/dynamic_path_with_slashes.rs) | Router match tests for routes with custom, complex forward-slash layouts. | None |
| [fast_start_server.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/fast_start_server.rs) | Startup configuration utilizing simple, standard parameters. | None |
| [fast_start_server_shared.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/fast_start_server_shared.rs) | Quick configuration setup with custom shared context values. | `thread_shared_struct` |
| [html_render.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/html_render.rs) | Serving dynamic web pages by integrating with templating frameworks like `askama`. | None |
| [lazy_response.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/lazy_response.rs) | Basic overview of setting deferred/lazy responses. | `lazy_response` |
| [lazy_response_interceptor.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/lazy_response_interceptor.rs) | Complex interceptor hierarchies modifying child controller payloads before network flushing. | `lazy_response` |
| [middleware.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/middleware.rs) | Standard local request blocking and middleware setup example. | None |
| [mini_server.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/mini_server.rs) | Ultra-high performance plaintext server bypassing macro routers completely. | `mini` |
| [path_params.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/path_params.rs) | Binding dynamic parameters inside path segments (e.g. `users/{id}`). | None |
| [public_files_serving.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/public_files_serving.rs) | Serving files dynamically from a folder designated for public client assets. | None |
| [redirect.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/redirect.rs) | Sending redirection headers and codes to browser clients. | None |
| [sending_files.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/sending_files.rs) | Streaming files, download attachments, range settings, and dynamic block crypting callbacks. | None |
| [uploading_files.rs](file:///Users/hassansharara/work/projects/rust/h_crates/water/water_http/examples/uploading_files.rs) | Handling file upload streams, parsing boundary formats, and saving to disk. | None |
