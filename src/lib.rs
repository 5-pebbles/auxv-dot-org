#[macro_use]
extern crate rocket;

pub mod analytics;
pub mod api;
pub mod emojis;
pub mod pages;

pub fn build_rocket(analytics: Option<analytics::Analytics>) -> rocket::Rocket<rocket::Build> {
    let mut rocket = rocket::build()
        .mount("/", routes![api::html_or_file, api::search])
        .register("/", catchers![api::not_found]);

    if let Some(analytics) = analytics {
        rocket = rocket
            .mount(
                "/",
                routes![analytics::analytics_form, analytics::dashboard],
            )
            .attach(analytics::AnalyticsFairing)
            .manage(analytics);
    }

    rocket
}
