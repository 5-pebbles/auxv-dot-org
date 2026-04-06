mod render;
mod sections;
mod storage;

use std::time::Instant;

use rocket::{
    Data, Request, Response,
    fairing::{Fairing, Info, Kind},
    form::Form,
    get, post,
    response::content::RawHtml,
};
use rsa::{RsaPrivateKey, pkcs8::DecodePrivateKey};
pub use storage::Analytics;
use storage::PageView;
use time::OffsetDateTime;

#[derive(Copy, Clone)]
struct RequestStartTime(Instant);

pub struct AnalyticsFairing;

#[rocket::async_trait]
impl Fairing for AnalyticsFairing {
    fn info(&self) -> Info {
        Info {
            name: "Analytics Tracker",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        request.local_cache(|| RequestStartTime(Instant::now()));
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let path = request.uri().path().to_string();
        if path.starts_with("/analytics") {
            return;
        }

        let Some(analytics) = request.rocket().state::<Analytics>() else {
            return;
        };

        let start_time = request.local_cache(|| RequestStartTime(Instant::now()));

        let view = PageView {
            timestamp: OffsetDateTime::now_utc().unix_timestamp(),
            path,
            method: request.method().as_str().to_owned(),
            status_code: response.status().code,
            response_time_ms: start_time.0.elapsed().as_millis() as i64,
            user_agent: request.headers().get_one("User-Agent").map(String::from),
            referrer: request.headers().get_one("Referer").map(String::from),
            visitor_hash: request
                .client_ip()
                .map(|ip| analytics.hash_visitor(&ip.to_string())),
        };

        analytics.record_page_view(&view);
    }
}

#[derive(FromForm)]
pub struct DecryptForm {
    private_key: String,
}

#[get("/analytics")]
pub fn analytics_form() -> RawHtml<String> {
    RawHtml(render::render_form())
}

#[post("/analytics", data = "<form>")]
pub fn dashboard(analytics: &rocket::State<Analytics>, form: Form<DecryptForm>) -> RawHtml<String> {
    let private_key = match RsaPrivateKey::from_pkcs8_pem(&form.private_key) {
        Ok(key) => key,
        Err(error) => {
            return RawHtml(render::render_error(&format!(
                "Invalid private key: {error}"
            )));
        }
    };

    match analytics.decrypt_all(&private_key) {
        Ok(views) => {
            let now = OffsetDateTime::now_utc().unix_timestamp();
            RawHtml(render::render_dashboard(&views, now))
        }
        Err(message) => RawHtml(render::render_error(&message)),
    }
}
