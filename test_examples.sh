#!/bin/bash

# Exit immediately if any command fails
set -e

run_example() {
    # We use "$@" instead of $1 to capture the example name AND all its --features flags
    echo "=== Running Example: $@ ==="

    # Start cargo run with all arguments passed to the function
    cargo run --example $@ &
    CARGO_PID=$!

    sleep 6
    echo "Stopping example processes..."
    pkill -P $CARGO_PID || true
    kill $CARGO_PID 2>/dev/null || true
    wait $CARGO_PID 2>/dev/null || true
}

run_example "all_post_requests"
run_example "all_post_requests_shared --features thread_shared_struct"
run_example "all_post_requests_shared_send_tls --features thread_shared_struct,use_tokio_send,support_tls"
run_example "cross_middlewares"
run_example "cross_redirect"
run_example "default"
run_example "dynamic_path_with_slashes"
run_example "fast_start_server"
run_example "fast_start_server_shared --features thread_shared_struct"
run_example "html_render"
run_example "lazy_response --features lazy_response"
run_example "lazy_response_interceptor --features lazy_response"
run_example "middleware"
run_example "mini_server --features mini"
run_example "path_params"
run_example "public_files_serving"
run_example "redirect"
run_example "sending_files"
run_example "uploading_files"

echo "All examples executed successfully!"