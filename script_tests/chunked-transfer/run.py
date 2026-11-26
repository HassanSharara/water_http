import http.client

def send_chunked_post(connection, path, chunks):
    connection.putrequest("POST", path)
    connection.putheader("Transfer-Encoding", "chunked")
    connection.putheader("Content-Type", "text/plain")
    connection.putheader("Connection", "keep-alive")  # ensure persistence
    connection.endheaders()

    for chunk in chunks:
        chunk_bytes = chunk.encode()
        size = hex(len(chunk_bytes))[2:].encode()
        connection.send(size + b"\r\n")
        connection.send(chunk_bytes + b"\r\n")

    # End of chunks
    connection.send(b"0\r\n\r\n")

    # Read and print response
    response = connection.getresponse()
    print(f"Response: {response.status} {response.reason}")
    print(response.read().decode())
    response.close()  # read entire response body!


# Create one persistent connection
conn = http.client.HTTPConnection("localhost", 8084)

# First request
send_chunked_post(conn, "/transfer-chunked", ["first-", "request-", "chunks"])

# Second request using same connection
send_chunked_post(conn, "/transfer-chunked", ["second-", "request-", "chunks"])

conn.close()
