use super::dto::{NginxRelease, NginxReleaseChannel};
use super::error::{NginxError, NginxResult};
use super::registry::write_json_atomically;
use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::redirect::Policy;
use scraper::{Html, Selector};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use url::Url;

const DOWNLOAD_PAGE: &str = "https://nginx.org/en/download.html";
const MAX_METADATA_BYTES: usize = 2 * 1024 * 1024;
const CACHE_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedRelease {
    pub release: NginxRelease,
    pub checked_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChannelCache {
    current: Option<CachedRelease>,
    previous: Option<CachedRelease>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseCache {
    version: u32,
    channels: BTreeMap<String, ChannelCache>,
}

impl Default for ReleaseCache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            channels: BTreeMap::new(),
        }
    }
}

pub struct ReleaseUpdateService {
    cache_path: PathBuf,
    cache: Mutex<ReleaseCache>,
    client: reqwest::Client,
}

impl ReleaseUpdateService {
    pub fn new(cache_path: PathBuf) -> NginxResult<Self> {
        let cache = load_cache(&cache_path);
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("ZxManager/0.1 NginxReleaseProvider")
            .redirect(Policy::custom(|attempt| {
                let url = attempt.url();
                if url.scheme() == "https" && url.host_str() == Some("nginx.org") {
                    attempt.follow()
                } else {
                    attempt.error("cross-origin redirect rejected")
                }
            }))
            .build()
            .map_err(|error| NginxError::new("NGINX_UPDATE_CLIENT_FAILED", error.to_string()))?;
        Ok(Self {
            cache_path,
            cache: Mutex::new(cache),
            client,
        })
    }

    pub fn cached(&self, channel: NginxReleaseChannel) -> Option<CachedRelease> {
        self.cache
            .lock()
            .unwrap()
            .channels
            .get(channel.as_key())
            .and_then(|entry| entry.current.clone())
    }

    pub async fn check(
        &self,
        channel: NginxReleaseChannel,
        force: bool,
        max_age_hours: u16,
    ) -> NginxResult<(CachedRelease, bool)> {
        if !(1..=168).contains(&max_age_hours) {
            return Err(NginxError::new(
                "NGINX_UPDATE_INTERVAL_INVALID",
                "update interval must be between 1 and 168 hours",
            ));
        }
        if !force {
            if let Some(cached) = self.cached(channel) {
                if !is_stale(&cached.checked_at, max_age_hours) {
                    return Ok((cached, false));
                }
            }
        }

        let response = self
            .client
            .get(DOWNLOAD_PAGE)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| NginxError::new("NGINX_UPDATE_FETCH_FAILED", error.to_string()))?;
        validate_nginx_url(response.url())?;
        if response.content_length().unwrap_or_default() > MAX_METADATA_BYTES as u64 {
            return Err(NginxError::new(
                "NGINX_UPDATE_METADATA_TOO_LARGE",
                "nginx release metadata exceeded the fixed size limit",
            ));
        }
        let html = read_bounded_response(response).await?;
        let release = parse_release_page(&html, channel)?;
        let cached = CachedRelease {
            release,
            checked_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        };
        self.store_success(channel, cached.clone())?;
        Ok((cached, true))
    }

    fn store_success(
        &self,
        channel: NginxReleaseChannel,
        cached: CachedRelease,
    ) -> NginxResult<()> {
        let mut cache = self.cache.lock().unwrap();
        let entry = cache
            .channels
            .entry(channel.as_key().to_owned())
            .or_default();
        entry.previous = entry.current.replace(cached);
        write_json_atomically(&self.cache_path, &*cache)
    }
}

pub fn is_stale(checked_at: &str, max_age_hours: u16) -> bool {
    DateTime::parse_from_rfc3339(checked_at)
        .map(|checked| {
            Utc::now()
                .signed_duration_since(checked.with_timezone(&Utc))
                .num_hours()
                >= i64::from(max_age_hours)
        })
        .unwrap_or(true)
}

