use aidoku::{
    GroupSetting, LoginMethod, LoginSetting, Setting,
    alloc::{Vec, format, string::String, vec},
    imports::defaults::{DefaultValue, defaults_get, defaults_set},
};

const BASE_URL_KEY: &str = "url";

pub fn get_base_url() -> String {
    let mut base_url = defaults_get::<String>(BASE_URL_KEY).unwrap_or_default();

    if base_url.is_empty() {
        let default_base_url = "https://bakamh.ru";

        defaults_set(
            BASE_URL_KEY,
            DefaultValue::String(String::from(default_base_url)),
        );

        base_url = String::from(default_base_url);
    }

    base_url
}

// Cloudflare 驗證按鈕要開「目前選用的網址」，但 settings.json 只能寫死網址：web 登入視窗
// 只讀 `url`，`urlKey` 只有 OAuth 會用（Aidoku `SettingView.swift`），所以改成動態產生。
// app 每次打開來源設定頁都會重新呼叫 `get_dynamic_settings`，切換網址後重開設定頁就會更新。
pub fn get_cf_settings() -> Vec<Setting> {
    let base_url = get_base_url();
    let host = cf_host(&base_url);

    let login = LoginSetting {
        // 每個網域各自一把 key：clearance 是分網域的，換網址後不該沿用「已驗證」的狀態
        key: format!("cfVerify_{}", host.replace('.', "_")).into(),
        title: format!("Cloudflare 驗證（{}）", host).into(),
        method: LoginMethod::Web,
        url: Some(format!("{}/", base_url).into()),
        logout_title: Some(format!("清除 Cloudflare 驗證（{}）", host).into()),
        // 必須是 false：Rust 端是 `bool`，app 端是 `Bool?`，postcard 解碼時 `true`（0x01）會被
        // 當成 Option 的 Some 標記、再多讀一個位元組，結果設定頁「解碼錯誤」。`false`（0x00）剛好
        // 等於 None 才解得開。所以「清除」只會重設按鈕狀態，不會刪 cookie。pkce/use_email 同理。
        clear_cookies_on_log_out: false,
        ..Default::default()
    };

    let group = GroupSetting {
        key: "cfGroup".into(),
        title: "Cloudflare".into(),
        footer: Some(
            "全站受 Cloudflare 保護。出現「Cloudflare 驗證已失效」時，點上面的按鈕，在開啟的頁面完成驗證，看到網站內容就代表成功，關閉視窗後再重試。換網址後要重新打開這個設定頁，再驗證一次。".into(),
        ),
        items: vec![login.into()],
        ..Default::default()
    };

    vec![group.into()]
}

/// "https://bakamh.ru" -> "bakamh.ru"
pub fn cf_host(base_url: &str) -> &str {
    base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
}
