use opensubsonic_client::api::retrieval::{GetCoverArtRequest, RetrievalApi};
use tauri::{AppHandle, State};

use crate::{
    commands::common::{client, encode_data_url, fetch_binary_response, format_api_error},
    models::{CoverArtRequest, CoverArtResponse},
    ActiveSessionState,
};

#[tauri::command]
#[specta::specta]
pub async fn get_cover_art(
    app: AppHandle,
    state: State<'_, ActiveSessionState>,
    payload: CoverArtRequest,
) -> Result<CoverArtResponse, String> {
    let cover_art_id = payload.cover_art_id.trim();
    if cover_art_id.is_empty() {
        return Err("coverArtId is required.".to_string());
    }

    let client = client(&app, &state.0)?;
    let request = client
        .get_cover_art(GetCoverArtRequest {
            id: cover_art_id.to_string(),
            size: payload.size,
        })
        .map_err(format_api_error)?;
    let (content_type, bytes) = fetch_binary_response(request).await?;

    Ok(CoverArtResponse {
        data_url: encode_data_url(&content_type, &bytes),
    })
}

#[cfg(test)]
mod tests {
    use crate::commands::common::encode_data_url;

    #[test]
    fn encode_data_url_uses_base64() {
        let encoded = encode_data_url("image/png", b"abc");

        assert_eq!(encoded, "data:image/png;base64,YWJj");
    }
}
