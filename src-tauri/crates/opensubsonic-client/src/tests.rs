use std::collections::BTreeMap;

use mockito::{Matcher, Server};
use serde_json::{json, Value};

use crate::{
    api::{
        annotation::{AnnotationApi, ReportPlaybackRequest},
        browsing::{BrowsingApi, GetIndexesRequest, GetMusicDirectoryRequest, GetSongRequest},
        search::{Search3Request, SearchApi},
        system::SystemApi,
        transcoding::{GetTranscodeDecisionRequest, GetTranscodeStreamRequest, TranscodingApi},
    },
    auth::build_token,
    normalize_base_url, ApiError, Auth, ClientConfig, ClientInfo, MediaType, OpenSubsonicClient,
    PlaybackState, ServerProbe,
};

fn build_client(server_url: &str, auth: Auth) -> OpenSubsonicClient {
    OpenSubsonicClient::new(ClientConfig::new(
        normalize_base_url(server_url).unwrap(),
        auth,
        "transonic",
    ))
    .unwrap()
}

#[test]
fn normalize_base_url_adds_rest_to_root_urls() {
    let normalized = normalize_base_url("https://demo.example").unwrap();
    assert_eq!(normalized.as_str(), "https://demo.example/rest");
}

#[test]
fn normalize_base_url_removes_embedded_credentials() {
    let normalized = normalize_base_url("https://demo:secret@example.com/player").unwrap();
    assert_eq!(normalized.as_str(), "https://example.com/player/rest");
}

#[test]
fn build_token_matches_the_subsonic_documentation_example() {
    let token = build_token("sesame", "c19b2d");
    assert_eq!(token, "26719a1196d2a940705a59634eb18eab");
}

#[test]
fn api_key_query_contains_only_api_key_authentication() {
    let query = Auth::ApiKey {
        api_key: "secret-key".to_string(),
    }
    .to_query_pairs();

    assert!(query.contains(&("apiKey".to_string(), "secret-key".to_string())));
    assert!(!query
        .iter()
        .any(|(key, _)| matches!(key.as_str(), "u" | "p" | "t" | "s")));
}

