import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { drawTrend } from "./chart";

// 元素辅助: document.getElementById 的类型安全包装 (非空断言)
const el = (id: string): HTMLElement => document.getElementById(id)!;

// 运行错误捕获: 记 console, 异常时临时显示到余额位
window.addEventListener("error", (e) => {
  const msg = e.message || String(e.error || "");
  console.error("[ui]", msg);
  const det = document.getElementById("ds-balance");
  if (det && det.textContent === "--") det.textContent = "ERR";
});
window.addEventListener("unhandledrejection", (e) => {
  console.error("[ui-reject]", String(e.reason));
});

// ---------- 类型 (与 Rust 命令返回值对应) ----------
interface Balance {
  currency: string;
  total: number;
  granted: number;
  topped_up: number;
}
interface QuotaWindow {
  percent: number;
  resets_at: string;
  status: string;
}
interface OpenCodeUsage {
  rolling: QuotaWindow;
  weekly: QuotaWindow;
  monthly: QuotaWindow;
}
interface WindowStats {
  requests: number;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_creation_tokens: number;
  cost_usd: number;
  cache_hit_rate: number;
}
interface UsageStats {
  today: WindowStats;
  week: WindowStats;
  month: WindowStats;
}
interface TrendPoint {
  date: string;
  requests: number;
  total_tokens: number;
  cost_usd: number;
}
interface ProviderStat {
  requests: number;
  input_tokens: number;
}
interface ProviderStats {
  deepseek: ProviderStat;
  opencode: ProviderStat;
}

// ---------- 格式化 ----------
function fmtTokens(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(2) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(0) + "K";
  return String(n);
}

function fmtCost(n: number): string {
  return "$" + (n >= 100 ? n.toFixed(0) : n.toFixed(2));
}

function fmtPercent(n: number): string {
  return n.toFixed(0) + "%";
}

function fmtReset(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "";
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const mi = String(d.getMinutes()).padStart(2, "0");
  return `重置 ${mm}-${dd} ${hh}:${mi}`;
}

// ---------- 渲染 ----------
function renderBalance(b: Balance) {
  el("ds-balance").textContent = `¥${b.total.toFixed(2)}`;
}

function renderOpenCode(u: OpenCodeUsage) {
  el("oc-rolling").textContent = fmtPercent(u.rolling.percent);
  el("oc-rolling-reset").textContent = fmtReset(u.rolling.resets_at);
  el("oc-weekly").textContent = fmtPercent(u.weekly.percent);
  el("oc-weekly-reset").textContent = fmtReset(u.weekly.resets_at);
  el("oc-monthly").textContent = fmtPercent(u.monthly.percent);
  el("oc-monthly-reset").textContent = fmtReset(u.monthly.resets_at);
  colorWindows();
}

function colorWindows() {
  // percent = 已用量: 用得越多越红 (绿→黄→红动态) + 进度条宽度
  const winEls = document.querySelectorAll<HTMLElement>(".oc-window");
  winEls.forEach((w) => {
    w.classList.remove("ok", "warn", "danger");
    const pct = parseInt(w.querySelector(".oc-percent")?.textContent || "0", 10);
    w.classList.add(pct >= 80 ? "danger" : pct >= 50 ? "warn" : "ok");
    const fill = w.querySelector<HTMLElement>(".oc-fill");
    if (fill) fill.style.width = Math.min(pct, 100) + "%";
  });
}

function renderStats(s: UsageStats) {
  const total = (w: WindowStats) =>
    w.input_tokens + w.output_tokens + w.cache_read_tokens + w.cache_creation_tokens;
  const sub = (w: WindowStats) => `${w.requests}次 · ${fmtCost(w.cost_usd)}`;
  el("tk-today").textContent = fmtTokens(total(s.today));
  el("tk-today-sub").textContent = sub(s.today);
  el("tk-week").textContent = fmtTokens(total(s.week));
  el("tk-week-sub").textContent = sub(s.week);
  el("tk-month").textContent = fmtTokens(total(s.month));
  el("tk-month-sub").textContent = sub(s.month);
  el("hit-rate").textContent = fmtPercent(s.today.cache_hit_rate);
}

