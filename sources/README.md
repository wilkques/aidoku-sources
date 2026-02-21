# How to use

```
git clone git@github.com:wilkques/aidoku-sources.git

cd /<document_root>/aidoku-sources/sources

docker build -t aidoku-sources .

docker run -d -it --rm \
--name aidoku-sources \
-v $(pwd)/../:/usr/src/app \
-v /etc/localtime:/etc/localtime:ro \
-v /etc/timezone:/etc/timezone:ro \
-w /usr/src/app \
-p 8080:8080 \
aidoku-sources

docker exec -it aidoku-sources /bin/sh
```

## Develop
```
cd /<document_root>/aidoku-sources/sources/<source name>

# run test
cargo test

# or
cargo test --release

#or
cargo test -- --nocapture
```

## Deployment
```
cd /<document_root>/aidoku-sources/sources/<source name>

aidoku package

cd /<document_root>/aidoku-sources

aidoku build sources/*/package.aix --name "Wilkques Sources"
```

## Docker command

```
# stop
docker ps -a

docker stop aidoku-sources
```