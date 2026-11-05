(() => {
  'use strict';

  const SVG_NS = 'http://www.w3.org/2000/svg';
  const DAY = 86400000;
  const KIND_LABEL = { partner: '伴侣', solo: '独自', other: '其他' };
  const KIND_COLOR = { partner: '#fb7185', solo: '#38bdf8', other: '#a78bfa' };
  const WEEKDAYS = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];
  const SLOTS = [
    { label: '深夜', range: '0-6' },
    { label: '早晨', range: '6-12' },
    { label: '下午', range: '12-18' },
    { label: '夜晚', range: '18-24' },
  ];
  const SAT_COLORS = ['#64748b', '#fb923c', '#fbbf24', '#2dd4bf', '#fb7185'];
  const COLORS = {
    grid: 'rgba(255,255,255,.075)',
    muted: '#8a95a8',
    muted2: '#667085',
    rose: '#fb7185',
    teal: '#2dd4bf',
    amber: '#fbbf24',
  };

  const payload = JSON.parse(document.getElementById('sexlog-data').textContent);
  const RECORDS = payload.records
    .map((record) => {
      const date = parseDateKey(record.date);
      const [hour, minute] = record.time.split(':').map(Number);
      return {
        ...record,
        hour,
        dateObj: date,
        dt: new Date(
          date.getFullYear(),
          date.getMonth(),
          date.getDate(),
          hour,
          minute
        ),
        weekday: (date.getDay() + 6) % 7,
      };
    })
    .sort((left, right) => left.dt - right.dt);

  const TODAY = parseDateKey(payload.today);
  TODAY.setHours(23, 59, 59, 999);

  const state = { range: '1y', kind: 'all', partner: 'all' };

  function $(id) {
    return document.getElementById(id);
  }

  function parseDateKey(key) {
    const [year, month, day] = key.split('-').map(Number);
    return new Date(year, month - 1, day);
  }

  function dateKey(date) {
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${date.getFullYear()}-${month}-${day}`;
  }

  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined && text !== null) node.textContent = text;
    return node;
  }

  function svgEl(tag, attrs, parent) {
    const node = document.createElementNS(SVG_NS, tag);
    if (attrs) {
      for (const [key, value] of Object.entries(attrs)) {
        node.setAttribute(key, String(value));
      }
    }
    if (parent) parent.appendChild(node);
    return node;
  }

  function svgText(parent, x, y, text, attrs) {
    const node = svgEl('text', { x, y, ...(attrs || {}) }, parent);
    node.textContent = text;
    return node;
  }

  function svgTitle(parent, text) {
    const node = svgEl('title', {}, parent);
    node.textContent = text;
    return node;
  }

  function clear(node) {
    node.replaceChildren();
  }

  function emptyState(node, message) {
    clear(node);
    node.appendChild(el('div', 'empty', message));
  }

  function fmt(value, digits = 1) {
    if (value === null || value === undefined || !Number.isFinite(value)) {
      return '—';
    }
    return value
      .toFixed(digits)
      .replace(/\.0+$/, '')
      .replace(/(\.\d*?)0+$/, '$1');
  }

  function fmtRate(value) {
    if (value === null || !Number.isFinite(value)) return '—';
    return value >= 1 ? fmt(value, 1) : fmt(value, 2);
  }

  function setText(id, text) {
    const node = $(id);
    if (node) node.textContent = text;
  }

  function minusMonths(date, months) {
    const next = new Date(date);
    const day = next.getDate();
    next.setDate(1);
    next.setMonth(next.getMonth() - months);
    const lastDay = new Date(next.getFullYear(), next.getMonth() + 1, 0).getDate();
    next.setDate(Math.min(day, lastDay));
    next.setHours(0, 0, 0, 0);
    return next;
  }

  function startOfMonth(date) {
    return new Date(date.getFullYear(), date.getMonth(), 1);
  }

  function addDays(date, days) {
    const next = new Date(date);
    next.setDate(next.getDate() + days);
    return next;
  }

  function rangeStart() {
    switch (state.range) {
      case '7d': {
        const start = addDays(TODAY, -6);
        start.setHours(0, 0, 0, 0);
        return start;
      }
      case '1m':
        return minusMonths(TODAY, 1);
      case '6m':
        return minusMonths(TODAY, 6);
      case '1y':
        return minusMonths(TODAY, 12);
      default:
        return null;
    }
  }

  function filtered(applyRange = true) {
    const start = applyRange ? rangeStart() : null;
    return RECORDS.filter((record) => {
      if (record.dt > TODAY) return false;
      if (start && record.dt < start) return false;
      if (state.kind !== 'all' && record.kind !== state.kind) return false;
      if (state.partner !== 'all') {
        if (state.partner === '__none__') {
          if (record.partner) return false;
        } else if (record.partner !== state.partner) {
          return false;
        }
      }
      return true;
    });
  }

  function total(records) {
    return records.reduce((sum, record) => sum + record.duration, 0);
  }

  function initFilters() {
    const partnerSelect = $('filter-partner');
    const partners = [...new Set(RECORDS.map((record) => record.partner).filter(Boolean))]
      .sort((left, right) => left.localeCompare(right, 'zh-CN'));

    for (const partner of partners) {
      const option = el('option', '', partner);
      option.value = partner;
      partnerSelect.appendChild(option);
    }
    if (RECORDS.some((record) => !record.partner)) {
      const option = el('option', '', '未指定');
      option.value = '__none__';
      partnerSelect.appendChild(option);
    }

    $('filter-range').addEventListener('change', (event) => {
      state.range = event.target.value;
      renderAll();
    });
    $('filter-kind').addEventListener('change', (event) => {
      state.kind = event.target.value;
      renderAll();
    });
    partnerSelect.addEventListener('change', (event) => {
      state.partner = event.target.value;
      renderAll();
    });
  }

  function renderAll() {
    const records = filtered();
    renderFilterMeta(records);
    renderKpis(records);
    renderMonthly(records);
    renderSatisfaction(records);
    renderWeekday(records);
    renderDuration(records);
    renderPartner(records);
    renderHeatmap(records);
    renderActs(records);
    renderRecent(records);
  }

  function renderFilterMeta(records) {
    setText('filter-count', `${records.length} 条记录`);
    if (!records.length) {
      setText('filter-span', '');
      return;
    }
    const first = records[0].dateObj;
    const last = records[records.length - 1].dateObj;
    const sameYear = first.getFullYear() === last.getFullYear();
    const format = (date) =>
      sameYear
        ? `${date.getMonth() + 1}月${date.getDate()}日`
        : `${date.getFullYear()}/${date.getMonth() + 1}/${date.getDate()}`;
    setText(
      'filter-span',
      first.getTime() === last.getTime()
        ? format(first)
        : `${format(first)} – ${format(last)}`
    );
  }

  function renderKpis(records) {
    const count = records.length;
    const hasData = count > 0;

    setText('kpi-count', hasData ? count.toLocaleString('zh-CN') : '0');
    setText('kpi-count-sub', hasData ? '筛选范围内' : '暂无记录');

    let weekly = null;
    if (hasData) {
      const start = rangeStart() || records[0].dateObj;
      const days = Math.max(7, Math.floor((TODAY - start) / DAY) + 1);
      weekly = count / (days / 7);
    }
    setText('kpi-weekly', fmtRate(weekly));
    setText('kpi-weekly-sub', hasData ? '按所选时间范围' : '暂无数据');

    const avgSatisfaction = hasData
      ? records.reduce((sum, record) => sum + record.satisfaction, 0) / count
      : null;
    setText('kpi-satisfaction', fmt(avgSatisfaction));
    setText('kpi-satisfaction-sub', hasData ? '满分 5 分' : '暂无数据');

    const avgDuration = hasData ? total(records) / count : null;
    setText('kpi-duration', fmt(avgDuration));
    setText('kpi-duration-sub', hasData ? `单次平均 · ${count} 次` : '暂无数据');

    let avgInterval = null;
    if (count > 1) {
      let sum = 0;
      for (let index = 1; index < count; index += 1) {
        sum += (records[index].dt - records[index - 1].dt) / DAY;
      }
      avgInterval = sum / (count - 1);
    }
    setText('kpi-interval', fmt(avgInterval));
    setText(
      'kpi-interval-sub',
      avgInterval === null ? '暂无数据' : `相邻两次 · ${count - 1} 个间隔`
    );

    const partnerRecords = records.filter((record) => record.kind === 'partner');
    const protectedCount = partnerRecords.filter((record) => record.protection).length;
    const protectionRate = partnerRecords.length
      ? (protectedCount / partnerRecords.length) * 100
      : null;
    setText(
      'kpi-protection',
      protectionRate === null ? '—' : String(Math.round(protectionRate))
    );
    setText(
      'kpi-protection-sub',
      partnerRecords.length
        ? `${protectedCount}/${partnerRecords.length} 伴侣场次`
        : '伴侣场次 · 0 次'
    );
  }

  function renderMonthly(records) {
    const chart = $('monthly-chart');
    if (!records.length) {
      emptyState(chart, '当前筛选下暂无月度数据');
      setText('monthly-total', '—');
      setText('monthly-sub', '按自然月统计');
      return;
    }

    const firstMonth = startOfMonth(
      rangeStart() ? rangeStart() : records[0].dateObj
    );
    const lastMonth = startOfMonth(TODAY);
    const months = [];
    for (
      let cursor = new Date(firstMonth);
      cursor <= lastMonth;
      cursor = new Date(cursor.getFullYear(), cursor.getMonth() + 1, 1)
    ) {
      months.push(new Date(cursor));
    }

    const indexByKey = new Map(
      months.map((month, index) => [
        `${month.getFullYear()}-${String(month.getMonth() + 1).padStart(2, '0')}`,
        index,
      ])
    );
    const counts = months.map(() => 0);
    for (const record of records) {
      const index = indexByKey.get(record.date.slice(0, 7));
      if (index !== undefined) counts[index] += 1;
    }

    const totalCount = counts.reduce((sum, value) => sum + value, 0);
    const peak = Math.max(...counts);
    const first = months[0];
    const last = months[months.length - 1];
    const firstLabel = `${first.getFullYear()}/${first.getMonth() + 1}`;
    const lastLabel = `${last.getFullYear()}/${last.getMonth() + 1}`;
    setText('monthly-total', `${totalCount} 次`);
    setText(
      'monthly-sub',
      `${firstLabel === lastLabel ? firstLabel : `${firstLabel} – ${lastLabel}`} · 共 ${months.length} 个月`
    );

    const width = Math.max(560, months.length * 40);
    const height = 252;
    const margin = { top: 16, right: 12, bottom: 38, left: 34 };
    const plotWidth = width - margin.left - margin.right;
    const plotHeight = height - margin.top - margin.bottom;
    const yMax = Math.max(4, Math.ceil(peak / 4) * 4);

    const svg = svgEl('svg', {
      viewBox: `0 0 ${width} ${height}`,
      width,
      height,
      role: 'img',
      'aria-label': '月度次数柱状图',
    });
    const defs = svgEl('defs', {}, svg);
    const gradient = svgEl(
      'linearGradient',
      { id: 'monthly-bar-gradient', x1: '0', y1: '0', x2: '0', y2: '1' },
      defs
    );
    svgEl(
      'stop',
      { offset: '0%', 'stop-color': '#fb7185', 'stop-opacity': '0.95' },
      gradient
    );
    svgEl(
      'stop',
      { offset: '100%', 'stop-color': '#fb7185', 'stop-opacity': '0.28' },
      gradient
    );

    for (let tick = 0; tick <= 4; tick += 1) {
      const y = margin.top + plotHeight - (plotHeight * tick) / 4;
      svgEl(
        'line',
        {
          x1: margin.left,
          y1: y,
          x2: width - margin.right,
          y2: y,
          stroke: COLORS.grid,
          'stroke-width': 1,
        },
        svg
      );
      svgText(
        svg,
        margin.left - 8,
        y + 3,
        String((yMax * tick) / 4),
        { 'text-anchor': 'end', fill: COLORS.muted2 }
      );
    }

    const step = plotWidth / months.length;
    const barWidth = Math.min(30, step * 0.58);
    const labelEvery = Math.ceil(months.length / 12);
    const multiYear =
      months[0].getFullYear() !== months[months.length - 1].getFullYear();

    months.forEach((month, index) => {
      const barHeight = (counts[index] / yMax) * plotHeight;
      const x = margin.left + step * index + (step - barWidth) / 2;
      const y = margin.top + plotHeight - barHeight;

      const group = svgEl('g', {}, svg);
      const bar = svgEl(
        'rect',
        {
          x,
          y,
          width: barWidth,
          height: Math.max(counts[index] > 0 ? 2 : 0, barHeight),
          rx: 4,
          fill: 'url(#monthly-bar-gradient)',
        },
        group
      );
      svgTitle(
        bar,
        `${month.getFullYear()}年${month.getMonth() + 1}月 · ${counts[index]} 次`
      );

      if (
        months.length <= 18 ||
        index % labelEvery === 0 ||
        index === months.length - 1
      ) {
        const year = month.getFullYear();
        const monthNumber = month.getMonth() + 1;
        const label = multiYear
          ? `${String(year).slice(2)}/${monthNumber}`
          : index === 0
            ? `${year}/${monthNumber}`
            : `${monthNumber}月`;
        svgText(
          svg,
          x + barWidth / 2,
          height - 16,
          label,
          { 'text-anchor': 'middle', fill: COLORS.muted2 }
        );
      }
    });

    clear(chart);
    chart.appendChild(svg);
  }

  function renderSatisfaction(records) {
    const chart = $('satisfaction-chart');
    if (!records.length) {
      emptyState(chart, '当前筛选下暂无记录');
      setText('satisfaction-avg', '—');
      setText('satisfaction-sub', '1 至 5 分');
      return;
    }

    const counts = [0, 0, 0, 0, 0];
    for (const record of records) {
      counts[record.satisfaction - 1] += 1;
    }
    const max = Math.max(...counts, 1);
    const average =
      records.reduce((sum, record) => sum + record.satisfaction, 0) /
      records.length;

    setText('satisfaction-avg', `均 ${fmt(average)}`);
    setText('satisfaction-sub', `1 至 5 分 · 共 ${records.length} 次`);

    const list = el('div', 'sat-list');
    for (let score = 5; score >= 1; score -= 1) {
      const count = counts[score - 1];
      const row = el('div', 'sat-row');
      row.appendChild(el('span', 'sat-label', `${score} 分`));

      const track = el('div', 'sat-bar');
      const fill = el('i');
      fill.style.width = count ? `${Math.max((count / max) * 100, 5)}%` : '0%';
      fill.style.background = `linear-gradient(90deg, ${SAT_COLORS[score - 1]}55, ${SAT_COLORS[score - 1]})`;
      track.appendChild(fill);
      row.appendChild(track);

      const share = Math.round((count / records.length) * 100);
      row.appendChild(
        el('span', 'sat-count', count ? `${count} · ${share}%` : '0')
      );
      list.appendChild(row);
    }

    clear(chart);
    chart.appendChild(list);
  }

  function renderWeekday(records) {
    const chart = $('weekday-chart');
    if (!records.length) {
      emptyState(chart, '当前筛选下暂无记录');
      setText('weekday-peak', '—');
      setText('weekday-sub', '按记录时间分布');
      return;
    }

    const counts = SLOTS.map(() => WEEKDAYS.map(() => 0));
    let peak = { count: -1, slot: 0, weekday: 0 };
    for (const record of records) {
      const slot = Math.min(3, Math.floor(record.hour / 6));
      counts[slot][record.weekday] += 1;
      if (counts[slot][record.weekday] > peak.count) {
        peak = {
          count: counts[slot][record.weekday],
          slot,
          weekday: record.weekday,
        };
      }
    }

    const max = Math.max(...counts.flat(), 1);
    const grid = el('div', 'wd-grid');
    grid.appendChild(el('div', 'wd-head', ''));
    for (const weekday of WEEKDAYS) {
      grid.appendChild(el('div', 'wd-head', weekday));
    }

    SLOTS.forEach((slot, slotIndex) => {
      const label = el('div', 'wd-slot');
      label.appendChild(el('span', '', slot.label));
      label.appendChild(el('small', '', slot.range));
      grid.appendChild(label);

      WEEKDAYS.forEach((weekdayLabel, weekday) => {
        const count = counts[slotIndex][weekday];
        const cell = el('div', 'wd-cell', count ? String(count) : '');
        cell.title = `${weekdayLabel} ${slot.label}（${slot.range} 点） · ${count} 次`;
        if (count === 0) {
          cell.classList.add('zero');
        } else {
          const alpha = 0.14 + 0.7 * (count / max);
          cell.style.background = `rgba(251, 113, 133, ${alpha.toFixed(3)})`;
        }
        grid.appendChild(cell);
      });
    });

    setText('weekday-peak', `${WEEKDAYS[peak.weekday]} ${SLOTS[peak.slot].label}`);
    setText('weekday-sub', `峰值 ${peak.count} 次 · 按记录时间分布`);

    clear(chart);
    chart.appendChild(grid);
  }

  function renderDuration(records) {
    const chart = $('duration-chart');
    if (!records.length) {
      emptyState(chart, '当前筛选下暂无记录');
      setText('duration-badge', '—');
      setText('duration-sub', '分钟 · 虚线为平均值');
      return;
    }

    const durations = records.map((record) => record.duration);
    const average = total(records) / records.length;
    const peak = Math.max(...durations);
    setText('duration-badge', `均 ${fmt(average)} 分`);
    setText('duration-sub', `${records.length} 次 · 最长 ${peak} 分钟`);

    const width = 720;
    const height = 252;
    const margin = { top: 18, right: 16, bottom: 34, left: 36 };
    const plotWidth = width - margin.left - margin.right;
    const plotHeight = height - margin.top - margin.bottom;
    const yMax = Math.max(10, Math.ceil(peak / 10) * 10);
    const first = records[0].dt;
    const last = records[records.length - 1].dt;
    const span = Math.max(1, last - first);

    const xFor = (record) =>
      records.length === 1
        ? margin.left + plotWidth / 2
        : margin.left + ((record.dt - first) / span) * plotWidth;
    const yFor = (duration) =>
      margin.top + plotHeight - (duration / yMax) * plotHeight;

    const svg = svgEl('svg', {
      viewBox: `0 0 ${width} ${height}`,
      role: 'img',
      'aria-label': '时长趋势折线图',
    });
    const defs = svgEl('defs', {}, svg);
    const gradient = svgEl(
      'linearGradient',
      { id: 'duration-area-gradient', x1: '0', y1: '0', x2: '0', y2: '1' },
      defs
    );
    svgEl(
      'stop',
      { offset: '0%', 'stop-color': '#2dd4bf', 'stop-opacity': '0.26' },
      gradient
    );
    svgEl(
      'stop',
      { offset: '100%', 'stop-color': '#2dd4bf', 'stop-opacity': '0' },
      gradient
    );

    for (let tick = 0; tick <= 2; tick += 1) {
      const value = (yMax * tick) / 2;
      const y = margin.top + plotHeight - (plotHeight * tick) / 2;
      svgEl(
        'line',
        {
          x1: margin.left,
          y1: y,
          x2: width - margin.right,
          y2: y,
          stroke: COLORS.grid,
          'stroke-width': 1,
        },
        svg
      );
      svgText(svg, margin.left - 8, y + 3, String(Math.round(value)), {
        'text-anchor': 'end',
        fill: COLORS.muted2,
      });
    }

    const points = records.map((record) => ({
      x: xFor(record),
      y: yFor(record.duration),
      record,
    }));
    const linePath = points
      .map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`)
      .join(' ');

    if (records.length > 1) {
      const areaPath = `${linePath} L ${points[points.length - 1].x} ${margin.top + plotHeight} L ${points[0].x} ${margin.top + plotHeight} Z`;
      svgEl('path', { d: areaPath, fill: 'url(#duration-area-gradient)' }, svg);
    }

    svgEl(
      'path',
      {
        d: linePath,
        fill: 'none',
        stroke: COLORS.teal,
        'stroke-width': 2,
        'stroke-linejoin': 'round',
        'stroke-linecap': 'round',
      },
      svg
    );

    const averageY = yFor(average);
    svgEl(
      'line',
      {
        x1: margin.left,
        y1: averageY,
        x2: width - margin.right,
        y2: averageY,
        stroke: COLORS.amber,
        'stroke-width': 1,
        'stroke-dasharray': '4 5',
        opacity: 0.7,
      },
      svg
    );

    if (points.length <= 60) {
      for (const point of points) {
        const dot = svgEl(
          'circle',
          {
            cx: point.x,
            cy: point.y,
            r: 2.7,
            fill: '#0d1219',
            stroke: COLORS.teal,
            'stroke-width': 1.6,
          },
          svg
        );
        svgTitle(
          dot,
          `${point.record.date} ${point.record.time} · ${point.record.duration} 分钟`
        );
      }
    }

    const sameYear = records[0].dateObj.getFullYear() === TODAY.getFullYear();
    const formatX = (date) =>
      sameYear ? `${date.getMonth() + 1}/${date.getDate()}` : `${String(date.getFullYear()).slice(2)}/${date.getMonth() + 1}`;
    const mid = records[Math.floor(records.length / 2)];

    const xLabels = [
      {
        x: points[0].x,
        text: formatX(records[0].dateObj),
        anchor: records.length === 1 ? 'middle' : 'start',
      },
    ];
    if (records.length > 2) {
      xLabels.push({
        x: xFor(mid),
        text: formatX(mid.dateObj),
        anchor: 'middle',
      });
    }
    if (records.length > 1) {
      xLabels.push({
        x: points[points.length - 1].x,
        text: formatX(records[records.length - 1].dateObj),
        anchor: 'end',
      });
    }
    for (const label of xLabels) {
      svgText(svg, label.x, height - 12, label.text, {
        'text-anchor': label.anchor,
        fill: COLORS.muted2,
      });
    }

    clear(chart);
    chart.appendChild(svg);
  }

  function renderPartner(records) {
    const chart = $('partner-chart');
    if (!records.length) {
      emptyState(chart, '当前筛选下暂无记录');
      setText('partner-badge', '—');
      setText('partner-sub', '类型构成与伴侣分布');
      return;
    }

    const typeCounts = { partner: 0, solo: 0, other: 0 };
    for (const record of records) typeCounts[record.kind] += 1;

    const layout = el('div', 'partner-layout');

    const left = el('div', 'partner-block');
    left.appendChild(el('h3', '', '类型构成'));

    const donutWrap = el('div', 'donut-wrap');
    const size = 150;
    const radius = 54;
    const donut = svgEl('svg', {
      viewBox: `0 0 ${size} ${size}`,
      width: size,
      height: size,
      role: 'img',
      'aria-label': '类型构成环形图',
    });
    svgEl(
      'circle',
      {
        cx: size / 2,
        cy: size / 2,
        r: radius,
        fill: 'none',
        stroke: 'rgba(255,255,255,.05)',
        'stroke-width': 16,
      },
      donut
    );

    let offset = 0;
    for (const kind of ['partner', 'solo', 'other']) {
      const count = typeCounts[kind];
      if (!count) continue;
      const percent = (count / records.length) * 100;
      const circle = svgEl(
        'circle',
        {
          cx: size / 2,
          cy: size / 2,
          r: radius,
          fill: 'none',
          stroke: KIND_COLOR[kind],
          'stroke-width': 16,
          'stroke-dasharray': `${Math.max(percent - 0.7, 0.7)} 100`,
          'stroke-dashoffset': -offset,
          pathLength: 100,
          transform: `rotate(-90 ${size / 2} ${size / 2})`,
        },
        donut
      );
      svgTitle(
        circle,
        `${KIND_LABEL[kind]} ${count} 次 · ${Math.round(percent)}%`
      );
      offset += percent;
    }

    const center = el('div', 'donut-center');
    const centerValue = el('strong', '', String(records.length));
    center.appendChild(centerValue);
    center.appendChild(el('span', '', '次'));
    donutWrap.appendChild(donut);
    donutWrap.appendChild(center);
    left.appendChild(donutWrap);

    const legend = el('div', 'donut-legend');
    for (const kind of ['partner', 'solo', 'other']) {
      const item = el('span');
      const dot = el('i', 'dot');
      dot.style.background = KIND_COLOR[kind];
      if (!typeCounts[kind]) dot.style.opacity = '0.35';
      item.appendChild(dot);
      item.appendChild(
        document.createTextNode(`${KIND_LABEL[kind]} ${typeCounts[kind]}`)
      );
      legend.appendChild(item);
    }
    left.appendChild(legend);

    const right = el('div', 'partner-block');
    right.appendChild(el('h3', '', '伴侣分布'));

    const partnerCounts = new Map();
    for (const record of records) {
      if (record.kind !== 'partner') continue;
      const name = record.partner || '未指定';
      partnerCounts.set(name, (partnerCounts.get(name) || 0) + 1);
    }
    const partnerTotal = [...partnerCounts.values()].reduce(
      (sum, value) => sum + value,
      0
    );

    if (!partnerCounts.size) {
      right.appendChild(el('div', 'partner-empty', '所选范围内没有伴侣场次'));
    } else {
      const sorted = [...partnerCounts.entries()].sort(
        (leftEntry, rightEntry) =>
          rightEntry[1] - leftEntry[1] ||
          leftEntry[0].localeCompare(rightEntry[0], 'zh-CN')
      );
      const max = sorted[0][1];
      const bars = el('div', 'partner-bars');
      for (const [name, count] of sorted.slice(0, 6)) {
        const row = el('div', 'partner-row');
        const label = el('span', 'partner-name', name);
        label.title = name;
        row.appendChild(label);

        const track = el('div', 'partner-track');
        const fill = el('i');
        fill.style.width = `${(count / max) * 100}%`;
        track.appendChild(fill);
        row.appendChild(track);
        row.appendChild(el('span', 'partner-count', String(count)));
        bars.appendChild(row);
      }
      right.appendChild(bars);
      if (sorted.length > 6) {
        right.appendChild(
          el('p', 'partner-more', `另有 ${sorted.length - 6} 位`)
        );
      }
    }

    layout.appendChild(left);
    layout.appendChild(right);

    setText('partner-badge', `${records.length} 次`);
    setText(
      'partner-sub',
      partnerTotal
        ? `伴侣场次 ${partnerTotal} 次 · ${partnerCounts.size} 位`
        : '所选范围内没有伴侣场次'
    );

    clear(chart);
    chart.appendChild(layout);
  }

  function renderHeatmap(records) {
    const chart = $('heatmap-chart');

    const windowStart = rangeStart() || (records.length ? records[0].dateObj : TODAY);
    const startDay = new Date(windowStart);
    startDay.setHours(0, 0, 0, 0);
    const todayWeekday = (TODAY.getDay() + 6) % 7;
    const thisMonday = new Date(TODAY);
    thisMonday.setDate(thisMonday.getDate() - todayWeekday);
    thisMonday.setHours(0, 0, 0, 0);
    const startWeekday = (startDay.getDay() + 6) % 7;
    const firstMonday = addDays(startDay, -startWeekday);
    const weekCount = Math.max(
      1,
      Math.round((thisMonday - firstMonday) / DAY / 7) + 1
    );
    const startKey = dateKey(startDay);
    const cellSize = weekCount > 30 ? 9 : weekCount > 18 ? 14 : 20;
    const cellGap = weekCount > 30 ? 2 : weekCount > 18 ? 3 : 5;
    chart.style.setProperty('--cal-size', `${cellSize}px`);
    chart.style.setProperty('--cal-gap', `${cellGap}px`);

    const counts = new Map();
    for (const record of records) {
      if (record.date < startKey || record.date > payload.today) continue;
      counts.set(record.date, (counts.get(record.date) || 0) + 1);
    }

    const weekMondays = [];
    for (let week = 0; week < weekCount; week += 1) {
      weekMondays.push(addDays(firstMonday, week * 7));
    }

    const months = [];
    weekMondays.forEach((monday, week) => {
      if (
        week === 0 ||
        monday.getMonth() !== weekMondays[week - 1].getMonth()
      ) {
        months.push({
          start: week,
          label:
            monday.getMonth() === 0
              ? `${monday.getFullYear()}/1`
              : `${monday.getMonth() + 1}月`,
        });
      }
    });

    const monthLabels = el('div', 'calendar-months');
    monthLabels.style.gridTemplateColumns = `repeat(${weekCount}, ${cellSize}px)`;
    months.forEach((month, index) => {
      const end = index + 1 < months.length ? months[index + 1].start : weekCount;
      const label = el('span', '', month.label);
      label.style.gridColumn = `${month.start + 1} / span ${end - month.start}`;
      monthLabels.appendChild(label);
    });

    const weeks = el('div', 'calendar-weeks');
    const max = Math.max(...counts.values(), 1);
    let visibleTotal = 0;
    let activeDays = 0;

    weekMondays.forEach((monday) => {
      for (let day = 0; day < 7; day += 1) {
        const date = addDays(monday, day);
        const key = dateKey(date);
        const cell = el('div', 'cal-day');
        if (key > payload.today) {
          cell.classList.add('future');
          cell.title = `${date.getFullYear()}-${key.slice(5)} · 未到`;
          weeks.appendChild(cell);
          continue;
        }
        if (key < startKey) {
          cell.classList.add('future');
          cell.title = `${date.getFullYear()}-${key.slice(5)} · 范围外`;
          weeks.appendChild(cell);
          continue;
        }
        const count = counts.get(key) || 0;
        visibleTotal += count;
        if (count > 0) {
          activeDays += 1;
          const alpha = 0.18 + 0.74 * (count / max);
          cell.style.background = `rgba(251, 113, 133, ${alpha.toFixed(3)})`;
          cell.style.borderColor = `rgba(251, 113, 133, ${(alpha * 0.6).toFixed(3)})`;
        }
        const weekdayLabel = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'][
          date.getDay()
        ];
        cell.title = `${date.getFullYear()}/${date.getMonth() + 1}/${date.getDate()} ${weekdayLabel} · ${count} 次`;
        weeks.appendChild(cell);
      }
    });

    const main = el('div', 'calendar-main');
    main.appendChild(monthLabels);
    main.appendChild(weeks);

    const days = el('div', 'calendar-days');
    ['周一', '', '周三', '', '周五', '', '周日'].forEach((label) => {
      days.appendChild(el('span', '', label));
    });

    const calendar = el('div', 'calendar');
    calendar.appendChild(days);
    calendar.appendChild(main);

    const legend = el('div', 'cal-legend');
    legend.appendChild(el('span', '', '少'));
    [0.18, 0.34, 0.52, 0.7, 0.92].forEach((alpha) => {
      const swatch = el('i');
      swatch.style.background = `rgba(251, 113, 133, ${alpha})`;
      legend.appendChild(swatch);
    });
    legend.appendChild(el('span', '', '多'));

    const wrapper = el('div', 'heatmap-wrap');
    wrapper.appendChild(calendar);
    wrapper.appendChild(legend);

    const rangeLabel =
      state.range === 'all'
        ? `全部 ${weekCount} 周`
        : `最近 ${weekCount} 周`;
    const visibleStart = startDay > firstMonday ? startDay : firstMonday;
    const sameYear = visibleStart.getFullYear() === TODAY.getFullYear();
    const formatDay = (date) =>
      sameYear
        ? `${date.getMonth() + 1}/${date.getDate()}`
        : `${date.getFullYear()}/${date.getMonth() + 1}/${date.getDate()}`;
    setText('heatmap-title', rangeLabel);
    setText('heatmap-total', `${visibleTotal} 次`);
    setText(
      'heatmap-sub',
      visibleTotal
        ? `活跃 ${activeDays} 天 · ${formatDay(visibleStart)} – ${formatDay(TODAY)}`
        : `${rangeLabel}暂无记录`
    );

    clear(chart);
    chart.appendChild(wrapper);
  }

  function renderActs(records) {
    const chart = $('acts-chart');
    if (!records.length) {
      emptyState(chart, '当前筛选下暂无记录');
      setText('acts-count', '—');
      setText('acts-sub', '出现频次');
      return;
    }

    const counts = new Map();
    let marks = 0;
    for (const record of records) {
      for (const act of record.acts) {
        counts.set(act, (counts.get(act) || 0) + 1);
        marks += 1;
      }
    }

    setText('acts-count', `${counts.size} 个`);
    setText(
      'acts-sub',
      counts.size ? `共 ${marks} 次标记` : '还没有行为标签'
    );

    if (!counts.size) {
      emptyState(chart, '还没有行为标签');
      return;
    }

    const sorted = [...counts.entries()].sort(
      (leftEntry, rightEntry) =>
        rightEntry[1] - leftEntry[1] ||
        leftEntry[0].localeCompare(rightEntry[0], 'zh-CN')
    );
    const max = sorted[0][1];
    const tags = el('div', 'tags');
    const limit = 20;

    for (const [name, count] of sorted.slice(0, limit)) {
      const ratio = count / max;
      const level = ratio >= 0.66 ? ' level-3' : ratio >= 0.33 ? ' level-2' : '';
      const tag = el('span', `tag${level}`);
      tag.appendChild(document.createTextNode(name));
      tag.appendChild(el('em', '', String(count)));
      tag.title = `${name} · ${count} 次`;
      tags.appendChild(tag);
    }
    if (sorted.length > limit) {
      tags.appendChild(el('span', 'tag', `+${sorted.length - limit}`));
    }

    clear(chart);
    chart.appendChild(tags);
  }

  function renderRecent(records) {
    const chart = $('recent-chart');
    if (!records.length) {
      emptyState(chart, '当前筛选下暂无记录');
      setText('recent-count', '—');
      setText('recent-sub', '按时间倒序');
      return;
    }

    const rows = [...records].sort((left, right) => right.dt - left.dt).slice(0, 10);
    setText('recent-count', `最近 ${rows.length} 条`);
    setText('recent-sub', `按时间倒序 · 共 ${records.length} 条`);

    const table = el('table');
    const thead = el('thead');
    const headRow = el('tr');
    for (const title of [
      '日期',
      '类型',
      '伴侣',
      '行为',
      '措施',
      '高潮',
      '满意度',
      '时长',
      '备注',
    ]) {
      headRow.appendChild(el('th', '', title));
    }
    thead.appendChild(headRow);
    table.appendChild(thead);

    const tbody = el('tbody');
    for (const record of rows) {
      const row = el('tr');

      const dateCell = el('td', 'date-cell');
      dateCell.appendChild(el('strong', '', record.date));
      dateCell.appendChild(el('span', '', record.time));
      row.appendChild(dateCell);

      const kindCell = el('td');
      kindCell.appendChild(
        el('span', `pill pill-${record.kind}`, KIND_LABEL[record.kind])
      );
      row.appendChild(kindCell);

      const partnerCell = el('td');
      if (record.partner) {
        const partner = el('span', 'truncate', record.partner);
        partner.title = record.partner;
        partnerCell.appendChild(partner);
      } else {
        partnerCell.appendChild(el('span', 'muted', '—'));
      }
      row.appendChild(partnerCell);

      const actsCell = el('td');
      if (record.acts.length) {
        const acts = el('span', 'truncate', record.acts.join(' · '));
        acts.title = record.acts.join('、');
        actsCell.appendChild(acts);
      } else {
        actsCell.appendChild(el('span', 'muted', '—'));
      }
      row.appendChild(actsCell);

      const protectionCell = el('td');
      protectionCell.appendChild(
        el(
          'span',
          `pill ${record.protection ? 'pill-yes' : 'pill-no'}`,
          record.protection ? '是' : '否'
        )
      );
      row.appendChild(protectionCell);

      row.appendChild(el('td', '', String(record.orgasms)));

      const starsCell = el('td');
      const stars = el('span', 'stars');
      for (let index = 0; index < 5; index += 1) {
        stars.appendChild(
          el('span', index < record.satisfaction ? '' : 'off', '★')
        );
      }
      starsCell.appendChild(stars);
      row.appendChild(starsCell);

      row.appendChild(el('td', '', `${record.duration} 分`));

      const notesCell = el('td');
      if (record.notes) {
        const notes = el('span', 'truncate', record.notes);
        notes.title = record.notes;
        notesCell.appendChild(notes);
      } else {
        notesCell.appendChild(el('span', 'muted', '—'));
      }
      row.appendChild(notesCell);

      tbody.appendChild(row);
    }

    table.appendChild(tbody);
    const wrap = el('div', 'table-wrap');
    wrap.appendChild(table);

    clear(chart);
    chart.appendChild(wrap);
  }

  initFilters();
  renderAll();
})();
