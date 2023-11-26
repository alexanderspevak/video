use axum::{
    routing::{delete, get, put},
    Router,
};
use bytes::Bytes;
use handlers::{
    delete_video, download_video, get_videos_list, get_videos_list_filter, upload_video,
};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use stream::ReceiverStream;

pub mod handlers;
pub mod stream;

pub struct VideoUpload {
    data: RwLock<Vec<Bytes>>,
    is_uploaded: RwLock<bool>,
}

#[derive(Serialize)]
pub struct VideoResponse {
    is_uploaded: bool,
    id: String,
}

#[derive(Serialize)]
pub struct VideoListResponse {
    pub videos: Vec<VideoResponse>,
}

pub type StateData = Arc<RwLock<BTreeMap<String, VideoUpload>>>;

#[derive(Clone)]
pub struct AppState {
    pub data: StateData,
}

impl VideoUpload {
    fn new() -> Self {
        let data = RwLock::new(Vec::new());
        let is_uploaded = RwLock::new(false);

        return VideoUpload { data, is_uploaded };
    }
}

#[tokio::main]
async fn main() {
    let storage: StateData = Arc::new(RwLock::new(BTreeMap::new()));
    let state = AppState { data: storage };
    let app = Router::new()
        .route("/upload/:video_id", put(upload_video))
        .route("/download/:video_id", get(download_video))
        .route("/list", get(get_videos_list))
        .route("/delete/:video_id", delete(delete_video))
        .route("/list/:prefix", get(get_videos_list_filter))
        .with_state(state);

    axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
