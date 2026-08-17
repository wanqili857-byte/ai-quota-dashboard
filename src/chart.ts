// Canvas 折线图: 近 N 天 token 用量。纯紫色折线, 无填充, 拐点圆点, 最高点标注, 坐标轴刻度。

interface TrendPoint {
  date: string;
  requests: number;
  total_tokens: number;
  cost_usd: number;
}

export function drawTrend(canvas: HTMLCanvasElement, points: TrendPoint[]) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const W = canvas.width;
  const H = canvas.height;
  ctx.clearRect(0, 0, W, H);

  if (points.length < 2) {
    ctx.fillStyle = "rgba(255,255,255,0.35)";
    ctx.font = "11px system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("数据不足", W / 2, H / 2);
    return;
  }

  const pad = { l: 42, r: 14, t: 16, b: 18 };
  const iw = W - pad.l - pad.r;
  const ih = H - pad.t - pad.b;

  const max = Math.max(...points.map((p) => p.total_tokens), 1);
  const x = (i: number) => pad.l + (i * iw) / (points.length - 1);
  const y = (v: number) => pad.t + ih - (v / max) * ih;

  // Y 轴刻度 (左侧, 4 个)
  ctx.font = "9px system-ui, sans-serif";
  ctx.textAlign = "right";
  ctx.fillStyle = "rgba(255,255,255,0.4)";
  ctx.strokeStyle = "rgba(255,255,255,0.07)";
  ctx.lineWidth = 1;
  for (let g = 0; g <= 3; g++) {
    const gy = pad.t + (ih * g) / 3;
    ctx.beginPath();
    ctx.moveTo(pad.l, gy);
    ctx.lineTo(pad.l + iw, gy);
    ctx.stroke();
    const val = max - (max * g) / 3;
    ctx.fillText(fmtShort(val), pad.l - 4, gy + 3);
  }
  // Y 轴竖线
  ctx.beginPath();
  ctx.moveTo(pad.l, pad.t);
  ctx.lineTo(pad.l, pad.t + ih);
  ctx.stroke();

  const xs = points.map((_, i) => x(i));
  const ys = points.map((p) => y(p.total_tokens));

  // 纯紫色折线 (无填充)
  ctx.beginPath();
  xs.forEach((xi, i) => (i === 0 ? ctx.moveTo(xi, ys[i]) : ctx.lineTo(xi, ys[i])));
  ctx.strokeStyle = "#a78bfa";
  ctx.lineWidth = 2;
  ctx.lineJoin = "round";
  ctx.lineCap = "round";
  ctx.stroke();

  // 拐点圆点
  xs.forEach((xi, i) => {
    ctx.beginPath();
    ctx.arc(xi, ys[i], 2, 0, Math.PI * 2);
    ctx.fillStyle = "#a78bfa";
    ctx.fill();
  });

  // 最高点: 大圆点 + 数值标注 (动态定位, 不超画布)
  let maxIdx = 0;
  points.forEach((p, i) => {
    if (p.total_tokens > points[maxIdx].total_tokens) maxIdx = i;
  });
  ctx.beginPath();
  ctx.arc(xs[maxIdx], ys[maxIdx], 4, 0, Math.PI * 2);
  ctx.fillStyle = "#fff";
  ctx.fill();
  ctx.strokeStyle = "#a78bfa";
  ctx.lineWidth = 2;
  ctx.stroke();
  const label = fmtShort(points[maxIdx].total_tokens);
  ctx.font = "bold 9px system-ui, sans-serif";
  const labelW = ctx.measureText(label).width;
  const lx = Math.min(Math.max(xs[maxIdx], pad.l + labelW / 2 + 6), pad.l + iw - labelW / 2 - 6);
  const ly = Math.max(ys[maxIdx] - 12, 14);
  ctx.textAlign = "center";
  ctx.fillStyle = "#e0d4ff";
  ctx.fillText(label, lx, ly);

  // X 轴日期标签 (全部显示, 首末向内偏移防截断)
  ctx.fillStyle = "rgba(255,255,255,0.4)";
  ctx.font = "8px system-ui, sans-serif";
  ctx.textAlign = "center";
  points.forEach((p, i) => {
    const [, m, d] = p.date.split("-");
    const label = `${m}-${d}`;
    const xi = i === 0 ? x(i) + 7 : i === points.length - 1 ? x(i) - 7 : x(i);
    ctx.fillText(label, xi, H - 5);
  });
  // X 轴横线 (浅灰, 非紫色)
  ctx.beginPath();
  ctx.moveTo(pad.l, pad.t + ih);
  ctx.lineTo(pad.l + iw, pad.t + ih);
  ctx.strokeStyle = "rgba(255,255,255,0.15)";
  ctx.lineWidth = 1;
  ctx.stroke();
}

function fmtShort(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(1) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(0) + "K";
  return String(Math.round(n));
}
