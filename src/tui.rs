use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Bar, BarChart, Block, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table, Tabs,
};

use crate::analytics::{
    Counted, Heatmap, PartnerFilter, RangeFilter, SLOT_LABELS, Snapshot, WEEKDAY_LABELS,
    partner_options,
};
use crate::cli::TuiArgs;
use crate::model::{Kind, Session};

const ROSE: Color = Color::Rgb(251, 113, 133);
const TEAL: Color = Color::Rgb(45, 212, 191);
const AMBER: Color = Color::Rgb(251, 191, 36);
const BLUE: Color = Color::Rgb(56, 189, 248);
const PURPLE: Color = Color::Rgb(167, 139, 250);
const MUTED: Color = Color::Rgb(138, 149, 168);
const DIM: Color = Color::Rgb(102, 112, 133);
const BORDER: Color = Color::Rgb(55, 60, 75);
const SAT_COLORS: [Color; 5] = [
    Color::Rgb(100, 116, 139),
    Color::Rgb(251, 146, 60),
    AMBER,
    TEAL,
    ROSE,
];

const TABS: [&str; 4] = ["概览", "趋势", "分布", "记录"];

pub struct App {
    sessions: Vec<Session>,
    today: NaiveDate,
    range: RangeFilter,
    kind: Option<Kind>,
    partner: PartnerFilter,
    partner_options: Vec<PartnerFilter>,
    tab: usize,
}

impl App {
    fn new(
        mut sessions: Vec<Session>,
        range: RangeFilter,
        kind: Option<Kind>,
        partner: PartnerFilter,
    ) -> Self {
        sessions.sort_by_key(|session| session.occurred_at);
        let mut options = partner_options(&sessions);
        if matches!(partner, PartnerFilter::Named(_)) && !options.contains(&partner) {
            options.insert(1, partner.clone());
        }

        Self {
            sessions,
            today: Local::now().date_naive(),
            range,
            kind,
            partner,
            partner_options: options,
            tab: 0,
        }
    }

    fn snapshot(&self) -> Snapshot<'_> {
        Snapshot::build(
            &self.sessions,
            self.today,
            self.range,
            self.kind,
            &self.partner,
        )
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return false;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return false,
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => self.next_tab(),
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => self.prev_tab(),
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => self.tab = 2,
            KeyCode::Char('4') => self.tab = 3,
            KeyCode::Char('r') => self.range = self.range.next(),
            KeyCode::Char('k') => {
                self.kind = match self.kind {
                    None => Some(Kind::Partner),
                    Some(Kind::Partner) => Some(Kind::Solo),
                    Some(Kind::Solo) => Some(Kind::Other),
                    Some(Kind::Other) => None,
                };
            }
            KeyCode::Char('p') => self.cycle_partner(),
            KeyCode::Char('c') => {
                self.range = RangeFilter::OneYear;
                self.kind = None;
                self.partner = PartnerFilter::All;
            }
            _ => {}
        }
        true
    }

    fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % TABS.len();
    }

    fn prev_tab(&mut self) {
        self.tab = (self.tab + TABS.len() - 1) % TABS.len();
    }

    fn cycle_partner(&mut self) {
        if self.partner_options.is_empty() {
            return;
        }
        let next = self
            .partner_options
            .iter()
            .position(|option| option == &self.partner)
            .map(|index| (index + 1) % self.partner_options.len())
            .unwrap_or(0);
        self.partner = self.partner_options[next].clone();
    }
}

