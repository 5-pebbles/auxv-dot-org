#[macro_use]
extern crate rocket;

pub mod api;
pub mod emojis;
pub mod pages;

pub fn build_rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .mount("/", routes![api::index, api::html_or_file, api::search])
        .register("/", catchers![api::not_found])
}
