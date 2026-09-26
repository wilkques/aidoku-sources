use aidoku::{
    GroupSetting, LoginMethod, LoginSetting, SelectSetting, Setting,
    alloc::{Vec, format, string::String, vec},
    imports::defaults::{DefaultValue, defaults_get, defaults_set},
};

const BASE_URL_KEY: &str = "url";
const DEFAULT_BASE_URL: &str = "https://bakamh.com";
const BASE_URLS: [&str; 3] = ["https://bakamh.com", "https://bakamh.ru", "https://baka3.me"];

pub fn get_base_url() -> String {
    let mut base_url = defaults_get::<String>(BASE_URL_KEY).unwrap_or_default();

    if base_url.is_empty() {
        defaults_set(
            BASE_URL_KEY,
            DefaultValue::String(String::from(DEFAULT_BASE_URL)),
        );

        base_url = String::from(DEFAULT_BASE_URL);
    }

    base_url
}

// Cloudflare 驗證按鈕要開「目前選用的網址」，但 settings.json 只能寫死網址：web 登入視窗
// 只讀 `url`，`urlKey` 只有 OAuth 會用（Aidoku `SettingView.swift`），所以改成動態產生。
// 設定頁收到 `refresh-settings` 通知時會重新呼叫 `get_dynamic_settings`。
pub fn get_cf_settings() -> Vec<Setting> {
    let base_url = get_base_url();
    let host = cf_host(&base_url);

    // 網址選單自己出，不用 source.json 的 `allowsBaseUrlSelect`：內建那個選單只有
    // `refreshes: ["content"]`（AidokuRunner `Source.swift`），切換後設定頁不會重新載入，
    // 下面的驗證 / 登入按鈕就一直停在舊網址。這裡多加 "settings" 讓按鈕跟著換。
    let url_select = SelectSetting {
        key: BASE_URL_KEY.into(),
        title: "網址".into(),
        refreshes: Some(vec!["content".into(), "listings".into(), "settings".into()]),
        values: BASE_URLS.iter().map(|url| (*url).into()).collect(),
        default: Some(DEFAULT_BASE_URL.into()),
        ..Default::default()
    };

    let url_group = GroupSetting {
        key: "urlGroup".into(),
        title: "網址".into(),
        items: vec![url_select.into()],
        ..Default::default()
    };

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
            "全站受 Cloudflare 保護。出現「Cloudflare 驗證已失效」時，點上面的按鈕，在開啟的頁面完成驗證，看到網站內容就代表成功，關閉視窗後再重試。換網址後要再驗證一次。".into(),
        ),
        items: vec![login.into()],
        ..Default::default()
    };

    // 帳號登入也是同一套機制：網站的登入是首頁上的 JS 彈窗，web 登入視窗是完整的 WKWebView，
    // 在裡面登入後 WordPress 的登入 cookie 會落在同一個 per-source store，`Web` 的 XHR 自動帶上。
    let account = LoginSetting {
        // 登入狀態也是分網域的（cookie 綁網域）
        key: format!("account_{}", host.replace('.', "_")).into(),
        title: format!("登入帳號（{}）", host).into(),
        method: LoginMethod::Web,
        url: Some(format!("{}/", base_url).into()),
        logout_title: Some(format!("重設登入狀態（{}）", host).into()),
        // 同上，必須是 false
        clear_cookies_on_log_out: false,
        ..Default::default()
    };

    let account_group = GroupSetting {
        key: "accountGroup".into(),
        title: "帳號".into(),
        footer: Some(
            "部分漫畫要登入才能看。點上面的按鈕會打開網站首頁，用網站右上角的登入功能登入後關閉視窗即可。按鈕的「重設登入狀態」不會真的登出，要登出請在視窗裡用網站的登出功能。換網址後要重新登入。".into(),
        ),
        items: vec![account.into()],
        ..Default::default()
    };

    vec![url_group.into(), group.into(), account_group.into()]
}

/// "https://bakamh.ru" -> "bakamh.ru"
pub fn cf_host(base_url: &str) -> &str {
    base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
}
