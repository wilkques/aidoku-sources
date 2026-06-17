#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod crypto;
mod fetch;
mod json;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, FilterValue, Manga, MangaPageResult, Page, Result, Source, Viewer,
    alloc::{String, Vec},
    imports::{js::WebView, std::current_date},
    prelude::*,
};

use crate::fetch::Fetch;
use crate::json::{ApiResponse, GenManga, ReadingApiResponse};
use crate::url::Url;

struct Happy;

impl Source for Happy {
    fn new() -> Self {
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        filters: Vec<FilterValue>,
    ) -> Result<MangaPageResult> {
        let base = settings::get_base_url();
        let url_obj = Url::filters(query.as_deref(), page, &filters)?;

        let data: ApiResponse = match &url_obj {
            // Search goes through the foreground request so the app can show the
            // interactive CF bypass dialog (background WebView can't pass an
            // interactive captcha). Retry once after the user clears it.
            Url::Search { query: q, .. } => {
                let body = format!("{{\"searchkey\":\"{}\",\"v\":\"v2.13\"}}", q);
                let make_request = || -> Result<String> {
                    Fetch::post(url_obj.to_string())?
                        .header("Content-Type", "application/json")
                        .header("Referer", &format!("{}/sssearch", base))
                        .header("X-Requested-With", "XMLHttpRequest")
                        .body(body.as_bytes())
                        .string()
                };
                let mut raw = make_request()?;
                if raw.starts_with("<!") {
                    raw = make_request()?;
                }
                serde_json::from_str(&raw).map_err(|_| aidoku::error!("search parse failed"))?
            }
            _ => {
                Fetch::get(url_obj.to_string())?
                    .header("Referer", &format!("{}/latest", base))
                    .json_owned()?
            }
        };

        GenManga::list(data)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        _needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        manga.viewer = Viewer::Webtoon;

        if needs_chapters {
            manga.chapters = Some(GenManga::chapters(&manga.key)?);
        }

        Ok(manga)
    }

    fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let base = settings::get_base_url();
        let json = fetch_reading(&base, &manga.key, &chapter.key)?;
        GenManga::chapter(json, &manga.key, &chapter.key)
    }
}

// happymh returns the correct tile order only when `avifSupport`/`webpSupport`
// cookies are present. URLSession ignores a manually set Cookie header, so we
// use a background WebView to write them into the shared cookie store via JS.
// CF bypass is handled at source entry via /sssearch.
fn fetch_reading(host: &str, manga_key: &str, chapter_key: &str) -> Result<ReadingApiResponse> {
    match fetch_reading_webview(host, manga_key, chapter_key) {
        Ok(resp) => Ok(resp),
        Err(_) => fetch_reading_direct(host, manga_key, chapter_key),
    }
}

fn fetch_reading_webview(
    host: &str,
    manga_key: &str,
    chapter_key: &str,
) -> Result<ReadingApiResponse> {
    let wv = WebView::new();
    let reader = format!("{}/mangaread/{}/{}", host, manga_key, chapter_key);
    wv.load_blocking(Fetch::get(reader)?)?;
    aidoku::imports::std::sleep(1);
    let _ = wv.eval("document.cookie='avifSupport=1;path=/';document.cookie='webpSupport=1;path=/';");
    let js = format!(
        "(function(){{var x=new XMLHttpRequest();\
         x.open('GET','/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300101&_t='+Date.now(),false);\
         x.setRequestHeader('x-requested-with','XMLHttpRequest');\
         x.send();return x.responseText;}})()",
        manga_key, chapter_key
    );
    let body = wv.eval(&js)?;
    serde_json::from_str::<ReadingApiResponse>(&body)
        .map_err(|_| aidoku::error!("webview reading parse failed (len={})", body.len()))
}

fn fetch_reading_direct(
    host: &str,
    manga_key: &str,
    chapter_key: &str,
) -> Result<ReadingApiResponse> {
    let ts = current_date() * 1000;
    let url = format!(
        "{}/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300101&_t={}",
        host, manga_key, chapter_key, ts
    );
    Fetch::get(url)?
        .header("Referer", &format!("{}/mangaread/{}/{}", host, manga_key, chapter_key))
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "application/json")
        .header("User-Agent", &settings::get_user_agent())
        .json_owned()
}

impl BaseUrlProvider for Happy {
    fn get_base_url(&self) -> Result<String> {
        Ok(settings::get_base_url())
    }
}

register_source!(
    Happy,
    BaseUrlProvider
);

#[cfg(test)]
mod test;
