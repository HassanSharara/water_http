
import http.client

try:
    # Establish connection
    connection = http.client.HTTPConnection("localhost", 8084)

    # Start POST request
    connection.putrequest("GET", "/")
    connection.endheaders()
    response = connection.getresponse()
    print(f"Status: {response.status}")
    print(f"Response: {response.read().decode()}")
    connection.close()
except ConnectionRefusedError:
    print("Error: Could not connect to server at localhost:8084")
except http.client.HTTPException as e:
    print(f"HTTP error: {e}")