pub fn run(sessions: Vec<Session>, args: TuiArgs) -> Result<()> {
    let partner = match args.partner.as_deref() {
        None => PartnerFilter::All,
        Some("__none__") => PartnerFilter::Unspecified,
        Some(name) => PartnerFilter::Named(name.to_string()),
    };
    let app = App::new(sessions, args.range, args.kind, partner);

    let mut terminal =
        ratatui::try_init().context("无法初始化终端，请在交互式终端中运行 sexlog tui")?;
    let result = run_app(&mut terminal, app);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut ratatui::DefaultTerminal, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && !app.handle_key(key)
        {
            break;
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < 72 || area.height < 24 {
        frame.render_widget(
            Paragraph::new("终端窗口过小，请调整到至少 72×24")
                .alignment(Alignment::Center)
                .style(Style::new().fg(MUTED)),
            area,
        );
        return;
    }

    let snapshot = app.snapshot();
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(frame, rows[0], app, &snapshot);
    draw_tabs(frame, rows[1], app);

    match app.tab {
        0 => draw_overview(frame, rows[2], &snapshot),
        1 => draw_trends(frame, rows[2], &snapshot),
        2 => draw_distribution(frame, rows[2], &snapshot),
        _ => draw_records(frame, rows[2], &snapshot),
    }

    draw_footer(frame, rows[3]);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App, snapshot: &Snapshot<'_>) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
    let top = Layout::horizontal([Constraint::Min(0), Constraint::Length(14)]).split(rows[0]);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("sexlog", Style::new().fg(ROSE).add_modifier(Modifier::BOLD)),
            Span::styled("  终端数据看板", Style::new().fg(MUTED)),
        ])),
        top[0],
    );
    frame.render_widget(
        Paragraph::new(snapshot.today.format("%Y-%m-%d").to_string())
            .alignment(Alignment::Right)
            .style(Style::new().fg(DIM)),
        top[1],
    );

    let mut spans = vec![
        Span::styled("范围 ", Style::new().fg(DIM)),
        Span::styled(app.range.label().to_string(), Style::new().fg(ROSE)),
        Span::styled("  ·  类型 ", Style::new().fg(DIM)),
        Span::styled(kind_label(app.kind), Style::new().fg(TEAL)),
        Span::styled("  ·  伴侣 ", Style::new().fg(DIM)),
        Span::styled(app.partner.label().to_string(), Style::new().fg(AMBER)),
        Span::styled(
            format!("  ·  {} 条记录", snapshot.summary.count),
            Style::new().fg(MUTED),
        ),
    ];
    if let Some(span) = snapshot.date_span() {
        spans.push(Span::styled(format!("  ·  {span}"), Style::new().fg(DIM)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), rows[1]);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let tabs = Tabs::new(TABS)
        .select(app.tab)
        .divider("  ")
        .highlight_style(Style::new().fg(ROSE).add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new("q 退出  ←/→ 切换  r 范围  k 类型  p 伴侣  c 重置")
            .style(Style::new().fg(DIM)),
        area,
    );
}

fn draw_overview(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let rows = Layout::vertical([Constraint::Length(8), Constraint::Min(0)]).split(area);
    let cards = Layout::vertical([Constraint::Length(4), Constraint::Length(4)]).split(rows[0]);

    let top = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .spacing(1)
    .split(cards[0]);
    let bottom = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .spacing(1)
    .split(cards[1]);

    let summary = &snapshot.summary;
    let count_sub = if summary.count == 0 {
        "暂无记录"
    } else {
        "筛选范围内"
    };
    frame.render_widget(
        kpi_card(
            "次数",
            summary.count.to_string(),
            count_sub.to_string(),
            ROSE,
        ),
        top[0],
    );
    frame.render_widget(
        kpi_card(
            "每周频率",
            format_opt(summary.weekly, |value| format!("{value:.1}")),
            "按所选时间范围".to_string(),
            TEAL,
        ),
        top[1],
    );
    frame.render_widget(
        kpi_card(
            "平均满意度",
            format_opt(summary.avg_satisfaction, |value| format!("{value:.1}")),
            "满分 5 分".to_string(),
            AMBER,
        ),
        top[2],
    );
    frame.render_widget(
        kpi_card(
            "平均时长",
            summary
                .avg_duration
                .map(|value| format!("{value:.0} 分钟"))
                .unwrap_or_else(|| "—".to_string()),
            if summary.count == 0 {
                "暂无数据".to_string()
            } else {
                format!("单次平均 · {} 次", summary.count)
            },
            BLUE,
        ),
        bottom[0],
    );
    frame.render_widget(
        kpi_card(
            "平均间隔",
            summary
                .avg_interval
                .map(|value| format!("{value:.1} 天"))
                .unwrap_or_else(|| "—".to_string()),
            if summary.count > 1 {
                format!("相邻两次 · {} 个间隔", summary.count - 1)
            } else {
                "暂无数据".to_string()
            },
            PURPLE,
        ),
        bottom[1],
    );
    frame.render_widget(
        kpi_card(
            "措施使用率",
            summary
                .protection_rate
                .map(|value| format!("{value:.0}%"))
                .unwrap_or_else(|| "—".to_string()),
            format!(
                "{}/{} 伴侣场次",
                summary.protected_partner, summary.partner_sessions
            ),
            ROSE,
        ),
        bottom[2],
    );

    let charts = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .spacing(1)
        .split(rows[1]);
    draw_monthly(frame, charts[0], snapshot, "月度次数");
    draw_heatmap(frame, charts[1], &snapshot.heatmap);
}

