# How to use

```
docker build -t aidoku-rs .

docker run -d -it --rm \
--name aidoku-rs \
-v $(pwd)/../:/usr/src/app \
-v /etc/localtime:/etc/localtime:ro \
-v /etc/timezone:/etc/timezone:ro \
-w /usr/src/app \
aidoku-rs

docker exec -it aidoku-rs /bin/sh
```

## develop
```
cd /<document_root>/sources/<source name>

# run test
cargo test

# or
cargo test --release

#or
cargo test -- --nocapture
```

## deployment
```
cd /<document_root>/sources/<source name>

aidoku package

cd ../..

aidoku build sources/*/package.aix --name "Wilkques Sources"
```

docker command

```
# stop
docker ps -a

docker stop <Container ID/name>

docker rm <Container ID/name>
```