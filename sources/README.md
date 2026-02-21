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

aidoku package

aidoku build package.aix --name "Wilkques Sources"

cd public/

aidoku serve sources/zh.jmtt-v2.aix
```

Navigate to `http://<IP_ADDRESS or localhost>:8080/index.min.json` on your host's browser. Once you've confirmed the data is loading, return to Aidoku on your iPhone and add `http://<IP_ADDRESS or localhost>:8080/index.min.json` to your Source List.

Note: Need to open port `8080` in your firewall settings to allow the connection (see [Steps to Open a Firewall Port](#steps-to-open-a-firewall-port)).

You can use `aidoku::println!("URL={} Query={:?} Filter={:?}", &url, &query, &filters);` to view the results in the Aidoku logs (Settings > Show Logs).

## Test
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

### Steps to Open a Firewall Port

- Windows
    Press Win + R, type wf.msc, and hit Enter.

    Go to Inbound Rules (or Outbound, depending on your needs) and select New Rule...

    Select Port as the rule type.

    Choose TCP and specify port 8080 in "Specific local ports."

    Select Allow the connection.

    Choose the Network profile types (Domain, Private, Public) that this rule applies to.

    Name the rule and click Finish.