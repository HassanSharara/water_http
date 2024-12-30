
to use wrk benchmarks tool 
we are going to use docker 
so 

- install docker 
- run command 
```shell 
docker pull skandyla/wrk
```

- then start wrk image
```shell
 docker run -it --rm --network host --entrypoint=/bin/sh skandyla/wrk
```

- then start you water_http server on port 8084 or any port you want
- then run wrk benchmarking 
```shell
wrk -t12 -c400 -d30s http://host.docker.internal:8084
```

-t12 : stands for creating 12 threads

-c400: stands for creating 400 connections

-d30s: stands for test for 30 seconds