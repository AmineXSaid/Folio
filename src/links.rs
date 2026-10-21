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

/// The page's `<title>` (or `og:title`), tidied; None if unreachable.
pub fn fetch_title(url: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .user_agent("Mozilla/5.0 (Folio link preview)")
        .build();
    let resp = agent.get(url).call().ok()?;
    let mut html = String::new();
    resp.into_reader().take(256 * 1024).read_to_string(&mut html).ok()?;
    title_of(&html)
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
    fn finds_titles() {
        assert_eq!(title_of("<html><head><TITLE>\n  Rust &amp; You </TITLE>").as_deref(), Some("Rust & You"));
        assert_eq!(
            title_of(r#"<meta property="og:title" content="Better title"><title>x</title>"#).as_deref(),
            Some("Better title")
        );
        assert_eq!(title_of("<p>no title</p>"), None);
    }
}
