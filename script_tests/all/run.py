import http.client
import uuid
import mimetypes
import time
import sys
# ------------------------------
# GET Request
# ------------------------------
def send_get(connection, path):
    print("\n=== GET Request ===")
    connection.putrequest("GET", path)
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

# ------------------------------
# x-www-form-urlencoded POST
# ------------------------------
def send_urlencoded_post(connection, path, data):
    print("\n=== x-www-form-urlencoded POST ===")
    body_bytes = "&".join(f"{k}={v}" for k, v in data.items()).encode()
    connection.putrequest("POST", path)
    connection.putheader("Content-Type", "application/x-www-form-urlencoded")
    connection.putheader("Content-Length", str(len(body_bytes)))
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    connection.send(body_bytes)
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

def send_half_urlencoded_post(connection, path, data):
    print("\n=== x-www-form-urlencoded POST (half-send) ===")
    body_bytes = "&".join(f"{k}={v}" for k, v in data.items()).encode()
    half = len(body_bytes) // 2
    connection.putrequest("POST", path)
    connection.putheader("Content-Type", "application/x-www-form-urlencoded")
    connection.putheader("Content-Length", str(len(body_bytes)))
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    connection.send(body_bytes[:half])
    print("Sent first half:", body_bytes[:half])
    time.sleep(2)
    connection.send(body_bytes[half:])
    print("Sent second half:", body_bytes[half:])
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

# ------------------------------
# Multipart/form-data POST
# ------------------------------
def send_multipart_post(connection, path, fields, files=None):
    print("\n=== Multipart/form-data POST ===")
    boundary = uuid.uuid4().hex
    lines = []

    # Fields
    for name, value in fields.items():
        lines.extend([
            f"--{boundary}",
            f'Content-Disposition: form-data; name="{name}"',
            "",
            value,
        ])
    # Files
    if files:
        for field_name, (filename, file_content, content_type) in files.items():
            if content_type is None:
                content_type = mimetypes.guess_type(filename)[0] or "application/octet-stream"
            lines.extend([
                f"--{boundary}",
                f'Content-Disposition: form-data; name="{field_name}"; filename="{filename}"',
                f"Content-Type: {content_type}",
                "",
                file_content,
            ])
    # End boundary
    lines.append(f"--{boundary}--")
    lines.append("")
    body_bytes = "\r\n".join(lines).encode()

    connection.putrequest("POST", path)
    connection.putheader("Content-Type", f"multipart/form-data; boundary={boundary}")
    connection.putheader("Content-Length", str(len(body_bytes)))
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    connection.send(body_bytes)
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

def send_half_multipart_post(connection, path, fields, files=None):
    print("\n=== Multipart/form-data POST (half-send) ===")
    boundary = uuid.uuid4().hex
    lines = []

    for name, value in fields.items():
        lines.extend([
            f"--{boundary}",
            f'Content-Disposition: form-data; name="{name}"',
            "",
            value,
        ])
    if files:
        for field_name, (filename, file_content, content_type) in files.items():
            if content_type is None:
                content_type = mimetypes.guess_type(filename)[0] or "application/octet-stream"
            lines.extend([
                f"--{boundary}",
                f'Content-Disposition: form-data; name="{field_name}"; filename="{filename}"',
                f"Content-Type: {content_type}",
                "",
                file_content,
            ])
    lines.append(f"--{boundary}--")
    lines.append("")
    body_bytes = "\r\n".join(lines).encode()
    half = len(body_bytes) // 2

    connection.putrequest("POST", path)
    connection.putheader("Content-Type", f"multipart/form-data; boundary={boundary}")
    connection.putheader("Content-Length", str(len(body_bytes)))
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    connection.send(body_bytes[:half])
    print("Sent first half of multipart")
    time.sleep(2)
    connection.send(body_bytes[half:])
    print("Sent second half of multipart")
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