fn draw_trends(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let rows = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
    draw_duration(frame, rows[0], snapshot);
    draw_monthly(frame, rows[1], snapshot, "月度次数");
}

fn draw_distribution(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let rows = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
    let top = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .spacing(1)
    .split(rows[0]);
    let bottom = Layout::horizontal([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)])
        .spacing(1)
        .split(rows[1]);

    draw_satisfaction(frame, top[0], snapshot);
    draw_partners(frame, top[1], snapshot);
    draw_kinds(frame, top[2], snapshot);
    draw_weekday(frame, bottom[0], snapshot);
    draw_acts(frame, bottom[1], snapshot);
}

fn draw_monthly(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>, title: &str) {
    if snapshot.records.is_empty() || snapshot.months.is_empty() {
        let title = format!("{title} · 共 {} 次", snapshot.summary.count);
        draw_empty(frame, area, title, "当前筛选下暂无月度数据");
        return;
    }

    let available = area.width.saturating_sub(6) as usize / 3;
    let limit = available.clamp(3, 13);
    let start = snapshot.months.len().saturating_sub(limit);
    let months = &snapshot.months[start..];
    let span = match (months.first(), months.last()) {
        (Some(first), Some(last)) if first.year == last.year && first.month == last.month => {
            first.heading()
        }
        (Some(first), Some(last)) => format!("{} – {}", first.heading(), last.heading()),
        _ => String::new(),
    };
    let title = format!("{title} · {span} · 共 {} 次", snapshot.summary.count);

    let bars: Vec<Bar> = months
        .iter()
        .map(|month| {
            Bar::with_label(month.label(), month.count as u64)
                .style(Style::new().fg(ROSE))
                .value_style(Style::new().fg(MUTED))
        })
        .collect();
    let chart = BarChart::vertical(bars)
        .block(panel(title))
        .bar_width(1)
        .bar_gap(1);
    frame.render_widget(chart, area);
}

fn draw_duration(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let title = format!("时长趋势 · 最近 {} 次", snapshot.duration.len());
    if snapshot.duration.is_empty() {
        draw_empty(frame, area, title, "当前筛选下暂无时长数据");
        return;
    }

    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let x_max = (snapshot.duration.len() - 1).max(1) as f64;
    let max_y = snapshot
        .duration
        .iter()
        .map(|(_, duration)| *duration as f64)
        .fold(0.0f64, f64::max);
    let y_max = (max_y / 10.0).ceil().max(1.0) * 10.0;

    let points: Vec<(f64, f64)> = snapshot
        .duration
        .iter()
        .enumerate()
        .map(|(index, (_, duration))| (index as f64, *duration as f64))
        .collect();

    let dataset = Dataset::default()
        .name("分钟")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::new().fg(ROSE))
        .data(&points);

    let x_labels = match snapshot.duration.len() {
        1 => vec![Span::raw(
            snapshot.duration[0].0.format("%m-%d").to_string(),
        )],
        2 => vec![
            Span::raw(snapshot.duration[0].0.format("%m-%d").to_string()),
            Span::raw(snapshot.duration[1].0.format("%m-%d").to_string()),
        ],
        _ => {
            let middle = snapshot.duration.len() / 2;
            vec![
                Span::raw(snapshot.duration[0].0.format("%m-%d").to_string()),
                Span::raw(snapshot.duration[middle].0.format("%m-%d").to_string()),
                Span::raw(
                    snapshot
                        .duration
                        .last()
                        .unwrap()
                        .0
                        .format("%m-%d")
                        .to_string(),
                ),
            ]
        }
    };

    let y_labels = vec![
        Span::raw("0"),
        Span::raw(format!("{:.0}", y_max / 2.0)),
        Span::raw(format!("{y_max:.0}")),
    ];

    let chart = Chart::new(vec![dataset])
        .x_axis(
            Axis::default()
                .bounds([0.0, x_max])
                .labels(x_labels)
                .style(Style::new().fg(DIM)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max])
                .labels(y_labels)
                .style(Style::new().fg(DIM)),
        );
    frame.render_widget(chart, inner);
}

