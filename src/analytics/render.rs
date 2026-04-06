use std::collections::{HashMap, HashSet};

use time::OffsetDateTime;

use super::{PageView, epoch_to_date};

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

fn render_card(value: &str, label: &str) -> String {
    format!(
        r#"<div class="card"><div class="value">{value}</div><div class="label">{label}</div></div>"#
    )
}

fn render_table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
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

pub fn render_form() -> String {
    let body = include_str!("templates/form.html");
    analytics_page("Analytics", "centered", body)
}

pub fn render_error(message: &str) -> String {
    let body = include_str!("templates/error.html").replace("{{message}}", &escape_html(message));
    analytics_page("Analytics Error", "centered error-page", &body)
}

pub fn render_dashboard(views: &[PageView]) -> String {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let today = epoch_to_date(now);
    let thirty_days_ago = now - 30 * 86400;

    let summary = render_summary(views, &today);
    let daily_traffic = render_daily_traffic(views, thirty_days_ago);
    let top_pages = render_top_pages(views, thirty_days_ago);
    let top_referrers = render_top_referrers(views, thirty_days_ago);
    let response_times = render_response_times(views, thirty_days_ago);
    let recent_errors = render_recent_errors(views);
    let top_user_agents = render_top_user_agents(views);

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

fn render_summary(views: &[PageView], today: &str) -> String {
    let mut request_count = 0usize;
    let mut unique_visitors: HashSet<Option<&String>> = HashSet::new();
    let mut total_response_time = 0i64;
    let mut error_count = 0usize;

    for view in views
        .iter()
        .filter(|view| epoch_to_date(view.timestamp) == *today)
    {
        request_count += 1;
        unique_visitors.insert(view.visitor_hash.as_ref());
        total_response_time += view.response_time_ms;
        if view.status_code >= 400 {
            error_count += 1;
        }
    }

    let avg_response_time = if request_count > 0 {
        total_response_time as f64 / request_count as f64
    } else {
        0.0
    };
    let error_rate = if request_count > 0 {
        error_count as f64 / request_count as f64 * 100.0
    } else {
        0.0
    };
    let visitor_count = unique_visitors.len();

    [
        render_card(&request_count.to_string(), "Requests today"),
        render_card(&visitor_count.to_string(), "Unique visitors today"),
        render_card(&format!("{avg_response_time:.1}ms"), "Avg response time"),
        render_card(&format!("{error_rate:.1}%"), "Error rate"),
    ]
    .join("\n")
}

fn render_daily_traffic(views: &[PageView], since: i64) -> String {
    let mut by_date: HashMap<String, (usize, HashSet<Option<&String>>)> = HashMap::new();
    for view in views.iter().filter(|view| view.timestamp >= since) {
        let date = epoch_to_date(view.timestamp);
        let entry = by_date.entry(date).or_default();
        entry.0 += 1;
        entry.1.insert(view.visitor_hash.as_ref());
    }

    let mut sorted: Vec<_> = by_date.into_iter().collect();
    sorted.sort_by(|left, right| right.0.cmp(&left.0));

    let rows = sorted
        .into_iter()
        .map(|(date, (count, visitors))| vec![date, count.to_string(), visitors.len().to_string()])
        .collect();

    render_table(&["Date", "Requests", "Unique Visitors"], rows)
}

fn render_top_pages(views: &[PageView], since: i64) -> String {
    let mut by_path: HashMap<&str, (usize, HashSet<Option<&String>>)> = HashMap::new();
    for view in views
        .iter()
        .filter(|view| view.timestamp >= since)
        .filter(|view| !view.path.contains('.'))
    {
        let entry = by_path.entry(&view.path).or_default();
        entry.0 += 1;
        entry.1.insert(view.visitor_hash.as_ref());
    }

    let mut sorted: Vec<_> = by_path.into_iter().collect();
    sorted.sort_by(|left, right| right.1.0.cmp(&left.1.0));
    sorted.truncate(20);

    let rows = sorted
        .into_iter()
        .map(|(path, (count, visitors))| {
            vec![
                path.to_owned(),
                count.to_string(),
                visitors.len().to_string(),
            ]
        })
        .collect();

    render_table(&["Path", "Views", "Unique Visitors"], rows)
}

fn render_top_referrers(views: &[PageView], since: i64) -> String {
    let mut by_referrer: HashMap<&str, usize> = HashMap::new();
    for view in views.iter().filter(|view| view.timestamp >= since) {
        if let Some(referrer) = view
            .referrer
            .as_deref()
            .filter(|referrer| !referrer.is_empty())
        {
            *by_referrer.entry(referrer).or_default() += 1;
        }
    }

    let mut sorted: Vec<_> = by_referrer.into_iter().collect();
    sorted.sort_by(|left, right| right.1.cmp(&left.1));
    sorted.truncate(20);

    let rows = sorted
        .into_iter()
        .map(|(referrer, count)| vec![referrer.to_owned(), count.to_string()])
        .collect();

    render_table(&["Referrer", "Count"], rows)
}

fn render_response_times(views: &[PageView], since: i64) -> String {
    let mut by_date: HashMap<String, Vec<i64>> = HashMap::new();
    for view in views.iter().filter(|view| view.timestamp >= since) {
        let date = epoch_to_date(view.timestamp);
        by_date.entry(date).or_default().push(view.response_time_ms);
    }

    let mut sorted: Vec<_> = by_date.into_iter().collect();
    sorted.sort_by(|left, right| right.0.cmp(&left.0));

    let rows = sorted
        .into_iter()
        .map(|(date, times)| {
            let avg = times.iter().sum::<i64>() / times.len() as i64;
            let max = times.iter().copied().max().unwrap_or(0);
            vec![date, avg.to_string(), max.to_string()]
        })
        .collect();

    render_table(&["Date", "Avg (ms)", "Max (ms)"], rows)
}

fn render_recent_errors(views: &[PageView]) -> String {
    let mut errors: Vec<_> = views
        .iter()
        .filter(|view| view.status_code >= 400)
        .collect();
    errors.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    errors.truncate(100);

    let rows = errors
        .into_iter()
        .map(|view| {
            vec![
                {
                    let datetime = OffsetDateTime::from_unix_timestamp(view.timestamp).unwrap();
                    format!(
                        "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                        datetime.year(),
                        datetime.month() as u8,
                        datetime.day(),
                        datetime.hour(),
                        datetime.minute(),
                        datetime.second(),
                    )
                },
                view.method.clone(),
                view.path.clone(),
                view.status_code.to_string(),
            ]
        })
        .collect();

    render_table(&["Timestamp", "Method", "Path", "Status"], rows)
}

fn render_top_user_agents(views: &[PageView]) -> String {
    let mut by_agent: HashMap<&str, usize> = HashMap::new();
    for view in views {
        if let Some(agent) = view.user_agent.as_deref().filter(|agent| !agent.is_empty()) {
            *by_agent.entry(agent).or_default() += 1;
        }
    }

    let mut sorted: Vec<_> = by_agent.into_iter().collect();
    sorted.sort_by(|left, right| right.1.cmp(&left.1));
    sorted.truncate(10);

    let rows = sorted
        .into_iter()
        .map(|(agent, count)| vec![agent.to_owned(), count.to_string()])
        .collect();

    render_table(&["User Agent", "Count"], rows)
}
