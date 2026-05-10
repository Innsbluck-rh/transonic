use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use opensubsonic_client::{
    api::retrieval::{GetCoverArtRequest, RetrievalApi},
    OpenSubsonicClient, PreparedBinaryRequest,
};
use serde::{Deserialize, Serialize};

use crate::{commands::common::format_api_error, models::CoverArtCacheStatus};

const DEFAULT_COVER_ART_TTL_SECONDS: u64 = 60 * 60 * 24 * 7;
const METADATA_FILE_NAME: &str = "metadata.json";

#[derive(Clone)]
pub struct CoverArtCache {
    inner: Arc<CoverArtCacheInner>,
}

struct CoverArtCacheInner {
    root_dir: PathBuf,
    ttl: Duration,
    http_client: reqwest::blocking::Client,
    in_flight: Mutex<HashMap<CacheKey, Arc<InFlight>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    profile_id: String,
    cover_art_id: String,
    size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMetadata {
    file_name: String,
    content_type: String,
    updated_at_ms: u64,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    path: PathBuf,
    is_fresh: bool,
}

struct InFlight {
    state: Mutex<InFlightState>,
    done: Condvar,
}

#[derive(Debug, Clone)]
enum InFlightState {
    Running,
    Completed(Result<PathBuf, String>),
}

impl InFlight {
    fn new() -> Self {
        Self {
            state: Mutex::new(InFlightState::Running),
            done: Condvar::new(),
        }
    }

    fn complete(&self, result: Result<PathBuf, String>) {
        let mut state = self.state.lock().unwrap();
        *state = InFlightState::Completed(result);
        self.done.notify_all();
    }

    fn wait(&self) -> Result<PathBuf, String> {
        let mut state = self.state.lock().unwrap();
        loop {
            match &*state {
                InFlightState::Running => {
                    state = self.done.wait(state).unwrap();
                }
                InFlightState::Completed(result) => return result.clone(),
            }
        }
    }
}

enum FlightRole {
    Leader(Arc<InFlight>),
    Follower(Arc<InFlight>),
}

impl CoverArtCache {
    pub fn new(root_dir: PathBuf) -> Self {
        Self::with_ttl(root_dir, Duration::from_secs(DEFAULT_COVER_ART_TTL_SECONDS))
    }

    fn with_ttl(root_dir: PathBuf, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(CoverArtCacheInner {
                root_dir,
                ttl,
                http_client: reqwest::blocking::Client::new(),
                in_flight: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn resolve_cover_art(
        &self,
        client: &OpenSubsonicClient,
        profile_id: &str,
        cover_art_id: &str,
        size: Option<u32>,
    ) -> Result<PathBuf, String> {
        let cache_key = CacheKey {
            profile_id: profile_id.to_string(),
            cover_art_id: cover_art_id.to_string(),
            size,
        };
        let cached_entry = self.read_entry(&cache_key)?;
        if let Some(entry) = cached_entry.as_ref().filter(|entry| entry.is_fresh) {
            return Ok(entry.path.clone());
        }

        let stale_path = cached_entry.map(|entry| entry.path);
        match self.acquire_flight(cache_key.clone()) {
            FlightRole::Leader(flight) => {
                let result = self
                    .refresh_entry(client, &cache_key, stale_path)
                    .map_err(|error| error.to_string());
                flight.complete(result.clone());

                let mut in_flight = self.inner.in_flight.lock().unwrap();
                in_flight.remove(&cache_key);

                result
            }
            FlightRole::Follower(flight) => flight.wait(),
        }
    }

    pub fn remove_profile(&self, profile_id: &str) -> Result<(), String> {
        let profile_dir = self.inner.root_dir.join(profile_id);
        match fs::remove_dir_all(&profile_dir) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "Failed to delete the cover art cache for profile {profile_id}: {error}"
            )),
        }
    }