function renderProviders(p: ProviderStats) {
  const fmt = (st: ProviderStat) => `今日 ${st.requests}次 · 输入 ${fmtTokens(st.input_tokens)}`;
  el("pr-opencode").textContent = fmt(p.opencode);
  el("pr-deepseek").textContent = fmt(p.deepseek);
}

function renderTrend(points: TrendPoint[]) {
  drawTrend(el("trend") as HTMLCanvasElement, points);
}

// ---------- 数据拉取 ----------
async function refresh() {
  const [bal, oc, stats, trend, providers] = await Promise.allSettled([
    invoke<Balance>("get_balance"),
    invoke<OpenCodeUsage>("get_opencode_usage"),
    invoke<UsageStats>("get_usage_stats"),
    invoke<TrendPoint[]>("get_trend", { days: 5 }),
    invoke<ProviderStats>("get_provider_stats"),
  ]);

  if (bal.status === "fulfilled") {
    renderBalance(bal.value);
  } else {
    el("ds-balance").textContent = "不可用";
  }

  if (oc.status === "fulfilled") {
    renderOpenCode(oc.value);
  } else {
    ["oc-rolling", "oc-weekly", "oc-monthly"].forEach((id) => el(id).textContent = "err");
  }

  if (stats.status === "fulfilled") {
    renderStats(stats.value);
  } else {
    ["tk-today", "tk-week", "tk-month", "hit-rate"].forEach((id) => el(id).textContent = "err");
  }

  if (trend.status === "fulfilled") {
    renderTrend(trend.value);
  }

  if (providers.status === "fulfilled") {
    renderProviders(providers.value);
  } else {
    ["pr-opencode", "pr-deepseek"].forEach((id) => el(id).textContent = "err");
  }
}

// ---------- 折叠态 / 吸顶 ----------
async function toggleCollapse() {
  const collapsed = await invoke<boolean>("toggle_collapse");
  document.body.classList.toggle("collapsed", collapsed);
}

async function updatePinState() {
  try {
    const on = await getCurrentWindow().isAlwaysOnTop();
    el("btn-pin").classList.toggle("active", on);
  } catch { /* 忽略 */ }
}

// ---------- 初始化 (readyState 兜底: DOM 未就绪则等 DOMContentLoaded) ----------
function init() {
  const requiredIds = [
    "btn-collapse", "btn-pin", "btn-quit", "btn-ds-topup",
    "ds-balance", "oc-rolling", "oc-weekly", "oc-monthly",
    "oc-rolling-reset", "oc-weekly-reset", "oc-monthly-reset",
    "tk-today", "tk-week", "tk-month", "tk-today-sub", "tk-week-sub", "tk-month-sub",
    "hit-rate", "pr-opencode", "pr-deepseek", "trend",
  ];
  const missingIds = requiredIds.filter((id) => !document.getElementById(id));
  if (missingIds.length) {
    console.error("[init] missing ids:", missingIds.join(","));
    return;
  }
  el("btn-collapse").addEventListener("click", toggleCollapse);
  el("btn-pin").addEventListener("click", async () => {
    await invoke("toggle_pin");
    updatePinState();
  });
  el("btn-quit").addEventListener("click", () => getCurrentWindow().close());
  el("btn-ds-topup").addEventListener("click", () => invoke("open_topup", { platform: "deepseek" }));

  // 每个模块右上角折叠箭头: 折叠/展开该卡片 (document 委托)
  document.addEventListener("click", (e) => {
    const btn = (e.target as HTMLElement).closest<HTMLElement>(".collapse-btn");
    if (!btn) return;
    const card = btn.closest(".card");
    if (!card) return;
    const collapsed = card.classList.toggle("collapsed-card");
    btn.textContent = collapsed ? "▸" : "▾";
  });

  refresh();
  setInterval(refresh, 30_000); // 30s 刷新
  updatePinState();
}
if (document.readyState === "loading") {
  window.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
