//! POST /api/loads/extract — multipart `screenshot` → suggested load fields. Admin only.

use axum::{
    Json,
    extract::{
        Multipart, State,
        multipart::{MultipartError, MultipartRejection},
    },
    http::StatusCode,
};

use crate::{
    auth::{AuthUser, Role},
    error::{AppError, AppResult},
    services::load_extraction::{self, ExtractedLoad, SUPPORTED_MEDIA_TYPES},
    state::AppState,
};

fn multipart_error(error: MultipartError) -> AppError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        AppError::PayloadTooLarge("Screenshot is too large".into())
    } else {
        AppError::BadRequest(error.body_text())
    }
}

pub async fn extract(
    user: AuthUser,
    State(state): State<AppState>,
    multipart: Result<Multipart, MultipartRejection>,
) -> AppResult<Json<ExtractedLoad>> {
    user.require_role(&[Role::Admin])?;
    let mut multipart =
        multipart.map_err(|rejection| AppError::BadRequest(rejection.body_text()))?;

    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() != Some("screenshot") {
            continue;
        }

        // Bun let an unsupported type reach the SDK and fail as a 500.
        let media_type = field.content_type().unwrap_or_default().to_owned();
        if !SUPPORTED_MEDIA_TYPES.contains(&media_type.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Unsupported screenshot type: {media_type}"
            )));
        }

        let image = field.bytes().await.map_err(multipart_error)?;
        if image.is_empty() {
            return Err(AppError::BadRequest("Missing screenshot file".into()));
        }

        return Ok(Json(
            load_extraction::extract_load(&state, &media_type, &image).await?,
        ));
    }

    Err(AppError::BadRequest("Missing screenshot file".into()))
}
