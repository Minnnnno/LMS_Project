use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;

#[derive(Deserialize)]
pub struct ViewerHeartbeat {
    viewer_id: String,
}

#[derive(Serialize)]
struct ViewerCount {
    active_viewers: usize,
}

#[post("/viewers/heartbeat")]
pub async fn viewer_heartbeat(
    state: web::Data<AppState>,
    body: web::Json<ViewerHeartbeat>,
) -> impl Responder {
    let viewer_id = body.viewer_id.trim();

    if viewer_id.is_empty() || viewer_id.len() > 128 {
        return HttpResponse::BadRequest().body("Invalid viewer ID");
    }

    HttpResponse::Ok().json(ViewerCount {
        active_viewers: state.record_viewer(viewer_id.to_string()),
    })
}

#[post("/viewers/disconnect")]
pub async fn viewer_disconnect(
    state: web::Data<AppState>,
    body: web::Json<ViewerHeartbeat>,
) -> impl Responder {
    let viewer_id = body.viewer_id.trim();

    if viewer_id.is_empty() || viewer_id.len() > 128 {
        return HttpResponse::BadRequest().body("Invalid viewer ID");
    }

    HttpResponse::Ok().json(ViewerCount {
        active_viewers: state.remove_viewer(viewer_id),
    })
}
