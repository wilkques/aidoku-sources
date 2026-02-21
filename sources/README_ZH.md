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

在主機的瀏覽器上前往 `http://<IP 位址或 localhost>:8080/index.min.json`。確認數據能正常載入後，回到 iPhone 上的 Aidoku，將 `http://<IP 位址或 localhost>:8080/index.min.json` 加入到你的**來源清單（Source List）**中。

注意： 你需要在防火牆設定中開放 8080 埠（Port）以允許連線（請參考 [開啟防火牆連接埠的步驟](#steps-to-open-a-firewall-port)）。

你可以使用 `aidoku::println!("URL={} Query={:?} Filter={:?}", &url, &query, &filters);` 來在 Aidoku 日誌中查看結果（路徑：設定 > 顯示日誌）。

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

## Steps to Open a Firewall Port

- Windows
    按下 Win + R 鍵，輸入 wf.msc 並按下 Enter。

    點選左側的 輸入規則 (Inbound Rules)，接著在右側操作面板點選 新增規則... (New Rule...)。

    規則類型選擇 連接埠 (Port)。

    選擇 TCP，並在「特定本機埠」中輸入 8080。

    選擇 允許連線 (Allow the connection)。

    勾選套用此規則的網路設定檔類型（網域 Domain、專用 Private、公用 Public）。

    為此規則命名（例如：Aidoku Local Server），然後點擊 完成 (Finish)。