# ------------------------------
# Binary POST
# ------------------------------
def send_binary_post(connection, path, data_bytes):
    print("\n=== Binary POST ===")
    connection.putrequest("POST", path)
    connection.putheader("Content-Type", "application/octet-stream")
    connection.putheader("Content-Length", str(len(data_bytes)))
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    connection.send(data_bytes)
    print("getting response")
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

def send_half_binary_post(connection, path, data_bytes):
    print("\n=== Binary POST (half-send) ===")
    half = len(data_bytes) // 2
    connection.putrequest("POST", path)
    connection.putheader("Content-Type", "application/octet-stream")
    connection.putheader("Content-Length", str(len(data_bytes)))
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    connection.send(data_bytes[:half])
    print("Sent first half of binary")
    time.sleep(2)
    connection.send(data_bytes[half:])
    print("Sent second half of binary")
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

# ------------------------------
# Chunked POST
# ------------------------------
def send_chunked_post(connection, path, chunks):
    print("\n=== Chunked POST ===")
    connection.putrequest("POST", path)
    connection.putheader("Transfer-Encoding", "chunked")
    connection.putheader("Content-Type", "text/plain")
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    for chunk in chunks:
        chunk_bytes = chunk.encode()
        size = hex(len(chunk_bytes))[2:].encode()
        connection.send(size + b"\r\n")
        connection.send(chunk_bytes + b"\r\n")
    connection.send(b"0\r\n\r\n")
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()

def send_half_chunked_post(connection, path, chunks):
    print("\n=== Chunked POST (half-send) ===")
    connection.putrequest("POST", path)
    connection.putheader("Transfer-Encoding", "chunked")
    connection.putheader("Content-Type", "text/plain")
    connection.putheader("Connection", "keep-alive")
    connection.endheaders()
    half = len(chunks) // 2
    first_half = chunks[:half]
    second_half = chunks[half:]

    for chunk in first_half:
        chunk_bytes = chunk.encode()
        size = hex(len(chunk_bytes))[2:].encode()
        connection.send(size + b"\r\n")
        connection.send(chunk_bytes + b"\r\n")
        print(f"Sent first half chunk: {chunk}")
        time.sleep(1)

    print("Pausing before sending second half...")
    time.sleep(2)

    for chunk in second_half:
        chunk_bytes = chunk.encode()
        size = hex(len(chunk_bytes))[2:].encode()
        connection.send(size + b"\r\n")
        connection.send(chunk_bytes + b"\r\n")
        print(f"Sent second half chunk: {chunk}")

    connection.send(b"0\r\n\r\n")
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()


def runTest():
        conn = http.client.HTTPConnection("localhost", 8084)

        send_get(conn, "/hello")

        send_urlencoded_post(conn, "/submit-form", {"username": "john", "age": "25"})
        send_half_urlencoded_post(conn, "/submit-form", {"username": "john", "age": "25"})

        send_multipart_post(conn, "/upload", {"field1": "value1"}, {
            "file1": ("test.txt", "This is file content", "text/plain")
        })
        send_half_multipart_post(conn, "/upload", {"field1": "value1"}, {
            "file1": ("test.txt", "This is file content", "text/plain")
        })

        send_binary_post(conn, "/upload-bin", b"\x00\x01\x02\x03\x04\x05BinaryTestData")
        send_half_binary_post(conn, "/upload-bin", b"\x00\x01\x02\x03\x04\x05BinaryTestData")

        send_chunked_post(conn, "/transfer-chunked", ["chunk-A", "chunk-B", "chunk-C"])
        send_half_chunked_post(conn, "/transfer-chunked", ["chunk-A", "chunk-B", "chunk-C", "chunk-D"])

        conn.close()
        print("\nAll requests completed.")


def getParsedNumber():
    if len(sys.argv) == 2 :
        inp = int(sys.argv[1]) if (sys.argv[1]).isdigit() else 1
        return inp

    return 1

# ------------------------------
# MAIN TEST SEQUENCE
# ------------------------------
if __name__ == "__main__":
    n = getParsedNumber()
    for i in range(n):
        runTest()