    pub fn status(&self, profile_id: Option<&str>) -> Result<CoverArtCacheStatus, String> {
        let cache_dir = profile_id
            .map(|value| self.inner.root_dir.join(value))
            .unwrap_or_else(|| self.inner.root_dir.clone());
        let mut stats = CacheStats::default();
        collect_cache_stats(&cache_dir, &mut stats)?;

        Ok(CoverArtCacheStatus {
            profile_id: profile_id.map(str::to_string),
            cache_dir: cache_dir.to_string_lossy().to_string(),
            entry_count: u64_to_u32_saturating(stats.entry_count),
            file_count: u64_to_u32_saturating(stats.file_count),
            total_bytes: stats.total_bytes as f64,
            ttl_seconds: u64_to_u32_saturating(self.inner.ttl.as_secs()),
        })
    }

    fn acquire_flight(&self, cache_key: CacheKey) -> FlightRole {
        let mut in_flight = self.inner.in_flight.lock().unwrap();
        if let Some(flight) = in_flight.get(&cache_key) {
            return FlightRole::Follower(flight.clone());
        }

        let flight = Arc::new(InFlight::new());
        in_flight.insert(cache_key, flight.clone());
        FlightRole::Leader(flight)
    }

    fn read_entry(&self, cache_key: &CacheKey) -> Result<Option<CacheEntry>, String> {
        let metadata_path = self.metadata_path(cache_key);
        let metadata = match fs::read_to_string(&metadata_path) {
            Ok(contents) => serde_json::from_str::<CacheMetadata>(&contents).map_err(|error| {
                format!(
                    "Failed to parse the cover art cache metadata {}: {error}",
                    metadata_path.display()
                )
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return self.fallback_entry_without_metadata(cache_key);
            }
            Err(error) => {
                return Err(format!(
                    "Failed to read the cover art cache metadata {}: {error}",
                    metadata_path.display()
                ));
            }
        };

        let art_path = self.entry_dir(cache_key).join(&metadata.file_name);
        if !art_path.exists() {
            return self.fallback_entry_without_metadata(cache_key);
        }

        let age = current_time_ms().saturating_sub(metadata.updated_at_ms);
        let is_fresh = age <= ttl_to_ms(self.inner.ttl);
        Ok(Some(CacheEntry {
            path: art_path,
            is_fresh,
        }))
    }

    fn fallback_entry_without_metadata(
        &self,
        cache_key: &CacheKey,
    ) -> Result<Option<CacheEntry>, String> {
        let entry_dir = self.entry_dir(cache_key);
        let entries = match fs::read_dir(&entry_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "Failed to inspect the cover art cache directory {}: {error}",
                    entry_dir.display()
                ));
            }
        };
        let mut art_files = entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
            })
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("art."))
            .map(|entry| entry.path())
            .collect::<Vec<_>>();

        art_files.sort();
        let Some(path) = art_files.pop() else {
            return Ok(None);
        };

        Ok(Some(CacheEntry {
            path,
            is_fresh: false,
        }))
    }

    fn refresh_entry(
        &self,
        client: &OpenSubsonicClient,
        cache_key: &CacheKey,
        stale_path: Option<PathBuf>,
    ) -> Result<PathBuf, String> {
        let request = client
            .get_cover_art(GetCoverArtRequest {
                id: cache_key.cover_art_id.clone(),
                size: cache_key.size,
            })
            .map_err(format_api_error)?;
        match fetch_binary_response(&self.inner.http_client, request) {
            Ok((content_type, bytes)) => self.store_entry(cache_key, &content_type, &bytes),
            Err(error) => {
                if let Some(path) = stale_path.filter(|path| path.exists()) {
                    log::warn!(
                        "cover_art_cache.refresh_entry: serving stale cache entry for profile_id={} cover_art_id={} after refresh failure: {}",
                        cache_key.profile_id,
                        cache_key.cover_art_id,
                        error
                    );
                    Ok(path)
                } else {
                    Err(error)
                }
            }
        }
    }

    fn store_entry(
        &self,
        cache_key: &CacheKey,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, String> {
        let entry_dir = self.entry_dir(cache_key);
        fs::create_dir_all(&entry_dir).map_err(|error| {
            format!(
                "Failed to create the cover art cache directory {}: {error}",
                entry_dir.display()
            )
        })?;

        let extension = extension_from_content_type(content_type)?;
        let version = current_time_ns();
        let file_name = format!("art.{version}.{extension}");
        let final_path = entry_dir.join(&file_name);
        let temp_file_name = format!("art.{extension}.tmp-{}", std::process::id());
        let temp_path = entry_dir.join(temp_file_name);
        let metadata_path = entry_dir.join(METADATA_FILE_NAME);
        let metadata_temp_path = entry_dir.join(format!("{METADATA_FILE_NAME}.tmp"));
        let metadata = CacheMetadata {
            file_name: file_name.clone(),
            content_type: content_type.to_string(),
            updated_at_ms: current_time_ms(),
        };
        let metadata_body = serde_json::to_vec(&metadata)
            .map_err(|error| format!("Failed to serialize cover art metadata: {error}"))?;

        fs::write(&temp_path, bytes).map_err(|error| {
            format!(
                "Failed to write the cached cover art bytes to {}: {error}",
                temp_path.display()
            )
        })?;
        replace_file(&temp_path, &final_path)?;

        fs::write(&metadata_temp_path, metadata_body).map_err(|error| {
            format!(
                "Failed to write the cover art cache metadata {}: {error}",
                metadata_temp_path.display()
            )
        })?;
        replace_file(&metadata_temp_path, &metadata_path)?;

        remove_stale_artifacts(&entry_dir, &file_name)?;

        Ok(final_path)
    }

    fn entry_dir(&self, cache_key: &CacheKey) -> PathBuf {
        self.inner
            .root_dir
            .join(&cache_key.profile_id)
            .join(size_segment(cache_key.size))
            .join(format!(
                "{:x}",
                md5::compute(cache_key.cover_art_id.as_bytes())
            ))
    }

    fn metadata_path(&self, cache_key: &CacheKey) -> PathBuf {
        self.entry_dir(cache_key).join(METADATA_FILE_NAME)
    }
}

