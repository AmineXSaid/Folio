//! Link cards: telling a web address from other pasted text, and fetching a
//! page's title. Runs off the UI thread; a page that doesn't answer in a few
//! seconds just keeps its address as the title.

use std::io::Read;
use std::time::Duration;

/// A single `http(s)://` address with nothing else around it.
pub fn is_url(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("https://") || t.starts_with("http://"))
        && !t.contains(char::is_whitespace)
        && t.len() > 10
        && site(t).contains('.')
}

/// The host without `www.`: `https://www.rust-lang.org/learn` → `rust-lang.org`.
pub fn site(url: &str) -> String {
    let rest = url.split_once("://").map(|x| x.1).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit_once('@').map(|x| x.1).unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    host.strip_prefix("www.").unwrap_or(host).to_lowercase()
}

/// What a link card shows: the page's title, and its preview picture and
/// site icon as files (bytes and extension), each if there is one.
#[derive(Default)]
pub struct Page {
    pub title: Option<String>,
    pub image: Option<(Vec<u8>, String)>,
    pub icon: Option<(Vec<u8>, String)>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .user_agent("Mozilla/5.0 (Folio link preview)")
        .build()
}

/// Fetches the page, then its `og:image` and its icon (best effort).
pub fn fetch_page(url: &str) -> Page {
    let agent = agent();
    let Ok(resp) = agent.get(url).call() else { return Page::default() };
    let mut html = String::new();
    if resp.into_reader().take(512 * 1024).read_to_string(&mut html).is_err() {
        return Page::default();
    }
    let image = meta_content(&html, "og:image").map(|u| absolute(url, &u)).and_then(|u| download(&agent, &u, 6 << 20));
    let icon_url = icon_href(&html).map(|u| absolute(url, &u)).unwrap_or_else(|| absolute(url, "/favicon.ico"));
    let icon = download(&agent, &icon_url, 512 << 10);
    Page { title: title_of(&html), image, icon }
}

/// A picture's bytes and extension, if it's a picture Folio can show.
fn download(agent: &ureq::Agent, url: &str, limit: u64) -> Option<(Vec<u8>, String)> {
    let resp = agent.get(url).call().ok()?;
    let kind = resp.content_type().to_lowercase();
    let mut bytes = Vec::new();
    resp.into_reader().take(limit).read_to_end(&mut bytes).ok()?;
    let ext = match () {
        _ if kind.contains("png") || bytes.starts_with(b"\x89PNG") => "png",
        _ if kind.contains("jpeg") || kind.contains("jpg") || bytes.starts_with(&[0xff, 0xd8]) => "jpg",
        _ if kind.contains("gif") || bytes.starts_with(b"GIF8") => "gif",
        _ if kind.contains("webp") => "webp",
        _ if kind.contains("svg") || url.ends_with(".svg") => "svg",
        _ => return None,
    };
    (!bytes.is_empty()).then(|| (bytes, ext.to_string()))
}

/// `<meta property="NAME" content="…">`, either attribute order.
fn meta_content(html: &str, name: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let key = format!("\"{name}\"");
    let i = lower.find(&key)?;
    let start = lower[..i].rfind('<')?;
    let end = start + lower[start..].find('>')?;
    let tag = &html[start..end];
    let c = tag.to_lowercase().find("content=")? + 8;
    let q = tag[c..].chars().next()?;
    let rest = &tag[c + 1..];
    Some(decode(rest.split(q).next()?))
}

/// The `href` of `<link rel="icon">` (or apple-touch-icon).
fn icon_href(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    for rel in ["rel=\"apple-touch-icon\"", "rel=\"icon\"", "rel=\"shortcut icon\""] {
        if let Some(i) = lower.find(rel) {
            let start = lower[..i].rfind('<')?;
            let end = start + lower[start..].find('>')?;
            let tag = &html[start..end];
            let h = tag.to_lowercase().find("href=")? + 5;
            let q = tag[h..].chars().next()?;
            return Some(decode(tag[h + 1..].split(q).next()?));
        }
    }
    None
}

/// `href` made absolute against the page's address.
pub fn absolute(page: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    let scheme = if page.starts_with("http://") { "http" } else { "https" };
    if let Some(rest) = href.strip_prefix("//") {
        return format!("{scheme}://{rest}");
    }
    let after = page.split_once("://").map(|x| x.1).unwrap_or(page);
    let host = after.split(['/', '?', '#']).next().unwrap_or("");
    if href.starts_with('/') {
        format!("{scheme}://{host}{href}")
    } else {
        let base = page.rsplit_once('/').map(|x| x.0).unwrap_or(page);
        format!("{base}/{href}")
    }
}

/// Pulls a title out of a page's HTML.
pub fn title_of(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let raw = if let Some(i) = lower.find("property=\"og:title\"") {
        let tag_start = lower[..i].rfind('<')?;
        let tag_end = tag_start + lower[tag_start..].find('>')?;
        let tag = &html[tag_start..tag_end];
        let c = tag.to_lowercase().find("content=\"")? + 9;
        tag[c..].split('"').next()?.to_string()
    } else {
        let a = lower.find("<title")?;
        let a = a + lower[a..].find('>')? + 1;
        let b = a + lower[a..].find("</title>")?;
        html[a..b].to_string()
    };
    let t = decode(raw.split_whitespace().collect::<Vec<_>>().join(" ").trim());
    (!t.is_empty()).then(|| t.chars().take(140).collect())
}

fn decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_a_url_from_text() {
        assert!(is_url("https://www.rust-lang.org/learn"));
        assert!(!is_url("see https://x.org for more"));
        assert!(!is_url("hello"));
        assert_eq!(site("https://user@www.Example.com:8080/a?b"), "example.com");
    }

    #[test]
    fn finds_pictures_and_icons() {
        let html = r#"<meta content="/img/card.png" property="og:image"><link rel="icon" href="//cdn.x.org/i.png">"#;
        assert_eq!(meta_content(html, "og:image").as_deref(), Some("/img/card.png"));
        assert_eq!(icon_href(html).as_deref(), Some("//cdn.x.org/i.png"));
        assert_eq!(absolute("https://x.org/a/b", "/img/card.png"), "https://x.org/img/card.png");
        assert_eq!(absolute("https://x.org/a/b", "//cdn.x.org/i.png"), "https://cdn.x.org/i.png");
        assert_eq!(absolute("https://x.org/a/b", "c.png"), "https://x.org/a/c.png");
    }

    #[test]
    fn finds_titles() {
        assert_eq!(title_of("<html><head><TITLE>\n  Rust &amp; You </TITLE>").as_deref(), Some("Rust & You"));
        assert_eq!(
            title_of(r#"<meta property="og:title" content="Better title"><title>x</title>"#).as_deref(),
            Some("Better title")
        );
        assert_eq!(title_of("<p>no title</p>"), None);
    }
}
