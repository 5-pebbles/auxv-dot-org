use std::collections::{HashMap, HashSet};

use super::{
    render::{render_card, render_table},
    storage::{PageView, SECONDS_PER_DAY, epoch_to_date, epoch_to_datetime},
};

macro_rules! count_table {
    ($iter:expr, key: |$view:ident| $key:expr, limit: $limit:expr,
     headers: [$($h:expr),+] $(,)?) => {{
        let mut counts: HashMap<_, usize> = HashMap::new();
        for $view in $iter {
            if let Some(key) = $key {
                *counts.entry(key).or_default() += 1;
            }
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|left, right| right.1.cmp(&left.1));
        sorted.truncate($limit);
        let rows = sorted
            .into_iter()
            .map(|(key, count)| vec![key.to_owned(), count.to_string()])
            .collect();
        render_table(&[$($h),+], rows)
    }};
}

macro_rules! traffic_table {
    ($iter:expr, key: |$view:ident| $key:expr, sort: $sort:expr,
     $(limit: $limit:expr,)? headers: [$($h:expr),+] $(,)?) => {{
        let mut groups: HashMap<_, (usize, HashSet<Option<&String>>)> = HashMap::new();
        for $view in $iter {
            let entry = groups.entry($key).or_default();
            entry.0 += 1;
            entry.1.insert($view.visitor_hash.as_ref());
        }
        let mut sorted: Vec<_> = groups.into_iter().collect();
        sorted.sort_by($sort);
        $(sorted.truncate($limit);)?
        let rows = sorted
            .into_iter()
            .map(|(key, (count, visitors))| {
                vec![key.to_owned(), count.to_string(), visitors.len().to_string()]
            })
            .collect();
        render_table(&[$($h),+], rows)
    }};
    ($iter:expr, key: |$view:ident| $key:expr,
     $(limit: $limit:expr,)? headers: [$($h:expr),+] $(,)?) => {
        traffic_table! {
            $iter,
            key: |$view| $key,
            sort: |left, right| right.1 .0.cmp(&left.1 .0),
            $(limit: $limit,)?
            headers: [$($h),+],
        }
    };
}

pub(super) fn render_summary(views: &[PageView], today_in_days: i64) -> String {
    let mut request_count = 0usize;
    let mut unique_visitors: HashSet<Option<&String>> = HashSet::new();
    let mut total_response_time = 0i64;
    let mut error_count = 0usize;

    for view in views
        .iter()
        .filter(|view| view.timestamp / SECONDS_PER_DAY == today_in_days)
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

pub(super) fn render_daily_traffic(views: &[PageView], since: i64) -> String {
    traffic_table! {
        views.iter().filter(|view| view.timestamp >= since),
        key: |view| epoch_to_date(view.timestamp),
        sort: |left, right| right.0.cmp(&left.0),
        headers: ["Date", "Requests", "Unique Visitors"],
    }
}

pub(super) fn render_top_pages(views: &[PageView], since: i64) -> String {
    traffic_table! {
        views.iter()
            .filter(|view| view.timestamp >= since)
            .filter(|view| !view.path.contains('.')),
        key: |view| view.path.as_str(),
        limit: 20,
        headers: ["Path", "Views", "Unique Visitors"],
    }
}

pub(super) fn render_top_referrers(views: &[PageView], since: i64) -> String {
    count_table! {
        views.iter().filter(|view| view.timestamp >= since),
        key: |view| view.referrer.as_deref().filter(|referrer| !referrer.is_empty()),
        limit: 20,
        headers: ["Referrer", "Count"],
    }
}

pub(super) fn render_response_times(views: &[PageView], since: i64) -> String {
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

pub(super) fn render_recent_errors(views: &[PageView]) -> String {
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
                epoch_to_datetime(view.timestamp),
                view.method.clone(),
                view.path.clone(),
                view.status_code.to_string(),
            ]
        })
        .collect();

    render_table(&["Timestamp", "Method", "Path", "Status"], rows)
}

pub(super) fn render_top_user_agents(views: &[PageView]) -> String {
    count_table! {
        views.iter(),
        key: |view| view.user_agent.as_deref().filter(|agent| !agent.is_empty()),
        limit: 10,
        headers: ["User Agent", "Count"],
    }
}