fn draw_satisfaction(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let title = match snapshot.summary.avg_satisfaction {
        Some(average) => format!("满意度 · 均 {average:.1}"),
        None => "满意度".to_string(),
    };
    if snapshot.records.is_empty() {
        draw_empty(frame, area, title, "当前筛选下暂无记录");
        return;
    }

    let bars: Vec<Bar> = (1..=5)
        .rev()
        .map(|score| {
            Bar::with_label(
                format!("{score} 分"),
                snapshot.satisfaction[score - 1] as u64,
            )
            .style(Style::new().fg(SAT_COLORS[score - 1]))
        })
        .collect();
    let chart = BarChart::horizontal(bars)
        .block(panel(title))
        .bar_width(1)
        .bar_gap(0)
        .value_style(Style::new().fg(MUTED));
    frame.render_widget(chart, area);
}

fn draw_partners(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let title = if snapshot.records.is_empty() {
        "伴侣分布".to_string()
    } else {
        format!("伴侣分布 · {} 位", named_partner_count(&snapshot.partners))
    };
    if snapshot.partners.is_empty() {
        draw_empty(frame, area, title, "当前筛选下暂无记录");
        return;
    }

    let bars: Vec<Bar> = snapshot
        .partners
        .iter()
        .take(6)
        .map(|partner| {
            Bar::with_label(truncate_width(&partner.label, 6), partner.count as u64)
                .style(Style::new().fg(TEAL))
        })
        .collect();
    let chart = BarChart::horizontal(bars)
        .block(panel(title))
        .bar_width(1)
        .bar_gap(0)
        .value_style(Style::new().fg(MUTED));
    frame.render_widget(chart, area);
}

fn draw_kinds(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    if snapshot.records.is_empty() {
        draw_empty(frame, area, "类型分布".to_string(), "当前筛选下暂无记录");
        return;
    }

    let bars: Vec<Bar> = snapshot
        .kinds
        .iter()
        .map(|(kind, count)| {
            let color = match kind {
                Kind::Partner => ROSE,
                Kind::Solo => BLUE,
                Kind::Other => PURPLE,
            };
            Bar::with_label(kind.label(), *count as u64).style(Style::new().fg(color))
        })
        .collect();
    let chart = BarChart::horizontal(bars)
        .block(panel("类型分布".to_string()))
        .bar_width(1)
        .bar_gap(0)
        .value_style(Style::new().fg(MUTED));
    frame.render_widget(chart, area);
}

