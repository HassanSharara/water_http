
# Benching Water_http using autoCannon Benching tool

- let`s start with installing [npm](https://nodejs.org/en/download/package-manager)
- now to install autocannon globally 
```shell
 npm install -g autocannon
 #// or 
 npm i -g autocannon
```
- after installation autocannon run your server with desired endpoint
for example localhost:8084
- run autocannon benchmarks on localhost:8084
```shell
autocannon -c 100 -d 30 -p 10  http://localhost:8084
```
notice that the results would be different from one computer to another 
depending on resources ( ram , cpu , cores ..etc ),
and also depending on the current underusing applications and services
so to get the right results you could 
run two benchmarks with tow different types of frameworks
to figure which one is the faster


now the results will show on terminal or cmd outputs