async fn read_bounded_response(mut response: reqwest::Response) -> NginxResult<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| NginxError::new("NGINX_UPDATE_FETCH_FAILED", error.to_string()))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_METADATA_BYTES {
            return Err(NginxError::new(
                "NGINX_UPDATE_METADATA_TOO_LARGE",
                "nginx release metadata exceeded the fixed size limit",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| {
        NginxError::new(
            "NGINX_UPDATE_ENCODING_INVALID",
            "nginx release metadata was not valid UTF-8",
        )
    })
}

fn parse_release_page(html: &str, channel: NginxReleaseChannel) -> NginxResult<NginxRelease> {
    let marker = match channel {
        NginxReleaseChannel::Stable => "Stable version",
        NginxReleaseChannel::Mainline => "Mainline version",
    };
    let start = html.find(marker).ok_or_else(|| {
        NginxError::new("NGINX_UPDATE_PARSE_FAILED", "release section was not found")
    })?;
    let tail = &html[start + marker.len()..];
    let section = tail.find("<h4").map(|end| &tail[..end]).unwrap_or(tail);
    let fragment = Html::parse_fragment(section);
    let selector = Selector::parse("a[href]")
        .map_err(|error| NginxError::new("NGINX_UPDATE_PARSE_FAILED", error.to_string()))?;
    let mut candidates = fragment
        .select(&selector)
        .filter_map(|element| element.value().attr("href"))
        .filter_map(parse_source_release)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    let (_, version, href) = candidates.pop().ok_or_else(|| {
        NginxError::new("NGINX_UPDATE_PARSE_FAILED", "release link was not found")
    })?;
    let base = Url::parse(DOWNLOAD_PAGE)
        .map_err(|error| NginxError::new("NGINX_UPDATE_URL_INVALID", error.to_string()))?;
    let download = base
        .join(&href)
        .map_err(|error| NginxError::new("NGINX_UPDATE_URL_INVALID", error.to_string()))?;
    validate_nginx_url(&download)?;
    let signature = Url::parse(&format!("{}.asc", download.as_str()))
        .map_err(|error| NginxError::new("NGINX_UPDATE_URL_INVALID", error.to_string()))?;
    Ok(NginxRelease {
        version,
        download_url: download.into(),
        signature_url: signature.into(),
    })
}

fn parse_source_release(href: &str) -> Option<(Version, String, String)> {
    let file = href.strip_prefix("/download/nginx-")?;
    let version = file.strip_suffix(".tar.gz")?;
    let parsed = Version::parse(version).ok()?;
    Some((parsed, version.to_owned(), href.to_owned()))
}

fn validate_nginx_url(url: &Url) -> NginxResult<()> {
    if url.scheme() == "https" && url.host_str() == Some("nginx.org") {
        Ok(())
    } else {
        Err(NginxError::new(
            "NGINX_UPDATE_ORIGIN_REJECTED",
            "release URL must use HTTPS on nginx.org",
        ))
    }
}

fn load_cache(path: &PathBuf) -> ReleaseCache {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ReleaseCache>(&bytes).ok())
        .filter(|cache| cache.version == CACHE_VERSION)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"
      <h4>Mainline version</h4><table><tr><td><a href="/download/nginx-1.29.1.tar.gz">nginx-1.29.1</a></td></tr></table>
      <h4>Stable version</h4><table><tr><td><a href="/download/nginx-1.28.0.tar.gz">nginx-1.28.0</a></td></tr></table>
      <h4>Legacy versions</h4><table><tr><td><a href="/download/nginx-1.26.3.tar.gz">nginx-1.26.3</a></td></tr></table>
    "#;

    #[test]
    fn parses_fixed_stable_and_mainline_sections() {
        let stable = parse_release_page(PAGE, NginxReleaseChannel::Stable).expect("stable");
        let mainline = parse_release_page(PAGE, NginxReleaseChannel::Mainline).expect("mainline");
        assert_eq!(stable.version, "1.28.0");
        assert_eq!(mainline.version, "1.29.1");
        assert_eq!(
            stable.download_url,
            "https://nginx.org/download/nginx-1.28.0.tar.gz"
        );
        assert_eq!(
            stable.signature_url,
            "https://nginx.org/download/nginx-1.28.0.tar.gz.asc"
        );
    }

    #[test]
    fn rejects_non_nginx_origins() {
        assert!(validate_nginx_url(&Url::parse("https://example.com/nginx").unwrap()).is_err());
        assert!(validate_nginx_url(&Url::parse("http://nginx.org/download").unwrap()).is_err());
    }

    #[test]
    fn cache_preserves_current_and_previous_successes() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("release-cache-v1.json");
        let service = ReleaseUpdateService::new(path.clone()).expect("service");
        for version in ["1.28.0", "1.28.1"] {
            service
                .store_success(
                    NginxReleaseChannel::Stable,
                    CachedRelease {
                        release: NginxRelease {
                            version: version.to_owned(),
                            download_url: format!(
                                "https://nginx.org/download/nginx-{version}.tar.gz"
                            ),
                            signature_url: format!(
                                "https://nginx.org/download/nginx-{version}.tar.gz.asc"
                            ),
                        },
                        checked_at: "2026-07-31T00:00:00Z".to_owned(),
                    },
                )
                .expect("cache success");
        }
        let loaded = load_cache(&path);
        let stable = loaded.channels.get("stable").expect("stable cache");
        assert_eq!(
            stable
                .current
                .as_ref()
                .map(|value| value.release.version.as_str()),
            Some("1.28.1")
        );
        assert_eq!(
            stable
                .previous
                .as_ref()
                .map(|value| value.release.version.as_str()),
            Some("1.28.0")
        );
    }
}