#[derive(Default)]
struct CacheStats {
    entry_count: u64,
    file_count: u64,
    total_bytes: u64,
}

fn collect_cache_stats(path: &Path, stats: &mut CacheStats) -> Result<(), String> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to inspect the cover art cache directory {}: {error}",
                path.display()
            ));
        }
    };

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to iterate the cover art cache directory {}: {error}",
                path.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Failed to inspect the cover art cache artifact {}: {error}",
                entry.path().display()
            )
        })?;

        if file_type.is_dir() {
            collect_cache_stats(&entry.path(), stats)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        stats.file_count += 1;
        if file_name.starts_with("art.") && !file_name.contains(".tmp") {
            stats.entry_count += 1;
            stats.total_bytes += entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        }
    }

    Ok(())
}

fn fetch_binary_response(
    client: &reqwest::blocking::Client,
    request: PreparedBinaryRequest,
) -> Result<(String, Vec<u8>), String> {
    let response = client
        .get(request.url)
        .headers(request.headers)
        .send()
        .map_err(|error| format!("Failed to fetch binary data: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "The server returned HTTP {} for the binary request.",
            status.as_u16()
        ));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/jpeg")
        .split(';')
        .next()
        .map(str::trim)
        .unwrap_or("image/jpeg")
        .to_string();
    if !content_type.starts_with("image/") {
        return Err(format!(
            "Unexpected binary content type for cover art: {content_type}"
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|error| format!("Failed to read binary response bytes: {error}"))?
        .to_vec();
    Ok((content_type, bytes))
}

fn extension_from_content_type(content_type: &str) -> Result<String, String> {
    let Some(subtype) = content_type.strip_prefix("image/") else {
        return Err(format!(
            "Unsupported binary content type for cover art: {content_type}"
        ));
    };

    let normalized = match subtype {
        "jpeg" | "jpg" => "jpg".to_string(),
        "svg+xml" => "svg".to_string(),
        "x-icon" | "vnd.microsoft.icon" => "ico".to_string(),
        _ => subtype
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>(),
    };
    if normalized.is_empty() {
        return Err(format!(
            "Unsupported binary content type for cover art: {content_type}"
        ));
    }

    Ok(normalized)
}

fn replace_file(from: &Path, to: &Path) -> Result<(), String> {
    if to.exists() {
        fs::remove_file(to).map_err(|error| {
            format!(
                "Failed to replace the cached file {}: {error}",
                to.display()
            )
        })?;
    }

    fs::rename(from, to).map_err(|error| {
        format!(
            "Failed to move the cached file from {} to {}: {error}",
            from.display(),
            to.display()
        )
    })
}

fn remove_stale_artifacts(entry_dir: &Path, current_file_name: &str) -> Result<(), String> {
    let entries = match fs::read_dir(entry_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to inspect the cover art cache directory {}: {error}",
                entry_dir.display()
            ));
        }
    };

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to iterate the cover art cache directory {}: {error}",
                entry_dir.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Failed to inspect the cover art cache artifact {}: {error}",
                entry.path().display()
            )
        })?;
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let should_remove = if file_name == METADATA_FILE_NAME {
            false
        } else if file_name.starts_with("art.") {
            file_name.as_ref() != current_file_name
        } else {
            file_name.contains(".tmp")
        };
        if !should_remove {
            continue;
        }

        fs::remove_file(entry.path()).map_err(|error| {
            format!(
                "Failed to remove the stale cover art cache artifact {}: {error}",
                entry.path().display()
            )
        })?;
    }

    Ok(())
}

