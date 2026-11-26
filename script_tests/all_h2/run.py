import socket
import sys
import time
import uuid
import mimetypes
import h2.connection
import h2.config
import h2.events

# ------------------------------
# Core H2 Protocol Engine Helper
# ------------------------------
def execute_h2_request(sock, conn, method, path, headers_dict, body_bytes, split_delay=None):
    """
    Executes a strict HTTP/2 request over an existing socket and h2 connection.
    Supports payload splitting to preserve your 'half-send' test timelines.
    """
    stream_id = conn.get_next_available_stream_id()

    # 1. Build pseudo-headers and mix in custom headers
    headers = [
        (':method', method),
        (':path', path),
        (':authority', f"{sock.getpeername()[0]}:{sock.getpeername()[1]}"),
        (':scheme', 'http'),
    ]
    for k, v in headers_dict.items():
        headers.append((k.lower(), str(v)))

    # 2. Send HEADERS frame (keep stream open if there's a body)
    has_body = len(body_bytes) > 0
    conn.send_headers(stream_id, headers, end_stream=not has_body)
    sock.sendall(conn.data_to_send())

    # 3. Send DATA frames (The Payload)
    if has_body:
        if split_delay:
            half = len(body_bytes) // 2

            # First frame fragment
            conn.send_data(stream_id, body_bytes[:half], end_stream=False)
            sock.sendall(conn.data_to_send())
            print(f"   [H2 DATA] Sent first half ({half} bytes)...")

            time.sleep(split_delay)

            # Second frame fragment closes the stream
            conn.send_data(stream_id, body_bytes[half:], end_stream=True)
            sock.sendall(conn.data_to_send())
            print(f"   [H2 DATA] Sent second half ({len(body_bytes) - half} bytes).")
        else:
            # Send single data frame closing the stream
            conn.send_data(stream_id, body_bytes, end_stream=True)
            sock.sendall(conn.data_to_send())
            print(f"   [H2 DATA] Sent full payload ({len(body_bytes)} bytes).")

    # 4. Handle incoming HTTP/2 binary response frames
    response_ended = False
    response_status = None
    response_body = b""

    while not response_ended:
        data = sock.recv(65535)
        if not data:
            print("   [Error] Server disconnected socket early.")
            break

        events = conn.receive_data(data)
        for event in events:
            if isinstance(event, h2.events.ResponseReceived):
                response_status = dict(event.headers)[b':status'].decode()
            elif isinstance(event, h2.events.DataReceived):
                response_body += event.data
                conn.acknowledge_received_data(event.flow_controlled_length, stream_id)
            elif isinstance(event, h2.events.StreamEnded):
                response_ended = True
                break

        # Send back protocol-level ACKs or flow control updates
        bytes_to_send = conn.data_to_send()
        if bytes_to_send:
            sock.sendall(bytes_to_send)

    print(f"Response: {response_status}")
    print(response_body.decode(errors='replace'))

# ------------------------------
# Test Case Wrappers
# ------------------------------
def send_get(sock, conn, path):
    print("\n=== GET Request ===")
    execute_h2_request(sock, conn, "GET", path, {}, b"")

def send_urlencoded_post(sock, conn, path, data):
    print("\n=== x-www-form-urlencoded POST ===")
    body_bytes = "&".join(f"{k}={v}" for k, v in data.items()).encode()
    headers = {"Content-Type": "application/x-www-form-urlencoded"}
    execute_h2_request(sock, conn, "POST", path, headers, body_bytes)

def send_half_urlencoded_post(sock, conn, path, data):
    print("\n=== x-www-form-urlencoded POST (half-send) ===")
    body_bytes = "&".join(f"{k}={v}" for k, v in data.items()).encode()
    headers = {"Content-Type": "application/x-www-form-urlencoded"}
    execute_h2_request(sock, conn, "POST", path, headers, body_bytes, split_delay=2)

def send_multipart_post(sock, conn, path, fields, files=None):
    print("\n=== Multipart/form-data POST ===")
    boundary = uuid.uuid4().hex
    lines = []
    for name, value in fields.items():
        lines.extend([f"--{boundary}", f'Content-Disposition: form-data; name="{name}"', "", value])
    if files:
        for field_name, (filename, file_content, content_type) in files.items():
            if content_type is None:
                content_type = mimetypes.guess_type(filename)[0] or "application/octet-stream"
            lines.extend([
                f"--{boundary}",
                f'Content-Disposition: form-data; name="{field_name}"; filename="{filename}"',
                f"Content-Type: {content_type}", "", file_content
            ])
    lines.extend([f"--{boundary}--", ""])
    body_bytes = "\r\n".join(lines).encode()
    headers = {"Content-Type": f"multipart/form-data; boundary={boundary}"}
    execute_h2_request(sock, conn, "POST", path, headers, body_bytes)

