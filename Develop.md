# How to use

```
docker build -t aidoku-rs .

docker run -d -it --rm -v $(pwd)/../:/usr/src/app -w /usr/src/app aidoku-rs

rustup target add wasm32-unknown-unknown
```

## develop
```
cd /<document_root>/sources/<source name>

# run test
cargo test
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

docker stop <Container ID>

docker rm <Container ID>
```