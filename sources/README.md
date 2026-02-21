# How to use

```
git clone git@github.com:wilkques/aidoku-sources.git

cd /<document_root>/aidoku-sources/sources

docker build -t aidoku-sources .

docker run -d -it --rm \
--name aidoku-rs \
-v $(pwd)/../:/usr/src/app \
-v /etc/localtime:/etc/localtime:ro \
-v /etc/timezone:/etc/timezone:ro \
-w /usr/src/app \
aidoku-sources

docker exec -it aidoku-sources /bin/sh
```

## develop
```
cd /<document_root>/aidoku-sources/sources/<source name>

# run test
cargo test

# or
cargo test --release

#or
cargo test -- --nocapture
```

## deployment
```
cd /<document_root>/aidoku-sources/sources/<source name>

aidoku package

cd /<document_root>/aidoku-sources

aidoku build sources/*/package.aix --name "Wilkques Sources"
```

docker command

```
# stop
docker ps -a

docker stop aidoku-sources
```