def send_half_multipart_post(sock, conn, path, fields, files=None):
    print("\n=== Multipart/form-data POST (half-send) ===")
    boundary = uuid.uuid4().hex
    lines = []
    for name, value in fields.items():
        lines.extend([f"--{boundary}", f'Content-Disposition: form-data; name="{name}"', "", value])
    if files:
        for field_name, (filename, file_content, content_type) in files.items():
            if content_type is None:
                content_type = mimetypes.guess_type(filename)[0] or "application/octet-stream"
            lines.extend([
                f"--{boundary}",
                f'Content-Disposition: form-data; name="{field_name}"; filename="{filename}"',
                f"Content-Type: {content_type}", "", file_content
            ])
    lines.extend([f"--{boundary}--", ""])
    body_bytes = "\r\n".join(lines).encode()
    headers = {"Content-Type": f"multipart/form-data; boundary={boundary}"}
    execute_h2_request(sock, conn, "POST", path, headers, body_bytes, split_delay=2)

def send_binary_post(sock, conn, path, data_bytes):
    print("\n=== Binary POST ===")
    headers = {"Content-Type": "application/octet-stream"}
    execute_h2_request(sock, conn, "POST", path, headers, data_bytes)

def send_half_binary_post(sock, conn, path, data_bytes):
    print("\n=== Binary POST (half-send) ===")
    headers = {"Content-Type": "application/octet-stream"}
    execute_h2_request(sock, conn, "POST", path, headers, data_bytes, split_delay=2)

# Note: HTTP/2 does not have a "chunked" transport layer header.
# It frames native streaming via individual sequential data frames over time.
def send_chunked_post(sock, conn, path, chunks):
    print("\n=== Chunked POST (HTTP/2 Streamed Frame Emulation) ===")
    body_bytes = "".join(chunks).encode()
    headers = {"Content-Type": "text/plain"}
    execute_h2_request(sock, conn, "POST", path, headers, body_bytes)

def send_half_chunked_post(sock, conn, path, chunks):
    print("\n=== Chunked POST (half-send Frame Emulation) ===")
    body_bytes = "".join(chunks).encode()
    headers = {"Content-Type": "text/plain"}
    execute_h2_request(sock, conn, "POST", path, headers, body_bytes, split_delay=2)

# ------------------------------
# Core Execution Wrapper
# ------------------------------
def runTest():
    # Setup raw TCP connection matching your working Curl parameters
    sock = socket.create_connection(("127.0.0.1", 8084))

    config = h2.config.H2Configuration(client_side=True)
    conn = h2.connection.H2Connection(config=config)

    # Send mandatory HTTP/2 24-byte client connection preface
    conn.initiate_connection()
    sock.sendall(conn.data_to_send())

    # Run test sequences over the multiplexed pipeline
    send_get(sock, conn, "/hello")

    send_urlencoded_post(sock, conn, "/submit-form", {"username": "john", "age": "25"})
    send_half_urlencoded_post(sock, conn, "/submit-form", {"username": "john", "age": "25"})

    send_multipart_post(sock, conn, "/upload", {"field1": "value1"}, {
        "file1": ("test.txt", "This is file content", "text/plain")
    })
    send_half_multipart_post(sock, conn, "/upload", {"field1": "value1"}, {
        "file1": ("test.txt", "This is file content", "text/plain")
    })

    send_binary_post(sock, conn, "/upload-bin", b"\x00\x01\x02\x03\x04\x05BinaryTestData")
    send_half_binary_post(sock, conn, "/upload-bin", b"\x00\x01\x02\x03\x04\x05BinaryTestData")

    send_chunked_post(sock, conn, "/transfer-chunked", ["chunk-A", "chunk-B", "chunk-C"])
    send_half_chunked_post(sock, conn, "/transfer-chunked", ["chunk-A", "chunk-B", "chunk-C", "chunk-D"])

    sock.close()
    print("\nAll requests completed.")

def getParsedNumber():
    if len(sys.argv) == 2:
        inp = int(sys.argv[1]) if (sys.argv[1]).isdigit() else 1
        return inp
    return 1

if __name__ == "__main__":
    n = getParsedNumber()
    for i in range(n):
        runTest()