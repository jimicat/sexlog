use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate, Timelike};
use clap::ValueEnum;

use crate::model::{Kind, Session};

pub const WEEKDAY_LABELS: [&str; 7] = ["一", "二", "三", "四", "五", "六", "日"];
pub const SLOT_LABELS: [&str; 4] = ["0-6", "6-12", "12-18", "18-24"];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum RangeFilter {
    #[value(name = "7d")]
    SevenDays,
    #[value(name = "1m")]
    OneMonth,
    #[value(name = "6m")]
    SixMonths,
    #[value(name = "1y", alias = "12m")]
    #[default]
    OneYear,
    All,
}

impl RangeFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::SevenDays => "近 7 天",
            Self::OneMonth => "近 1 个月",
            Self::SixMonths => "近半年",
            Self::OneYear => "近 1 年",
            Self::All => "全部",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::SevenDays => Self::OneMonth,
            Self::OneMonth => Self::SixMonths,
            Self::SixMonths => Self::OneYear,
            Self::OneYear => Self::All,
            Self::All => Self::SevenDays,
        }
    }

    pub fn start(self, today: NaiveDate) -> Option<NaiveDate> {
        match self {
            Self::SevenDays => today.checked_sub_days(chrono::Days::new(6)),
            Self::OneMonth => Some(minus_months(today, 1)),
            Self::SixMonths => Some(minus_months(today, 6)),
            Self::OneYear => Some(minus_months(today, 12)),
            Self::All => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartnerFilter {
    All,
    Named(String),
    Unspecified,
}

impl PartnerFilter {
    pub fn label(&self) -> &str {
        match self {
            Self::All => "全部",
            Self::Named(name) => name,
            Self::Unspecified => "未指定",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Counted {
    pub label: String,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct MonthCount {
    pub year: i32,
    pub month: u32,
    pub count: u32,
}

impl MonthCount {
    pub fn label(&self) -> String {
        self.month.to_string()
    }

    pub fn heading(&self) -> String {
        format!("{}/{}", self.year, self.month)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Summary {
    pub count: usize,
    pub weekly: Option<f64>,
    pub avg_satisfaction: Option<f64>,
    pub avg_duration: Option<f64>,
    pub avg_interval: Option<f64>,
    pub protection_rate: Option<f64>,
    pub protected_partner: usize,
    pub partner_sessions: usize,
}

#[derive(Debug, Clone)]
pub struct Heatmap {
    pub cells: Vec<[u32; 7]>,
    pub max: u32,
    pub total: u32,
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl Heatmap {
    pub fn weeks(&self) -> usize {
        self.cells.len()
    }
}

pub struct Snapshot<'a> {
    pub today: NaiveDate,
    pub records: Vec<&'a Session>,
    pub summary: Summary,
    pub months: Vec<MonthCount>,
    pub satisfaction: [u32; 5],
    pub weekday_slots: [[u32; 4]; 7],
    pub duration: Vec<(NaiveDate, u32)>,
    pub partners: Vec<Counted>,
    pub kinds: [(Kind, u32); 3],
    pub heatmap: Heatmap,
    pub acts: Vec<Counted>,
}

impl<'a> Snapshot<'a> {
    pub fn build(
        sessions: &'a [Session],
        today: NaiveDate,
        range: RangeFilter,
        kind: Option<Kind>,
        partner: &PartnerFilter,
    ) -> Self {
        let range_start = range.start(today);
        let mut records: Vec<&Session> = sessions
            .iter()
            .filter(|session| matches(session, today, range_start, kind, partner))
            .collect();
        records.sort_by_key(|session| session.occurred_at);

        let summary = summarize(&records, today, range);
        let window_start = range_start
            .or_else(|| records.first().map(|session| session.occurred_at.date()))
            .unwrap_or(today);
        let months = monthly_counts(&records, window_start, today);

        let mut satisfaction = [0u32; 5];
        let mut weekday_slots = [[0u32; 4]; 7];
        for session in &records {
            satisfaction[(session.satisfaction - 1) as usize] += 1;
            let weekday = session.occurred_at.weekday().num_days_from_monday() as usize;
            let slot = (session.occurred_at.hour() / 6) as usize;
            weekday_slots[weekday][slot] += 1;
        }

        let duration = records
            .iter()
            .rev()
            .take(30)
            .rev()
            .map(|session| (session.occurred_at.date(), session.duration_min))
            .collect();

        Self {
            today,
            summary,
            months,
            satisfaction,
            weekday_slots,
            duration,
            partners: partner_counts(&records),
            kinds: kind_counts(&records),
            heatmap: build_heatmap(&records, today, window_start),
            acts: act_counts(&records),
            records,
        }
    }

    pub fn date_span(&self) -> Option<String> {
        let first = self.records.first()?.occurred_at.date();
        let last = self.records.last()?.occurred_at.date();
        if first == last {
            Some(first.format("%Y/%m/%d").to_string())
        } else {
            Some(format!(
                "{} – {}",
                first.format("%Y/%m/%d"),
                last.format("%Y/%m/%d")
            ))
        }
    }
}

pub fn partner_options(sessions: &[Session]) -> Vec<PartnerFilter> {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    let mut has_unnamed = false;
    for session in sessions {
        match session.partner.as_deref() {
            Some(partner) => *counts.entry(partner).or_default() += 1,
            None => has_unnamed = true,
        }
    }

    let mut names: Vec<(&str, u32)> = counts.into_iter().collect();
    names.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));

    let mut options = vec![PartnerFilter::All];
    options.extend(
        names
            .into_iter()
            .map(|(name, _)| PartnerFilter::Named(name.to_string())),
    );
    if has_unnamed {
        options.push(PartnerFilter::Unspecified);
    }
    options
}

fn matches(
    session: &Session,
    today: NaiveDate,
    start: Option<NaiveDate>,
    kind: Option<Kind>,
    partner: &PartnerFilter,
) -> bool {
    let date = session.occurred_at.date();
    if date > today || start.is_some_and(|start| date < start) {
        return false;
    }
    if kind.is_some_and(|kind| session.kind != kind) {
        return false;
    }
    match partner {
        PartnerFilter::All => true,
        PartnerFilter::Named(name) => session.partner.as_deref() == Some(name.as_str()),
        PartnerFilter::Unspecified => session.partner.is_none(),
    }
}

fn summarize(records: &[&Session], today: NaiveDate, range: RangeFilter) -> Summary {
    let count = records.len();
    if count == 0 {
        return Summary::default();
    }

    let start = range
        .start(today)
        .or_else(|| records.first().map(|session| session.occurred_at.date()))
        .unwrap_or(today);
    let days = ((today - start).num_days() + 1).max(7) as f64;

    let avg_satisfaction = records
        .iter()
        .map(|session| session.satisfaction as f64)
        .sum::<f64>()
        / count as f64;
    let avg_duration = records
        .iter()
        .map(|session| session.duration_min as f64)
        .sum::<f64>()
        / count as f64;

    let avg_interval = (count > 1).then(|| {
        records
            .windows(2)
            .map(|pair| (pair[1].occurred_at - pair[0].occurred_at).num_seconds() as f64 / 86_400.0)
            .sum::<f64>()
            / (count - 1) as f64
    });

    let partner_sessions = records
        .iter()
        .filter(|session| session.kind == Kind::Partner)
        .count();
    let protected_partner = records
        .iter()
        .filter(|session| session.kind == Kind::Partner && session.protection)
        .count();
    let protection_rate =
        (partner_sessions > 0).then(|| protected_partner as f64 / partner_sessions as f64 * 100.0);

    Summary {
        count,
        weekly: Some(count as f64 / (days / 7.0)),
        avg_satisfaction: Some(avg_satisfaction),
        avg_duration: Some(avg_duration),
        avg_interval,
        protection_rate,
        protected_partner,
        partner_sessions,
    }
}

fn monthly_counts(records: &[&Session], start: NaiveDate, end: NaiveDate) -> Vec<MonthCount> {
    let start_month = YearMonth::from_date(start);
    let end_month = YearMonth::from_date(end);
    if start_month > end_month {
        return Vec::new();
    }

    let mut counts: HashMap<YearMonth, u32> = HashMap::new();
    for session in records {
        *counts
            .entry(YearMonth::from_date(session.occurred_at.date()))
            .or_default() += 1;
    }

    let mut months = Vec::new();
    let mut cursor = start_month;
    loop {
        months.push(MonthCount {
            year: cursor.year,
            month: cursor.month,
            count: counts.get(&cursor).copied().unwrap_or(0),
        });
        if cursor == end_month {
            break;
        }
        cursor = cursor.next();
    }
    months
}

fn partner_counts(records: &[&Session]) -> Vec<Counted> {
    let mut names: HashMap<&str, u32> = HashMap::new();
    let mut unnamed = 0u32;
    for session in records {
        match session.partner.as_deref() {
            Some(partner) => *names.entry(partner).or_default() += 1,
            None => unnamed += 1,
        }
    }

    let mut items: Vec<Counted> = names
        .into_iter()
        .map(|(label, count)| Counted {
            label: label.to_string(),
            count,
        })
        .collect();
    if unnamed > 0 {
        items.push(Counted {
            label: "未指定".to_string(),
            count: unnamed,
        });
    }
    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items
}

fn kind_counts(records: &[&Session]) -> [(Kind, u32); 3] {
    let mut counts = [0u32; 3];
    for session in records {
        let index = match session.kind {
            Kind::Partner => 0,
            Kind::Solo => 1,
            Kind::Other => 2,
        };
        counts[index] += 1;
    }
    [
        (Kind::Partner, counts[0]),
        (Kind::Solo, counts[1]),
        (Kind::Other, counts[2]),
    ]
}

fn act_counts(records: &[&Session]) -> Vec<Counted> {
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for session in records {
        for act in &session.acts {
            *counts.entry(act.as_str()).or_default() += 1;
        }
    }
    let mut items: Vec<Counted> = counts
        .into_iter()
        .map(|(label, count)| Counted {
            label: label.to_string(),
            count,
        })
        .collect();
    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items
}

fn build_heatmap(records: &[&Session], today: NaiveDate, window_start: NaiveDate) -> Heatmap {
    let start = window_start.min(today);
    let first_monday = start - Duration::days(start.weekday().num_days_from_monday() as i64);
    let this_monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let weeks = (((this_monday - first_monday).num_days() / 7) + 1).max(1) as usize;

    let mut cells = vec![[0u32; 7]; weeks];
    for session in records {
        let date = session.occurred_at.date();
        if date < start || date > today {
            continue;
        }
        let offset = (date - first_monday).num_days();
        let week = (offset / 7) as usize;
        let weekday = (offset % 7) as usize;
        if week < weeks {
            cells[week][weekday] += 1;
        }
    }

    let total = cells.iter().flatten().sum();
    let max = cells.iter().flatten().copied().max().unwrap_or(0);
    Heatmap {
        cells,
        max,
        total,
        start: first_monday,
        end: today,
    }
}

fn minus_months(date: NaiveDate, months: u32) -> NaiveDate {
    let total = date.year() * 12 + date.month0() as i32 - months as i32;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) as u32 + 1;
    let last_day = NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|first| first.checked_add_months(chrono::Months::new(1)))
        .and_then(|next| next.pred_opt())
        .map(|last| last.day())
        .unwrap_or(28);
    NaiveDate::from_ymd_opt(year, month, date.day().min(last_day))
        .expect("clamped month should be valid")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct YearMonth {
    year: i32,
    month: u32,
}

impl YearMonth {
    fn from_date(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    fn next(self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    fn session(
        day: &str,
        hour: u32,
        kind: Kind,
        partner: Option<&str>,
        duration: u32,
        satisfaction: u8,
    ) -> Session {
        Session {
            id: 0,
            occurred_at: date(day).and_hms_opt(hour, 0, 0).unwrap(),
            kind,
            partner: partner.map(ToOwned::to_owned),
            acts: vec!["前戏".to_string()],
            protection: true,
            orgasms: 1,
            satisfaction,
            duration_min: duration,
            notes: None,
        }
    }

    #[test]
    fn month_subtraction_clamps_day() {
        assert_eq!(minus_months(date("2026-05-31"), 3), date("2026-02-28"));
        assert_eq!(minus_months(date("2024-05-31"), 3), date("2024-02-29"));
        assert_eq!(minus_months(date("2026-01-15"), 1), date("2025-12-15"));
    }

    #[test]
    fn range_windows_match_requested_labels() {
        let today = date("2026-11-05");
        assert_eq!(
            RangeFilter::SevenDays.start(today),
            Some(date("2026-10-30"))
        );
        assert_eq!(RangeFilter::OneMonth.start(today), Some(date("2026-10-05")));
        assert_eq!(
            RangeFilter::SixMonths.start(today),
            Some(date("2026-05-05"))
        );
        assert_eq!(RangeFilter::OneYear.start(today), Some(date("2025-11-05")));
        assert_eq!(RangeFilter::All.start(today), None);
        assert_eq!(RangeFilter::SevenDays.label(), "近 7 天");
    }

    #[test]
    fn summary_matches_filtered_records() {
        let sessions = vec![
            session("2026-11-01", 22, Kind::Partner, Some("A"), 30, 5),
            session("2026-11-03", 22, Kind::Solo, None, 50, 3),
            session("2026-10-30", 22, Kind::Partner, Some("B"), 10, 1),
        ];
        let snapshot = Snapshot::build(
            &sessions,
            date("2026-11-05"),
            RangeFilter::OneMonth,
            None,
            &PartnerFilter::All,
        );

        assert_eq!(snapshot.summary.count, 3);
        assert_eq!(snapshot.summary.avg_duration, Some(30.0));
        assert_eq!(snapshot.summary.avg_satisfaction, Some(3.0));
        assert_eq!(snapshot.summary.avg_interval, Some(2.0));
        assert_eq!(snapshot.summary.partner_sessions, 2);
        assert_eq!(snapshot.summary.protected_partner, 2);
        assert_eq!(snapshot.summary.protection_rate, Some(100.0));
    }

    #[test]
    fn heatmap_follows_selected_range() {
        let sessions = vec![
            session("2026-08-16", 22, Kind::Partner, Some("A"), 30, 5),
            session("2026-10-04", 22, Kind::Partner, Some("A"), 30, 5),
            session("2026-11-05", 22, Kind::Partner, Some("A"), 30, 5),
        ];
        let snapshot = Snapshot::build(
            &sessions,
            date("2026-11-05"),
            RangeFilter::OneMonth,
            None,
            &PartnerFilter::All,
        );

        assert_eq!(snapshot.heatmap.total, 1);
        assert_eq!(snapshot.heatmap.max, 1);
        assert_eq!(snapshot.heatmap.start, date("2026-10-05"));
        assert_eq!(snapshot.heatmap.end, date("2026-11-05"));
        assert_eq!(snapshot.heatmap.weeks(), 5);

        let week = Snapshot::build(
            &sessions,
            date("2026-11-05"),
            RangeFilter::SevenDays,
            None,
            &PartnerFilter::All,
        );
        assert_eq!(week.heatmap.total, 1);
        assert_eq!(week.heatmap.weeks(), 2);
    }

    #[test]
    fn monthly_counts_include_empty_months() {
        let sessions = [
            session("2026-09-10", 22, Kind::Partner, Some("A"), 30, 5),
            session("2026-11-01", 22, Kind::Partner, Some("A"), 30, 5),
        ];
        let counts = monthly_counts(
            &sessions.iter().collect::<Vec<_>>(),
            date("2026-09-01"),
            date("2026-11-05"),
        );
        let values: Vec<u32> = counts.iter().map(|month| month.count).collect();
        assert_eq!(values, vec![1, 0, 1]);
        let labels: Vec<String> = counts.iter().map(MonthCount::label).collect();
        assert_eq!(labels, vec!["9", "10", "11"]);
        let headings: Vec<String> = counts.iter().map(MonthCount::heading).collect();
        assert_eq!(headings, vec!["2026/9", "2026/10", "2026/11"]);
    }
}
