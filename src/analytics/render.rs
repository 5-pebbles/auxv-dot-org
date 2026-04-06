use super::{
    sections,
    storage::{PageView, SECONDS_PER_DAY},
};

fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn analytics_page(title: &str, body_class: &str, body: &str) -> String {
    include_str!("templates/page.html")
        .replace("{{title}}", title)
        .replace("{{style}}", include_str!("templates/style.css"))
        .replace("{{body_class}}", body_class)
        .replace("{{body}}", body)
}

pub(super) fn render_card(value: &str, label: &str) -> String {
    format!(
        r#"<div class="card"><div class="value">{value}</div><div class="label">{label}</div></div>"#
    )
}

pub(super) fn render_table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    if rows.is_empty() {
        return "<p>No data yet.</p>".to_owned();
    }

    let mut html = String::from("<table><tr>");
    for header in headers {
        html.push_str("<th>");
        html.push_str(header);
        html.push_str("</th>");
    }
    html.push_str("</tr>");

    for row in &rows {
        html.push_str("<tr>");
        for cell in row {
            html.push_str("<td>");
            html.push_str(&escape_html(cell));
            html.push_str("</td>");
        }
        html.push_str("</tr>");
    }
    html.push_str("</table>");
    html
}

pub(super) fn render_form() -> String {
    let body = include_str!("templates/form.html");
    analytics_page("Analytics", "centered", body)
}

pub(super) fn render_error(message: &str) -> String {
    let body = include_str!("templates/error.html").replace("{{message}}", &escape_html(message));
    analytics_page("Analytics Error", "centered error-page", &body)
}

pub(super) fn render_dashboard(views: &[PageView], now: i64) -> String {
    let today_in_days = now / SECONDS_PER_DAY;
    let thirty_days_ago_in_seconds = now - 30 * SECONDS_PER_DAY;

    let summary = sections::render_summary(views, today_in_days);
    let daily_traffic = sections::render_daily_traffic(views, thirty_days_ago_in_seconds);
    let top_pages = sections::render_top_pages(views, thirty_days_ago_in_seconds);
    let top_referrers = sections::render_top_referrers(views, thirty_days_ago_in_seconds);
    let response_times = sections::render_response_times(views, thirty_days_ago_in_seconds);
    let recent_errors = sections::render_recent_errors(views);
    let top_user_agents = sections::render_top_user_agents(views);

    let body = include_str!("templates/dashboard.html")
        .replace("{{summary}}", &summary)
        .replace("{{daily_traffic}}", &daily_traffic)
        .replace("{{top_pages}}", &top_pages)
        .replace("{{top_referrers}}", &top_referrers)
        .replace("{{response_times}}", &response_times)
        .replace("{{recent_errors}}", &recent_errors)
        .replace("{{top_user_agents}}", &top_user_agents);

    analytics_page("Analytics", "", &body)
}