fn draw_weekday(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let peak = snapshot
        .weekday_slots
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(0);
    let title = format!("星期 / 时段 · 峰值 {peak} 次");
    if snapshot.records.is_empty() {
        draw_empty(frame, area, title, "当前筛选下暂无记录");
        return;
    }

    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!(
                "{:<5}{:<5}{:<5}{:<5}",
                SLOT_LABELS[0], SLOT_LABELS[1], SLOT_LABELS[2], SLOT_LABELS[3]
            ),
            Style::new().fg(DIM),
        ),
    ])];
    for (weekday, label) in WEEKDAY_LABELS.iter().enumerate() {
        let mut spans = vec![Span::styled(format!("{label} "), Style::new().fg(MUTED))];
        for slot in 0..4 {
            spans.push(Span::styled(
                "██   ",
                heat_style(snapshot.weekday_slots[weekday][slot], peak),
            ));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_heatmap(frame: &mut Frame, area: Rect, heatmap: &Heatmap) {
    let capacity = area.width.saturating_sub(4) as usize;
    let total_weeks = heatmap.weeks();
    let shown = total_weeks.min(capacity.max(1));
    let offset = total_weeks - shown;
    let (visible_max, visible_total) = if offset == 0 {
        (heatmap.max, heatmap.total)
    } else {
        (
            heatmap.cells[offset..]
                .iter()
                .flatten()
                .copied()
                .max()
                .unwrap_or(0),
            heatmap.cells[offset..].iter().flatten().sum(),
        )
    };
    let span = if shown <= 1 {
        "本周".to_string()
    } else {
        format!("最近 {shown} 周")
    };
    let dates = format!(
        "{}–{}",
        heatmap.start.format("%m/%d"),
        heatmap.end.format("%m/%d")
    );
    let title = if shown < total_weeks {
        format!("{span} · {dates} · 共 {visible_total} 次 / 全部 {total_weeks} 周")
    } else {
        format!("{span} · {dates} · 共 {visible_total} 次")
    };

    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    for (weekday, label) in WEEKDAY_LABELS.iter().enumerate() {
        let mut spans = vec![Span::styled(format!("{label} "), Style::new().fg(MUTED))];
        for week in offset..total_weeks {
            let count = heatmap.cells[week][weekday];
            spans.push(Span::styled("■", heat_style(count, visible_max)));
        }
        lines.push(Line::from(spans));
    }
    if inner.height >= 9 {
        lines.push(Line::from(vec![
            Span::styled("少 ", Style::new().fg(DIM)),
            Span::styled("■", heat_style(1, 4)),
            Span::styled("■", heat_style(2, 4)),
            Span::styled("■", heat_style(3, 4)),
            Span::styled("■", heat_style(4, 4)),
            Span::styled(" 多", Style::new().fg(DIM)),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_acts(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let title = format!("行为标签 · {} 个", snapshot.acts.len());
    if snapshot.acts.is_empty() {
        draw_empty(frame, area, title, "当前筛选下暂无行为标签");
        return;
    }

    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let max_count = snapshot.acts[0].count.max(1);
    let bar_width = (inner.width as usize).saturating_sub(14).clamp(4, 30);
    let mut lines = Vec::new();
    for act in snapshot
        .acts
        .iter()
        .take(inner.height.saturating_sub(1) as usize)
    {
        let filled = ((act.count as f64 / max_count as f64) * bar_width as f64).round() as usize;
        lines.push(Line::from(vec![
            Span::styled(pad_width(&act.label, 10), Style::new().fg(TEAL)),
            Span::styled("█".repeat(filled.max(1)), Style::new().fg(TEAL)),
            Span::styled(format!(" {}", act.count), Style::new().fg(MUTED)),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_records(frame: &mut Frame, area: Rect, snapshot: &Snapshot<'_>) {
    let title = format!("最近记录 · 共 {} 条", snapshot.summary.count);
    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if snapshot.records.is_empty() {
        empty_message(frame, inner, "当前筛选下暂无记录");
        return;
    }

    let header = Row::new([
        "日期", "时间", "类型", "伴侣", "行为", "满意", "时长", "措施",
    ])
    .style(Style::new().fg(MUTED).add_modifier(Modifier::BOLD));
    let rows: Vec<Row> = snapshot
        .records
        .iter()
        .rev()
        .take(inner.height.saturating_sub(1) as usize)
        .map(|session| {
            Row::new(vec![
                Cell::from(session.occurred_at.format("%Y-%m-%d").to_string()),
                Cell::from(session.occurred_at.format("%H:%M").to_string()),
                Cell::from(session.kind.label()),
                Cell::from(session.partner.clone().unwrap_or_else(|| "—".to_string())),
                Cell::from(session.acts.join(" · ")),
                Cell::from(format!("{}/5", session.satisfaction)),
                Cell::from(format!("{} 分", session.duration_min)),
                Cell::from(if session.protection { "是" } else { "否" }),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(4),
        ],
    )
    .header(header)
    .column_spacing(1);
    frame.render_widget(table, inner);
}

fn kpi_card(title: &str, value: String, sub: String, accent: Color) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(Span::styled(
            value,
            Style::new().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(sub, Style::new().fg(MUTED))),
    ])
    .block(
        Block::bordered()
            .title(Line::from(Span::styled(
                format!(" {title} "),
                Style::new().fg(DIM),
            )))
            .border_style(Style::new().fg(BORDER)),
    )
}

fn panel(title: String) -> Block<'static> {
    Block::bordered()
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Style::new().fg(MUTED).add_modifier(Modifier::BOLD),
        )))
        .border_style(Style::new().fg(BORDER))
}

fn draw_empty(frame: &mut Frame, area: Rect, title: String, message: &str) {
    let block = panel(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    empty_message(frame, inner, message);
}

fn empty_message(frame: &mut Frame, area: Rect, message: &str) {
    let rows = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(Style::new().fg(DIM)),
        rows[1],
    );
}

fn kind_label(kind: Option<Kind>) -> String {
    kind.map(|kind| kind.label().to_string())
        .unwrap_or_else(|| "全部".to_string())
}

fn format_opt(value: Option<f64>, render: impl FnOnce(f64) -> String) -> String {
    value.map(render).unwrap_or_else(|| "—".to_string())
}

fn named_partner_count(partners: &[Counted]) -> usize {
    partners
        .iter()
        .filter(|partner| partner.label != "未指定")
        .count()
}

fn heat_style(count: u32, max: u32) -> Style {
    if count == 0 {
        return Style::new().fg(Color::Rgb(42, 46, 58));
    }
    let ratio = count as f64 / max.max(1) as f64;
    let color = if ratio <= 0.25 {
        Color::Rgb(22, 76, 65)
    } else if ratio <= 0.5 {
        Color::Rgb(27, 122, 99)
    } else if ratio <= 0.75 {
        Color::Rgb(45, 180, 140)
    } else {
        Color::Rgb(94, 234, 180)
    };
    Style::new().fg(color)
}

fn truncate_width(value: &str, max_width: usize) -> String {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    if UnicodeWidthStr::width(value) <= max_width {
        return value.to_string();
    }
    let mut output = String::new();
    let mut width = 0;
    for character in value.chars() {
        let char_width = character.width().unwrap_or(0);
        if width + char_width > max_width {
            break;
        }
        output.push(character);
        width += char_width;
    }
    output.push('…');
    output
}

fn pad_width(value: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    let mut output = String::new();
    let mut used = 0;
    for character in value.chars() {
        let char_width = character.width().unwrap_or(0);
        if used + char_width > width {
            break;
        }
        output.push(character);
        used += char_width;
    }
    output.push_str(&" ".repeat(width.saturating_sub(used)));
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn session(day: &str, satisfaction: u8, duration: u32) -> Session {
        let occurred_at = NaiveDate::parse_from_str(day, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(22, 0, 0)
            .unwrap();
        Session {
            id: 0,
            occurred_at,
            kind: Kind::Partner,
            partner: Some("A".to_string()),
            acts: vec!["前戏".to_string()],
            protection: true,
            orgasms: 1,
            satisfaction,
            duration_min: duration,
            notes: None,
        }
    }

    fn render(app: &App) {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
    }

    fn render_text(app: &App) -> String {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn renders_every_tab_and_range() {
        let sessions = vec![
            session("2024-06-15", 5, 40),
            session("2026-11-01", 5, 40),
            session("2026-11-03", 4, 30),
        ];
        let ranges = [
            (RangeFilter::SevenDays, "近 7 天"),
            (RangeFilter::OneMonth, "近 1 个月"),
            (RangeFilter::SixMonths, "近半年"),
            (RangeFilter::OneYear, "近 1 年"),
            (RangeFilter::All, "全部"),
        ];
        for (range, label) in ranges {
            let mut app = App::new(sessions.clone(), range, None, PartnerFilter::All);
            for tab in 0..TABS.len() {
                app.tab = tab;
                render(&app);
            }
            app.tab = 0;
            let compact: String = render_text(&app)
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect();
            let expected = label.replace(' ', "");
            assert!(compact.contains(&expected), "{label} 未显示");
        }
    }

    #[test]
    fn range_cycling_wraps() {
        let mut app = App::new(vec![], RangeFilter::OneYear, None, PartnerFilter::All);
        for _ in 0..5 {
            app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        }
        assert_eq!(app.range, RangeFilter::OneYear);
    }

    #[test]
    fn partner_cycling_wraps() {
        let mut app = App::new(
            vec![
                session("2026-11-01", 5, 40),
                Session {
                    partner: None,
                    ..session("2026-11-02", 4, 30)
                },
            ],
            RangeFilter::OneYear,
            None,
            PartnerFilter::All,
        );
        let mut seen = Vec::new();
        for _ in 0..app.partner_options.len() {
            app.cycle_partner();
            seen.push(app.partner.clone());
        }
        assert_eq!(seen.first(), Some(&PartnerFilter::Named("A".to_string())));
        assert_eq!(seen.last(), Some(&PartnerFilter::All));
    }
}
