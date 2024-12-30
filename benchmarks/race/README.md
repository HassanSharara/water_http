
# Benchmarks using race 
inside bench.js 
just update server_urls
to your custom urls
```javascript
const servers_urls = [
    'http://localhost:8084',// for your first server
    'http://localhost:8080', // for your second one
];
```

then we need to know who is the faster
so you would need to 
run npm start 
 notice that you need to listen to these ports 
before running 
```shell
npm start
```
then latency_results.txt file would be generated containing all the rounds 