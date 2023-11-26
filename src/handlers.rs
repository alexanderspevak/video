use super::{AppState, ReceiverStream, VideoListResponse, VideoResponse, VideoUpload};
use axum::{
    body::Body,
    extract::{BodyStream, Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use std::sync::mpsc;
use std::thread;

#[axum_macros::debug_handler]
pub async fn upload_video(
    State(state): State<AppState>,
    Path(video_id): Path<String>,
    mut stream: BodyStream,
) -> Response {
    // we need to check if video with same id is uploading
    {
        let state_lock = state.data.read();
        let video = state_lock.get(&video_id);
        if video.is_some() {
            let video = video.unwrap();
            if !video.is_uploaded.read().clone() {
                return (StatusCode::CONFLICT, "Video is being uploaded").into_response();
            }
        }
    }

    {
        let mut state_lock = state.data.write();
        let video_upload = VideoUpload::new();
        state_lock.insert(video_id.clone(), video_upload);
    }

    while let Some(chunk) = stream.next().await {
        let state = state.data.read();

        match chunk {
            Ok(data) => {
                let current_upload = state.get(&video_id).unwrap();
                let mut stored_data_lock = current_upload.data.write();
                stored_data_lock.push(data);
                drop(stored_data_lock);
            }

            Err(err) => {
                println!("{:?}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "error uploading video")
                    .into_response();
            }
        };
    }
    let state = state.data.read();
    let mut is_uploaded_write_lock = state.get(&video_id).unwrap().is_uploaded.write();
    *is_uploaded_write_lock = true;

    (
        StatusCode::OK,
        format!("Upload successful. Id: {}", video_id),
    )
        .into_response()
}

#[axum_macros::debug_handler]
pub async fn get_videos_list(State(state): State<AppState>) -> Json<VideoListResponse> {
    let hm = state.data.read();
    let mut videos = Vec::new();

    for (key, value) in hm.iter() {
        let is_uploaded = value.is_uploaded.read();
        videos.push(VideoResponse {
            is_uploaded: is_uploaded.clone(),
            id: key.clone(),
        })
    }

    Json(VideoListResponse { videos })
}

#[axum_macros::debug_handler]
pub async fn get_videos_list_filter(
    State(state): State<AppState>,
    Path(prefix): Path<String>,
) -> Json<VideoListResponse> {
    let mut videos = Vec::new();
    let map = state.data.read();
    for (k, v) in map
        .range(prefix.clone()..)
        .take_while(|(k, _)| k.starts_with(&prefix))
    {
        videos.push(VideoResponse {
            is_uploaded: v.is_uploaded.read().clone(),
            id: k.clone(),
        })
    }

    Json(VideoListResponse { videos })
}

pub async fn delete_video(State(state): State<AppState>, Path(video_id): Path<String>) -> Response {
    let mut map = state.data.write();

    let video = map.get(&video_id);

    if video.is_none() {
        return (StatusCode::NOT_FOUND, "Video does not exist").into_response();
    }

    let video = video.unwrap();
    if !video.is_uploaded.read().clone() {
        return (StatusCode::CONFLICT, "Video is being uploaded").into_response();
    }

    map.remove(&video_id);

    (
        StatusCode::OK,
        format!("Delete successful. Id: {}", video_id),
    )
        .into_response()
}

#[axum_macros::debug_handler]
pub async fn download_video(
    State(state): State<AppState>,
    Path(video_id): Path<String>,
) -> Response {
    let (tx, rx) = mpsc::channel();

    {
        let state = state.data.clone();
        let check_state_lock = state.read();

        if check_state_lock.get(&video_id).is_none() {
            return (
                StatusCode::NOT_FOUND,
                format!("video {} does not exist", video_id),
            )
                .into_response();
        }
    }

    thread::spawn(move || {
        let mut cursor = 0;
        let mut is_uploaded = false;
        while !is_uploaded {
            let tx_cloned = tx.clone();
            let mut data = Vec::new();
            {
                let state = state.data.read();
                let upload = state.get(&video_id.clone()).unwrap();
                let is_uploaded_lock = upload.is_uploaded.read();
                is_uploaded = is_uploaded_lock.clone();
                let data_lock = upload.data.read();
                data.extend_from_slice(&data_lock[cursor..]);
            }

            cursor += data.len();

            for byte in data.into_iter() {
                let _ = tx_cloned.send(byte.clone());
            }

            if !is_uploaded {
                //we can  wait a bit for more bytes to get uploaded
                thread::sleep(tokio::time::Duration::from_millis(50));
            }
        }
    });
    let stream = ReceiverStream { receiver: rx };

    Response::builder()
        .status(StatusCode::OK)
        .body(Body::wrap_stream(stream))
        .unwrap()
        .into_response()
}
