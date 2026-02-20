use std::{collections::HashMap, path::Path};

use auxv_dot_org::{build_rocket, pages};
use rocket::{http::Status, local::blocking::Client};
use scraper::{Html, Selector};

const REMOTE_SCHEMES: &[&str] = &["http://", "https://", "mailto:", "tel:"];
const SKIP_SCHEMES: &[&str] = &["data:", "javascript:"];

fn is_remote(url: &str) -> bool {
    REMOTE_SCHEMES.iter().any(|p| url.starts_with(p))
}

fn is_skipped(url: &str) -> bool {
    SKIP_SCHEMES.iter().any(|p| url.starts_with(p))
}

fn resolve_href(base: &str, href: &str) -> String {
    if href.starts_with('/') {
        return href.to_owned();
    }
    let parent = match base.rfind('/') {
        Some(i) => &base[..=i],
        None => "/",
    };
    format!("{parent}{href}")
}

fn check_external_url(from: &str, href: &str) -> Option<String> {
    let content = REMOTE_SCHEMES
        .iter()
        .find(|p| href.starts_with(*p))
        .map(|p| &href[p.len()..])?;
    content
        .is_empty()
        .then(|| format!("[{from}] '{href}' -> external URL has no target"))
}

struct LinkChecker {
    client: Client,
    anchor: Selector,
    image: Selector,
    script: Selector,
    link: Selector,
    any: Selector,
}

impl LinkChecker {
    fn new() -> Self {
        Self {
            client: Client::untracked(build_rocket()).unwrap(),
            anchor: Selector::parse("a[href]").unwrap(),
            image: Selector::parse("img[src]").unwrap(),
            script: Selector::parse("script[src]").unwrap(),
            link: Selector::parse("link[href]").unwrap(),
            any: Selector::parse("*").unwrap(),
        }
    }

    fn get(&self, path: &str) -> (Status, Option<Html>) {
        let response = self.client.get(path).dispatch();
        let status = response.status();
        let html = (status == Status::Ok)
            .then(|| response.into_string())
            .flatten()
            .map(|body| Html::parse_document(&body));
        (status, html)
    }

    fn contains_id(&self, doc: &Html, id: &str) -> bool {
        doc.select(&self.any).any(|el| el.attr("id") == Some(id))
    }

    fn find_duplicate_ids(&self, doc: &Html, url: &str) -> Vec<String> {
        let mut seen = HashMap::new();
        for element in doc.select(&self.any) {
            if let Some(id) = element.attr("id") {
                *seen.entry(id.to_owned()).or_insert(0u32) += 1;
            }
        }
        seen.into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(id, count)| format!("[{url}] duplicate id '{id}' appears {count} times"))
            .collect()
    }

    fn check_link(&self, from: &str, href: &str, current_doc: &Html) -> Option<String> {
        let (raw_page, anchor) = match href.split_once('#') {
            Some(("", a)) => (None, Some(a)),
            Some((p, a)) => (Some(p), Some(a).filter(|a| !a.is_empty())),
            None => (Some(href), None),
        };

        let resolved = raw_page.map(|p| resolve_href(from, p));
        let err = |msg: &str| format!("[{from}] '{href}' -> {msg}");

        if let Some(p) = resolved
            .as_deref()
            .filter(|p| Path::new(p).extension().is_some())
        {
            let status = self.client.get(p).dispatch().status();
            return (status != Status::Ok).then(|| err(&format!("file returned {status}")));
        }

        let fetched;
        let doc = match resolved.as_deref() {
            Some(p) => {
                let (status, opt_doc) = self.get(p);
                match opt_doc {
                    Some(d) => {
                        fetched = d;
                        &fetched
                    }
                    None => return Some(err(&format!("page returned {status}"))),
                }
            }
            None => current_doc,
        };

        anchor
            .filter(|id| !self.contains_id(doc, id))
            .map(|id| err(&format!("#{id} not found")))
    }

    fn check_resource(&self, from: &str, src: &str) -> Option<String> {
        let resolved = resolve_href(from, src);
        let status = self.client.get(resolved.as_str()).dispatch().status();
        (status != Status::Ok).then(|| format!("[{from}] '{src}' -> resource returned {status}"))
    }

    fn check_page(&self, url: &str) -> Vec<String> {
        let (status, doc) = self.get(url);
        let Some(doc) = doc else {
            return vec![format!("[{url}] page returned {status}")];
        };

        let mut errors = Vec::new();

        errors.extend(self.find_duplicate_ids(&doc, url));

        for element in doc.select(&self.anchor) {
            let Some(href) = element.attr("href") else {
                continue;
            };
            if is_skipped(href) {
                continue;
            }
            if is_remote(href) {
                errors.extend(check_external_url(url, href));
            } else {
                errors.extend(self.check_link(url, href, &doc));
            }
        }

        for (selector, attribute) in [
            (&self.image, "src"),
            (&self.script, "src"),
            (&self.link, "href"),
        ] {
            for element in doc.select(selector) {
                let Some(src) = element.attr(attribute) else {
                    continue;
                };
                if !is_remote(src) && !is_skipped(src) {
                    errors.extend(self.check_resource(url, src));
                }
            }
        }

        errors
    }
}

#[test]
fn all_links_resolve() {
    pages::set_page_cache().unwrap();
    let checker = LinkChecker::new();

    let failures: Vec<_> = pages::get_page_cache()
        .keys()
        .flat_map(|path| {
            let url = if *path == Path::new("index") {
                "/".to_owned()
            } else {
                format!("/{}", path.display())
            };
            checker.check_page(&url)
        })
        .collect();

    assert!(
        failures.is_empty(),
        "\n\n{} broken link(s):\n  - {}\n",
        failures.len(),
        failures.join("\n  - ")
    );
}