fn size_segment(size: Option<u32>) -> String {
    size.map(|value| value.to_string())
        .unwrap_or_else(|| "full".to_string())
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn current_time_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn u64_to_u32_saturating(value: u64) -> u32 {
    value.min(u64::from(u32::MAX)) as u32
}

fn ttl_to_ms(ttl: Duration) -> u64 {
    ttl.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
        time::{Duration, Instant},
    };

    use mockito::{Matcher, Server};
    use opensubsonic_client::{normalize_base_url, Auth, ClientConfig};
    use tempfile::tempdir;

    use super::{size_segment, CacheMetadata, CoverArtCache, METADATA_FILE_NAME};

    fn create_client_from_base_url(base_url: &str) -> opensubsonic_client::OpenSubsonicClient {
        let base_url = normalize_base_url(base_url).unwrap();
        opensubsonic_client::OpenSubsonicClient::new(ClientConfig::new(
            base_url,
            Auth::LegacyPassword {
                username: "demo".to_string(),
                password: "demo".to_string(),
            },
            "transonic".to_string(),
        ))
        .unwrap()
    }

    fn create_client(server: &Server) -> opensubsonic_client::OpenSubsonicClient {
        create_client_from_base_url(&format!("{}/rest", server.url()))
    }

    fn spawn_delayed_cover_art_server() -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let request_count = Arc::new(AtomicUsize::new(0));
        let request_count_for_thread = request_count.clone();

        let handle = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(500);
            while Instant::now() <= deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        request_count_for_thread.fetch_add(1, Ordering::SeqCst);
                        let mut buffer = [0u8; 2048];
                        let _ = stream.read(&mut buffer);
                        thread::sleep(Duration::from_millis(100));

                        let body = b"png-data";
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.write_all(body);
                        let _ = stream.flush();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        (format!("http://{address}/rest"), request_count, handle)
    }

    #[test]
    fn size_segment_uses_full_for_missing_size() {
        assert_eq!(size_segment(None), "full");
        assert_eq!(size_segment(Some(128)), "128");
    }

    #[test]
    fn cache_miss_is_persisted_and_reused() {
        let temp_dir = tempdir().unwrap();
        let cache = CoverArtCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("png-data")
            .expect(1)
            .create();
        let client = create_client(&server);

        let first_path = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();
        let second_path = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();

        assert_eq!(first_path, second_path);
        assert!(first_path.exists());
        mock.assert();
    }

    #[test]
    fn status_counts_profile_cache_entries() {
        let temp_dir = tempdir().unwrap();
        let cache = CoverArtCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("png-data")
            .expect(1)
            .create();
        let client = create_client(&server);

        cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();

        let status = cache.status(Some("profile-a")).unwrap();

        assert_eq!(status.profile_id.as_deref(), Some("profile-a"));
        assert_eq!(status.entry_count, 1);
        assert_eq!(status.file_count, 2);
        assert!(status.total_bytes >= "png-data".len() as f64);
        assert_eq!(status.ttl_seconds, 60);
        mock.assert();
    }

    #[test]
    fn size_variants_are_cached_separately() {
        let temp_dir = tempdir().unwrap();
        let cache = CoverArtCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let small_mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("small")
            .expect(1)
            .create();
        let large_mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "512".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("large")
            .expect(1)
            .create();
        let client = create_client(&server);

        let small_path = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();
        let large_path = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(512))
            .unwrap();

        assert_ne!(small_path, large_path);
        small_mock.assert();
        large_mock.assert();
    }

    #[test]
    fn profile_ids_isolate_the_same_cover_art_id() {
        let temp_dir = tempdir().unwrap();
        let cache = CoverArtCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("png-data")
            .expect(2)
            .create();
        let client = create_client(&server);

        let path_a = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();
        let path_b = cache
            .resolve_cover_art(&client, "profile-b", "cover-1", Some(64))
            .unwrap();

        assert_ne!(path_a, path_b);
        assert!(path_a.to_string_lossy().contains("profile-a"));
        assert!(path_b.to_string_lossy().contains("profile-b"));
        mock.assert();
    }

    #[test]
    fn stale_entries_refresh_and_fallback_to_existing_file_when_refresh_fails() {
        let temp_dir = tempdir().unwrap();
        let cache = CoverArtCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(1));
        let mut server = Server::new();
        let success_mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("fresh")
            .expect(1)
            .create();
        let client = create_client(&server);

        let cached_path = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();

        let metadata_path = cached_path.parent().unwrap().join(METADATA_FILE_NAME);
        let stale_metadata = CacheMetadata {
            file_name: cached_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            content_type: "image/png".to_string(),
            updated_at_ms: 0,
        };
        fs::write(metadata_path, serde_json::to_vec(&stale_metadata).unwrap()).unwrap();

        success_mock.assert();

        let failure_mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(500)
            .with_body("server-error")
            .expect(1)
            .create();

        let fallback_path = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();

        assert_eq!(fallback_path, cached_path);
        failure_mock.assert();
    }

    #[test]
    fn invalid_content_types_are_not_cached() {
        let temp_dir = tempdir().unwrap();
        let cache = CoverArtCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "text/xml")
            .with_body("<xml />")
            .expect(1)
            .create();
        let client = create_client(&server);

        let error = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap_err();

        assert!(error.contains("Unexpected binary content type"));
        assert!(!temp_dir.path().join("profile-a").exists());
        mock.assert();
    }

    #[test]
    fn concurrent_requests_share_a_single_upstream_fetch() {
        let temp_dir = tempdir().unwrap();
        let cache = Arc::new(CoverArtCache::with_ttl(
            temp_dir.path().to_path_buf(),
            Duration::from_secs(60),
        ));
        let (base_url, request_count, server_handle) = spawn_delayed_cover_art_server();
        let client = Arc::new(create_client_from_base_url(&base_url));

        let mut handles = Vec::new();
        for _ in 0..4 {
            let cache = cache.clone();
            let client = client.clone();
            handles.push(thread::spawn(move || {
                cache
                    .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
                    .unwrap()
            }));
        }

        let mut paths = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        paths.dedup();

        assert_eq!(paths.len(), 1);
        server_handle.join().unwrap();
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn remove_profile_deletes_cached_entries() {
        let temp_dir = tempdir().unwrap();
        let cache = CoverArtCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/rest/getCoverArt.view")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("id".into(), "cover-1".into()),
                Matcher::UrlEncoded("size".into(), "64".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("png-data")
            .expect(1)
            .create();
        let client = create_client(&server);

        let path = cache
            .resolve_cover_art(&client, "profile-a", "cover-1", Some(64))
            .unwrap();
        let profile_dir = temp_dir.path().join("profile-a");

        assert!(path.exists());
        assert!(profile_dir.exists());

        cache.remove_profile("profile-a").unwrap();

        assert!(!profile_dir.exists());
        mock.assert();
    }
}
