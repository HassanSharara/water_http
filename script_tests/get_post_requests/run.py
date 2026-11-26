import http.client
import json
import uuid
import urllib.parse

HOST = "localhost"
PORT = 8084



def make_request(method, path, headers=None, body=None):
    try:
        connection = http.client.HTTPConnection(HOST, PORT)

        # Prepare headers
        headers = headers or {}
        if body is not None and "Content-Length" not in {k.lower(): v for k, v in headers.items()}:
            if not isinstance(body, bytes):
                body = body.encode()
            headers["Content-Length"] = str(len(body))
        elif body is not None and isinstance(body, str):
            body = body.encode()

        # Start the request
        connection.putrequest(method, path)

        for key, value in headers.items():
            connection.putheader(key, value)

        connection.endheaders()

        if body:
            connection.send(body)

        response = connection.getresponse()
        print(f"\n[{method}] {path}")
        print(f"Status: {response.status}")
        print(f"Response: {response.read().decode()}")
        connection.close()
    except ConnectionRefusedError:
        print(f"Error: Could not connect to server at {HOST}:{PORT}")
    except http.client.HTTPException as e:
        print(f"HTTP error: {e}")

# 1. GET Request
make_request("GET", "/")


# 2. POST JSON
make_request(
    "POST", "/",
    headers={"Content-Type": "application/json"},
    body=json.dumps({"name": "Alice", "age": 30})
)


# 3. POST Form URL-Encoded
make_request(
    "POST", "/",
    headers={"Content-Type": "application/x-www-form-urlencoded"},
    body=urllib.parse.urlencode({"username": "admin", "password": "1234"})
)


# 4. POST Multipart Form-Data (simulate file upload)
boundary = str(uuid.uuid4())
multipart_body = (
    f"--{boundary}\r\n"
    'Content-Disposition: form-data; name="field1"\r\n\r\n'
    "value1\r\n"
    f"--{boundary}\r\n"
    'Content-Disposition: form-data; name="file"; filename="test.txt"\r\n'
    "Content-Type: text/plain\r\n\r\n"
    "This is file content.\r\n"
    f"--{boundary}--\r\n"
)
make_request(
    "POST", "/",
    headers={
        "Content-Type": f"multipart/form-data; boundary={boundary}",
        "Content-Length": str(len(multipart_body))
    },
    body=multipart_body
)


# 5. PUT Request
make_request("PUT", "/", body="Updated content")


# 6. DELETE Request
make_request("DELETE", "/")


# 7. HEAD Request
make_request("HEAD", "/")


# 8. PATCH Request
make_request("PATCH", "/", body="patched content")


# 9. OPTIONS Request
make_request("OPTIONS", "/")


# 10. TRACE Request
make_request("TRACE", "/")
