// node_http_test.js
const net = require("net");
const crypto = require("crypto");

// ------------------------------
// Helpers
// ------------------------------
function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

function send(socket, data) {
    socket.write(data);
}

function readResponse(socket) {
    return new Promise(resolve => {
        let data = "";
        const handler = chunk => {
            data += chunk.toString();
            if (data.includes("\r\n\r\n")) {
                socket.off("data", handler);
                console.log(data);
                resolve();
            }
        };
        socket.on("data", handler);
    });
}

// ------------------------------
// GET
// ------------------------------
async function sendGet(socket, path) {
    console.log("\n=== GET Request ===");
    send(socket,
        `GET ${path} HTTP/1.1\r
Host: localhost\r
Connection: keep-alive\r
\r
`);
    await readResponse(socket);
}

// ------------------------------
// x-www-form-urlencoded POST
// ------------------------------
async function sendUrlEncodedPost(socket, path, data) {
    console.log("\n=== x-www-form-urlencoded POST ===");
    const body = new URLSearchParams(data).toString();

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Content-Type: application/x-www-form-urlencoded\r
Content-Length: ${Buffer.byteLength(body)}\r
Connection: keep-alive\r
\r
${body}`);
    await readResponse(socket);
}

async function sendHalfUrlEncodedPost(socket, path, data) {
    console.log("\n=== x-www-form-urlencoded POST (half-send) ===");
    const body = new URLSearchParams(data).toString();
    const half = Math.floor(body.length / 2);

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Content-Type: application/x-www-form-urlencoded\r
Content-Length: ${Buffer.byteLength(body)}\r
Connection: keep-alive\r
\r
${body.slice(0, half)}`);

    console.log("Sent first half");
    await sleep(2000);

    send(socket, body.slice(half));
    console.log("Sent second half");

    await readResponse(socket);
}

// ------------------------------
// Multipart/form-data POST
// ------------------------------
function buildMultipart(fields, files) {
    const boundary = crypto.randomUUID().replace(/-/g, "");
    let body = "";

    for (const k in fields) {
        body +=
            `--${boundary}\r
Content-Disposition: form-data; name="${k}"\r
\r
${fields[k]}\r
`;
    }

    if (files) {
        for (const k in files) {
            const { filename, content, type } = files[k];
            body +=
                `--${boundary}\r
Content-Disposition: form-data; name="${k}"; filename="${filename}"\r
Content-Type: ${type || "application/octet-stream"}\r
\r
${content}\r
`;
        }
    }

    body += `--${boundary}--\r\n`;
    return { body, boundary };
}

async function sendMultipartPost(socket, path, fields, files) {
    console.log("\n=== Multipart/form-data POST ===");
    const { body, boundary } = buildMultipart(fields, files);

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Content-Type: multipart/form-data; boundary=${boundary}\r
Content-Length: ${Buffer.byteLength(body)}\r
Connection: keep-alive\r
\r
${body}`);

    await readResponse(socket);
}

async function sendHalfMultipartPost(socket, path, fields, files) {
    console.log("\n=== Multipart/form-data POST (half-send) ===");
    const { body, boundary } = buildMultipart(fields, files);
    const half = Math.floor(body.length / 2);

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Content-Type: multipart/form-data; boundary=${boundary}\r
Content-Length: ${Buffer.byteLength(body)}\r
Connection: keep-alive\r
\r
${body.slice(0, half)}`);

    await sleep(2000);
    send(socket, body.slice(half));

    await readResponse(socket);
}

// ------------------------------
// Binary POST
// ------------------------------
async function sendBinaryPost(socket, path, buffer) {
    console.log("\n=== Binary POST ===");

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Content-Type: application/octet-stream\r
Content-Length: ${buffer.length}\r
Connection: keep-alive\r
\r
`);
    send(socket, buffer);
    await readResponse(socket);
}

async function sendHalfBinaryPost(socket, path, buffer) {
    console.log("\n=== Binary POST (half-send) ===");
    const half = Math.floor(buffer.length / 2);

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Content-Type: application/octet-stream\r
Content-Length: ${buffer.length}\r
Connection: keep-alive\r
\r
`);
    send(socket, buffer.slice(0, half));
    await sleep(2000);
    send(socket, buffer.slice(half));

    await readResponse(socket);
}

// ------------------------------
// Chunked POST
// ------------------------------
async function sendChunkedPost(socket, path, chunks) {
    console.log("\n=== Chunked POST ===");

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Transfer-Encoding: chunked\r
Content-Type: text/plain\r
Connection: keep-alive\r
\r
`);

    for (const chunk of chunks) {
        send(socket, chunk.length.toString(16) + "\r\n");
        send(socket, chunk + "\r\n");
    }

    send(socket, "0\r\n\r\n");
    await readResponse(socket);
}

async function sendHalfChunkedPost(socket, path, chunks) {
    console.log("\n=== Chunked POST (half-send) ===");

    send(socket,
        `POST ${path} HTTP/1.1\r
Host: localhost\r
Transfer-Encoding: chunked\r
Content-Type: text/plain\r
Connection: keep-alive\r
\r
`);

    const half = Math.floor(chunks.length / 2);

    for (let i = 0; i < half; i++) {
        send(socket, chunks[i].length.toString(16) + "\r\n");
        send(socket, chunks[i] + "\r\n");
        await sleep(1000);
    }

    await sleep(2000);

    for (let i = half; i < chunks.length; i++) {
        send(socket, chunks[i].length.toString(16) + "\r\n");
        send(socket, chunks[i] + "\r\n");
    }

    send(socket, "0\r\n\r\n");
    await readResponse(socket);
}

// ------------------------------
// MAIN
// ------------------------------
(async () => {
    const socket = net.createConnection(8084, "localhost");

    await sendGet(socket, "/hello");

    await sendUrlEncodedPost(socket, "/submit-form", { username: "john", age: "25" });
    await sendHalfUrlEncodedPost(socket, "/submit-form", { username: "john", age: "25" });

    await sendMultipartPost(socket, "/upload",
        { field1: "value1" },
        { file1: { filename: "script_tests.txt", content: "This is file content", type: "text/plain" } }
    );

    await sendHalfMultipartPost(socket, "/upload",
        { field1: "value1" },
        { file1: { filename: "script_tests.txt", content: "This is file content", type: "text/plain" } }
    );

    const bin = Buffer.from([0,1,2,3,4,5,66,105,110,97,114,121]);
    await sendBinaryPost(socket, "/upload-bin", bin);
    await sendHalfBinaryPost(socket, "/upload-bin", bin);

    await sendChunkedPost(socket, "/transfer-chunked", ["chunk-A", "chunk-B", "chunk-C"]);
    await sendHalfChunkedPost(socket, "/transfer-chunked", ["chunk-A", "chunk-B", "chunk-C", "chunk-D"]);

    socket.end();
    console.log("\nAll requests completed.");
})();
