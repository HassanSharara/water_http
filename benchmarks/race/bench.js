
const http = require('http');
const fs = require('fs');

const servers_urls  = [
    'http://localhost:8084',
    'http://localhost:8080',
];

async function measureLatency(url) {
    return new Promise((resolve) => {
        const latencies = [];

        const makeRequest = async (attempt) => {
            const start = Date.now();

            await http.get(url, (res) => {
                const latency = Date.now() - start;
                latencies.push(latency);

                res.on('data', () => {
                    // Consume the response data to complete the request
                });

                res.on('end', async () => {
                    if (attempt < 20) {
                        // await new Promise(resolve => setTimeout(resolve, 20));
                        makeRequest(attempt + 1);
                    } else {
                        resolve(latencies);
                    }
                });
            }).on('error', (err) => {
                console.error(`Request to ${url} failed:`, err.message);
                latencies.push(null); // Push null to indicate a failed request

                if (attempt < 20) {
                    makeRequest(attempt + 1);
                } else {
                    resolve(latencies);
                }
            });
        };

        makeRequest(1);
    });
}

async function runLatencyTest() {
    const allLatenciesMap = new Map();

    // Run the benchmarking 20 times
    for (let run = 1; run <= 20; run++) {
        console.log(`Running benchmarking iteration ${run}...`);
        for (const path of servers_urls) {
            console.log(`Measuring latency for ${path}...`);
            const latencies = await measureLatency(path);
            if (!allLatenciesMap.has(path)) {
                allLatenciesMap.set(path, []);
            }
            allLatenciesMap.get(path).push(...latencies);
        }
    }

    const results = [];

    // Compute total and average latencies
    for (const [path, latencies] of allLatenciesMap.entries()) {
        const totalLatency = latencies.reduce((sum, latency) => sum + (latency || 0), 0);
        const averageLatency = latencies.filter(latency => latency !== null).reduce((sum, latency) => sum + latency, 0) / latencies.filter(latency => latency !== null).length;

        results.push(`Path: ${path}\nLatencies: ${latencies.join(', ')}\nTotal Latency: ${totalLatency} ms\nAverage Latency: ${averageLatency.toFixed(2)} ms\n`);
    }

    results.splice(0, 0, '\n');
    // Write results to a text file
    fs.appendFileSync('latency_results.txt', results.join('\n'));

    // Determine the winner (path with the lowest average latency)
    const winner = [...allLatenciesMap.entries()].reduce((lowest, [path, latencies]) => {
        const avgLatency = latencies.filter(latency => latency !== null).reduce((sum, latency) => sum + latency, 0) / latencies.filter(latency => latency !== null).length;
        return avgLatency < lowest.avgLatency ? { path, avgLatency } : lowest;
    }, { path: '', avgLatency: Infinity });

    // Determine the average loser (path with the highest average latency)
    const loser = [...allLatenciesMap.entries()].reduce((highest, [path, latencies]) => {
        const avgLatency = latencies.filter(latency => latency !== null).reduce((sum, latency) => sum + latency, 0) / latencies.filter(latency => latency !== null).length;
        return avgLatency > highest.avgLatency ? { path, avgLatency } : highest;
    }, { path: '', avgLatency: -Infinity });

    // Append the winner and loser to the end of the file
    const resultSummary = `\nWinner: ${winner.path}\nAverage Latency: ${winner.avgLatency.toFixed(2)} ms\n\nAverage Loser: ${loser.path}\nAverage Latency: ${loser.avgLatency.toFixed(2)} ms\n`;
    fs.appendFileSync('latency_results.txt', resultSummary);
    console.log('Latency results, winner, and average loser have been written to latency_results.txt');
}

async function run(){
    for(i=0; i < 10 ; i++){
        fs.appendFileSync('latency_results.txt',`\n Round ${i+1} `);
        await runLatencyTest();
    }
}

run();