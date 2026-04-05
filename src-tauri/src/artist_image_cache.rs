use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

const DEFAULT_ARTIST_IMAGE_TTL_SECONDS: u64 = 60 * 60 * 24 * 7;
const METADATA_FILE_NAME: &str = "metadata.json";

#[derive(Clone)]
pub struct ArtistImageCache {
    inner: Arc<ArtistImageCacheInner>,
}

struct ArtistImageCacheInner {
    root_dir: PathBuf,
    ttl: Duration,
    in_flight: Mutex<HashMap<CacheKey, Arc<InFlight>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    profile_id: String,
    source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMetadata {
    file_name: String,
    content_type: String,
    source_url: String,
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

impl ArtistImageCache {
    pub fn new(root_dir: PathBuf) -> Self {
        Self::with_ttl(
            root_dir,
            Duration::from_secs(DEFAULT_ARTIST_IMAGE_TTL_SECONDS),
        )
    }

    fn with_ttl(root_dir: PathBuf, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(ArtistImageCacheInner {
                root_dir,
                ttl,
                in_flight: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Resolves an artist image from the cache, downloading it from `source_url` if needed.
    pub fn resolve_artist_image(
        &self,
        profile_id: &str,
        source_url: &str,
    ) -> Result<PathBuf, String> {
        let cache_key = CacheKey {
            profile_id: profile_id.to_string(),
            source_url: source_url.to_string(),
        };
        let cached_entry = self.read_entry(&cache_key)?;
        if let Some(entry) = cached_entry.as_ref().filter(|entry| entry.is_fresh) {
            return Ok(entry.path.clone());
        }

        let stale_path = cached_entry.map(|entry| entry.path);
        match self.acquire_flight(cache_key.clone()) {
            FlightRole::Leader(flight) => {
                let result = self
                    .refresh_entry(&cache_key, stale_path)
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
                "Failed to delete the artist image cache for profile {profile_id}: {error}"
            )),
        }
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
                    "Failed to parse the artist image cache metadata {}: {error}",
                    metadata_path.display()
                )
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return self.fallback_entry_without_metadata(cache_key);
            }
            Err(error) => {
                return Err(format!(
                    "Failed to read the artist image cache metadata {}: {error}",
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
                    "Failed to inspect the artist image cache directory {}: {error}",
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
        cache_key: &CacheKey,
        stale_path: Option<PathBuf>,
    ) -> Result<PathBuf, String> {
        match fetch_image_from_url(&cache_key.source_url) {
            Ok((content_type, bytes)) => self.store_entry(cache_key, &content_type, &bytes),
            Err(error) => {
                if let Some(path) = stale_path.filter(|path| path.exists()) {
                    log::warn!(
                        "artist_image_cache.refresh_entry: serving stale cache entry for profile_id={} source_url={} after refresh failure: {}",
                        cache_key.profile_id,
                        cache_key.source_url,
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
                "Failed to create the artist image cache directory {}: {error}",
                entry_dir.display()
            )
        })?;

        let extension = extension_from_content_type(content_type)?;
        let file_name = format!("art.{extension}");
        let final_path = entry_dir.join(&file_name);
        let temp_file_name = format!("art.{extension}.tmp-{}", std::process::id());
        let temp_path = entry_dir.join(temp_file_name);
        let metadata_path = entry_dir.join(METADATA_FILE_NAME);
        let metadata_temp_path = entry_dir.join(format!("{METADATA_FILE_NAME}.tmp"));
        let metadata = CacheMetadata {
            file_name: file_name.clone(),
            content_type: content_type.to_string(),
            source_url: cache_key.source_url.clone(),
            updated_at_ms: current_time_ms(),
        };
        let metadata_body = serde_json::to_vec(&metadata)
            .map_err(|error| format!("Failed to serialize artist image metadata: {error}"))?;

        fs::write(&temp_path, bytes).map_err(|error| {
            format!(
                "Failed to write the cached artist image bytes to {}: {error}",
                temp_path.display()
            )
        })?;
        replace_file(&temp_path, &final_path)?;

        fs::write(&metadata_temp_path, metadata_body).map_err(|error| {
            format!(
                "Failed to write the artist image cache metadata {}: {error}",
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
            .join(format!(
                "{:x}",
                md5::compute(cache_key.source_url.as_bytes())
            ))
    }

    fn metadata_path(&self, cache_key: &CacheKey) -> PathBuf {
        self.entry_dir(cache_key).join(METADATA_FILE_NAME)
    }
}

fn fetch_image_from_url(url: &str) -> Result<(String, Vec<u8>), String> {
    let response = reqwest::blocking::Client::new()
        .get(url)
        .send()
        .map_err(|error| format!("Failed to fetch artist image from {url}: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "The server returned HTTP {} for artist image request to {url}.",
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
            "Unexpected content type for artist image: {content_type}"
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|error| format!("Failed to read artist image response bytes: {error}"))?
        .to_vec();
    Ok((content_type, bytes))
}

fn extension_from_content_type(content_type: &str) -> Result<String, String> {
    let Some(subtype) = content_type.strip_prefix("image/") else {
        return Err(format!(
            "Unsupported content type for artist image: {content_type}"
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
            "Unsupported content type for artist image: {content_type}"
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
                "Failed to inspect the artist image cache directory {}: {error}",
                entry_dir.display()
            ));
        }
    };

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to iterate the artist image cache directory {}: {error}",
                entry_dir.display()
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "Failed to inspect the artist image cache artifact {}: {error}",
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
                "Failed to remove the stale artist image cache artifact {}: {error}",
                entry.path().display()
            )
        })?;
    }

    Ok(())
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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

    use mockito::Server;
    use tempfile::tempdir;

    use super::ArtistImageCache;

    fn spawn_delayed_image_server() -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
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

        (format!("http://{address}"), request_count, handle)
    }

    #[test]
    fn cache_miss_is_persisted_and_reused() {
        let temp_dir = tempdir().unwrap();
        let cache =
            ArtistImageCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/artist.jpg")
            .with_status(200)
            .with_header("content-type", "image/jpeg")
            .with_body("jpeg-data")
            .expect(1)
            .create();

        let url = format!("{}/artist.jpg", server.url());
        let first_path = cache.resolve_artist_image("profile-a", &url).unwrap();
        let second_path = cache.resolve_artist_image("profile-a", &url).unwrap();

        assert_eq!(first_path, second_path);
        assert!(first_path.exists());
        assert_eq!(fs::read_to_string(&first_path).unwrap(), "jpeg-data");
        mock.assert();
    }

    #[test]
    fn profile_ids_isolate_the_same_url() {
        let temp_dir = tempdir().unwrap();
        let cache =
            ArtistImageCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/artist.jpg")
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body("png-data")
            .expect(2)
            .create();

        let url = format!("{}/artist.jpg", server.url());
        let path_a = cache.resolve_artist_image("profile-a", &url).unwrap();
        let path_b = cache.resolve_artist_image("profile-b", &url).unwrap();

        assert_ne!(path_a, path_b);
        assert!(path_a.exists());
        assert!(path_b.exists());
        mock.assert();
    }

    #[test]
    fn stale_entries_refresh_and_fallback_to_existing_file_when_refresh_fails() {
        let temp_dir = tempdir().unwrap();
        let cache = ArtistImageCache::with_ttl(temp_dir.path().to_path_buf(), Duration::ZERO);
        let mut server = Server::new();
        let mock = server
            .mock("GET", "/artist.jpg")
            .with_status(200)
            .with_header("content-type", "image/jpeg")
            .with_body("jpeg-data")
            .expect(1)
            .create();

        let url = format!("{}/artist.jpg", server.url());
        let first_path = cache.resolve_artist_image("profile-a", &url).unwrap();
        mock.assert();

        drop(server);

        let second_path = cache.resolve_artist_image("profile-a", &url).unwrap();
        assert_eq!(first_path, second_path);
        assert!(second_path.exists());
    }

    #[test]
    fn invalid_content_types_are_not_cached() {
        let temp_dir = tempdir().unwrap();
        let cache =
            ArtistImageCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        server
            .mock("GET", "/artist.txt")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("not an image")
            .create();

        let url = format!("{}/artist.txt", server.url());
        let result = cache.resolve_artist_image("profile-a", &url);
        assert!(result.is_err());
    }

    #[test]
    fn concurrent_requests_share_a_single_upstream_fetch() {
        let (base_url, request_count, server_handle) = spawn_delayed_image_server();
        let temp_dir = tempdir().unwrap();
        let cache =
            ArtistImageCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));

        let url = format!("{base_url}/artist.png");
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let cache = cache.clone();
                let url = url.clone();
                thread::spawn(move || cache.resolve_artist_image("profile-a", &url))
            })
            .collect();

        let paths: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap().unwrap())
            .collect();
        server_handle.join().unwrap();

        assert!(paths.iter().all(|p| p == &paths[0]));
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn remove_profile_deletes_cached_entries() {
        let temp_dir = tempdir().unwrap();
        let cache =
            ArtistImageCache::with_ttl(temp_dir.path().to_path_buf(), Duration::from_secs(60));
        let mut server = Server::new();
        server
            .mock("GET", "/artist.jpg")
            .with_status(200)
            .with_header("content-type", "image/jpeg")
            .with_body("jpeg-data")
            .create();
        let url = format!("{}/artist.jpg", server.url());

        let path = cache.resolve_artist_image("profile-a", &url).unwrap();
        assert!(path.exists());

        cache.remove_profile("profile-a").unwrap();
        assert!(!path.exists());
    }
}