#[tokio::test]
async fn public_probe_fetches_open_subsonic_extensions_without_auth() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "ok",
        "version": "1.16.1",
        "openSubsonic": true,
        "openSubsonicExtensions": [
          { "name": "apiKeyAuthentication", "versions": [1] }
        ]
      }
    }"#;

    server
        .mock("GET", "/rest/getOpenSubsonicExtensions.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("v".into(), "1.13.0".into()),
            Matcher::UrlEncoded("c".into(), "transonic".into()),
            Matcher::UrlEncoded("f".into(), "json".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let probe = ServerProbe::new("transonic").unwrap();
    let response = probe
        .get_open_subsonic_extensions(&normalize_base_url(&server.url()).unwrap())
        .await
        .unwrap();

    assert!(response.meta.open_subsonic.unwrap_or(false));
    assert_eq!(response.payload.open_subsonic_extensions.len(), 1);
}

#[tokio::test]
async fn token_info_uses_api_key_without_username() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "ok",
        "version": "1.16.1",
        "openSubsonic": true,
        "tokenInfo": { "username": "resolved-user" }
      }
    }"#;

    server
        .mock("GET", "/rest/tokenInfo.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("apiKey".into(), "secret-key".into()),
            Matcher::UrlEncoded("v".into(), "1.13.0".into()),
            Matcher::UrlEncoded("c".into(), "transonic".into()),
            Matcher::UrlEncoded("f".into(), "json".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let client = build_client(
        &server.url(),
        Auth::ApiKey {
            api_key: "secret-key".to_string(),
        },
    );
    let response = client.token_info().await.unwrap();

    assert_eq!(response.payload.token_info.username, "resolved-user");
}

#[tokio::test]
async fn get_transcode_decision_posts_json_body() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "ok",
        "version": "1.16.1",
        "openSubsonic": true,
        "transcodeDecision": {
          "canDirectPlay": false,
          "transcodeParams": "0001-0005-004"
        }
      }
    }"#;

    server
        .mock("POST", "/rest/getTranscodeDecision.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("u".into(), "demo".into()),
            Matcher::UrlEncoded("v".into(), "1.13.0".into()),
            Matcher::UrlEncoded("c".into(), "transonic".into()),
            Matcher::UrlEncoded("f".into(), "json".into()),
            Matcher::UrlEncoded("mediaId".into(), "song-1".into()),
            Matcher::UrlEncoded("mediaType".into(), "song".into()),
        ]))
        .match_body(Matcher::Json(json!({
            "name": "transonic",
            "platform": "windows"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let mut values = BTreeMap::new();
    values.insert("name".to_string(), Value::String("transonic".to_string()));
    values.insert("platform".to_string(), Value::String("windows".to_string()));

    let client = build_client(
        &server.url(),
        Auth::Token {
            username: "demo".to_string(),
            password: "sesame".to_string(),
        },
    );
    let response = client
        .get_transcode_decision(
            GetTranscodeDecisionRequest {
                media_id: "song-1".to_string(),
                media_type: MediaType::Song,
            },
            ClientInfo { values },
        )
        .await
        .unwrap();

    assert_eq!(
        response
            .payload
            .transcode_decision
            .get("transcodeParams")
            .and_then(Value::as_str),
        Some("0001-0005-004")
    );
}

#[test]
fn get_transcode_stream_reuses_server_transcode_params() {
    let client = build_client(
        "https://demo.example",
        Auth::Token {
            username: "demo".to_string(),
            password: "sesame".to_string(),
        },
    );

    let prepared = client
        .get_transcode_stream(GetTranscodeStreamRequest {
            media_id: "song-1".to_string(),
            media_type: MediaType::Song,
            offset: Some(12),
            transcode_params: "0001-0005-004".to_string(),
        })
        .unwrap();

    let query: BTreeMap<_, _> = prepared.url.query_pairs().into_owned().collect();
    assert_eq!(
        query.get("transcodeParams"),
        Some(&"0001-0005-004".to_string())
    );
    assert_eq!(query.get("mediaId"), Some(&"song-1".to_string()));
}

#[tokio::test]
async fn report_playback_sends_expected_query() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "ok",
        "version": "1.16.1",
        "openSubsonic": true
      }
    }"#;

    server
        .mock("GET", "/rest/reportPlayback.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("u".into(), "demo".into()),
            Matcher::UrlEncoded("mediaId".into(), "song-1".into()),
            Matcher::UrlEncoded("mediaType".into(), "song".into()),
            Matcher::UrlEncoded("positionMs".into(), "1234".into()),
            Matcher::UrlEncoded("state".into(), "playing".into()),
            Matcher::UrlEncoded("playbackRate".into(), "1.25".into()),
            Matcher::UrlEncoded("ignoreScrobble".into(), "false".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let client = build_client(
        &server.url(),
        Auth::Token {
            username: "demo".to_string(),
            password: "sesame".to_string(),
        },
    );

    client
        .report_playback(ReportPlaybackRequest {
            media_id: "song-1".to_string(),
            media_type: MediaType::Song,
            position_ms: 1234,
            state: PlaybackState::Playing,
            playback_rate: Some(1.25),
            ignore_scrobble: Some(false),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn get_indexes_includes_music_folder_id_when_requested() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "ok",
        "version": "1.16.1",
        "openSubsonic": true,
        "indexes": {
          "index": []
        }
      }
    }"#;

    server
        .mock("GET", "/rest/getIndexes.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("u".into(), "demo".into()),
            Matcher::UrlEncoded("v".into(), "1.13.0".into()),
            Matcher::UrlEncoded("c".into(), "transonic".into()),
            Matcher::UrlEncoded("f".into(), "json".into()),
            Matcher::UrlEncoded("musicFolderId".into(), "1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let client = build_client(
        &server.url(),
        Auth::Token {
            username: "demo".to_string(),
            password: "sesame".to_string(),
        },
    );

    client
        .get_indexes(GetIndexesRequest {
            music_folder_id: Some("1".to_string()),
            if_modified_since: None,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn get_music_directory_surfaces_api_failures_without_payload_fields() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "failed",
        "version": "1.16.1",
        "type": "navidrome",
        "serverVersion": "0.60.3",
        "openSubsonic": true,
        "error": {
          "code": 70,
          "message": "Directory not found"
        }
      }
    }"#;

    server
        .mock("GET", "/rest/getMusicDirectory.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("u".into(), "demo".into()),
            Matcher::UrlEncoded("v".into(), "1.13.0".into()),
            Matcher::UrlEncoded("c".into(), "transonic".into()),
            Matcher::UrlEncoded("f".into(), "json".into()),
            Matcher::UrlEncoded("id".into(), "1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let client = build_client(
        &server.url(),
        Auth::Token {
            username: "demo".to_string(),
            password: "sesame".to_string(),
        },
    );

    let error = client
        .get_music_directory(GetMusicDirectoryRequest {
            id: "1".to_string(),
        })
        .await
        .unwrap_err();

    match error {
        ApiError::Api { code, message, .. } => {
            assert_eq!(code, 70);
            assert_eq!(message.as_deref(), Some("Directory not found"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn search3_decodes_single_objects_and_string_numbers() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "ok",
        "version": "1.16.1",
        "searchResult3": {
          "artist": {
            "id": 1001,
            "name": "Artist One",
            "albumCount": "2"
          },
          "album": {
            "id": "album-1",
            "name": "Album One",
            "songCount": "10"
          },
          "song": {
            "id": "song-1",
            "title": "Song One",
            "track": "04",
            "duration": "178",
            "mediaType": " SONG "
          }
        }
      }
    }"#;

    server
        .mock("GET", "/rest/search3.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("u".into(), "demo".into()),
            Matcher::UrlEncoded("v".into(), "1.13.0".into()),
            Matcher::UrlEncoded("c".into(), "transonic".into()),
            Matcher::UrlEncoded("f".into(), "json".into()),
            Matcher::UrlEncoded("query".into(), "smoke".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let client = build_client(
        &server.url(),
        Auth::Token {
            username: "demo".to_string(),
            password: "sesame".to_string(),
        },
    );

    let response = client
        .search3(Search3Request {
            query: "smoke".to_string(),
            artist_count: None,
            artist_offset: None,
            album_count: None,
            album_offset: None,
            song_count: None,
            song_offset: None,
            music_folder_id: None,
        })
        .await
        .unwrap();

    let payload = response.payload.search_result3;
    assert_eq!(payload.artist[0].id, "1001");
    assert_eq!(payload.artist[0].album_count, Some(2));
    assert_eq!(payload.album[0].song_count, Some(10));
    assert_eq!(payload.song[0].track, Some(4));
    assert_eq!(payload.song[0].duration, Some(178));
    assert_eq!(payload.song[0].media_type.as_deref(), Some(" SONG "));
}

#[tokio::test]
async fn get_song_decode_requires_title_without_fallback() {
    let mut server = Server::new_async().await;
    let body = r#"{
      "subsonic-response": {
        "status": "ok",
        "version": "1.16.1",
        "song": {
          "id": "song-1",
          "name": "Song One"
        }
      }
    }"#;

    server
        .mock("GET", "/rest/getSong.view")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("u".into(), "demo".into()),
            Matcher::UrlEncoded("v".into(), "1.13.0".into()),
            Matcher::UrlEncoded("c".into(), "transonic".into()),
            Matcher::UrlEncoded("f".into(), "json".into()),
            Matcher::UrlEncoded("id".into(), "song-1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let client = build_client(
        &server.url(),
        Auth::Token {
            username: "demo".to_string(),
            password: "sesame".to_string(),
        },
    );

    let error = client
        .get_song(GetSongRequest {
            id: "song-1".to_string(),
        })
        .await
        .unwrap_err();

    match error {
        ApiError::Decode { message, .. } => assert!(message.contains("title")),
        other => panic!("unexpected error: {other:?}"),
    }
}
