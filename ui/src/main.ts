// ui entry - vanilla ts, no framework
// renders device bar, toolbar, table and calls backend commands

import { api } from "./api";
import type { DeviceInfo, Track } from "./types";
import "./style.css";

// ---- element refs ----------------------------------------------------------

const root = document.querySelector<HTMLDivElement>("#app")!;

// ---- UI shell --------------------------------------------------------------

function shell(): void {
  root.innerHTML = `
    <header class="titlebar">
      <h1>musicport</h1>
      <div class="device" id="deviceSlot"></div>
    </header>
    <main class="content">
      <section class="toolbar" id="toolbarSlot"></section>
      <section class="tracks">
        <table class="track-table">
          <thead>
            <tr>
              <th class="col-check"></th>
              <th class="col-title">Title</th>
              <th class="col-artist">Artist</th>
              <th class="col-album">Album</th>
              <th class="col-num col-year">Year</th>
              <th class="col-num col-length">Length</th>
              <th class="col-num col-size">Size</th>
            </tr>
          </thead>
          <tbody id="trackBody"></tbody>
        </table>
        <p class="empty" id="emptyMsg" hidden>No tracks - add a library to get started.</p>
      </section>
    </main>
  `;
}

// ---- state -----------------------------------------------------------------

interface AppState {
  device: DeviceInfo | null;
  tracks: Track[];
  selected: Set<number>;
  query: string;
}

const state: AppState = {
  device: null,
  tracks: [],
  selected: new Set(),
  query: "",
};

// ---- renderers -------------------------------------------------------------

function renderDevice(): void {
  const slot = document.querySelector<HTMLElement>("#deviceSlot")!;
  if (!state.device) {
    slot.innerHTML = /* html */ `
      <span class="dev-idle" title="No iPhone connected">No device</span>
      <button id="refreshBtn" type="button">Scan for devices</button>
    `;
    return;
  }
  const d = state.device;
  slot.innerHTML = /* html */ `
    <span class="dev-dot" title="${escapeAttr(d.udid)}"></span>
    <span class="dev-name" title="${escapeAttr(d.name ?? "iPhone")}">${escapeHtml(d.name ?? "iPhone")}</span>
    <span class="dev-meta">iOS ${escapeHtml(d.ios_version ?? "?")}</span>
  `;
}

function renderToolbar(): void {
  const slot = document.querySelector<HTMLElement>("#toolbarSlot")!;
  const disabled = state.device ? "" : "disabled";
  slot.innerHTML = /* html */ `
    <button id="addBtn" type="button" ${disabled}>Add files…</button>
    <button id="exportBtn" type="button" ${disabled}>Export…</button>
    <button id="removeBtn" type="button" ${disabled}>Remove selected</button>
    <input id="searchBox" type="search" placeholder="Search" value="${escapeAttr(state.query)}" ${disabled} />
    <span class="status" id="statusMsg"></span>
  `;
}

function renderTracks(): void {
  const body = document.querySelector<HTMLElement>("#trackBody")!;
  const empty = document.querySelector<HTMLElement>("#emptyMsg")!;
  body.innerHTML = "";

  const q = state.query.trim().toLowerCase();
  const rows = state.tracks.filter(
    (t) =>
      !q ||
      [t.title, t.artist, t.album].some((v) =>
        (v ?? "").toLowerCase().includes(q)
      ),
  );

  empty.hidden = rows.length > 0;

  for (const t of rows) {
    const tr = document.createElement("tr");
    tr.innerHTML = `
      <td class="col-check">
        <input type="checkbox" class="tick" data-pid="${t.pid}" ${state.selected.has(t.pid) ? "checked" : ""} />
      </td>
      <td class="col-title" title="${escapeAttr(t.title ?? "Untitled")}">${escapeHtml(t.title ?? "Untitled")}</td>
      <td class="col-artist" title="${escapeAttr(t.artist ?? "")}">${escapeHtml(t.artist ?? "")}</td>
      <td class="col-album" title="${escapeAttr(t.album ?? "")}">${escapeHtml(t.album ?? "")}</td>
      <td class="col-num col-year">${t.year ? String(t.year) : ""}</td>
      <td class="col-num col-length">${formatMs(t.duration_ms)}</td>
      <td class="col-num col-size">${formatBytes(t.file_size)}</td>
    `;
    body.appendChild(tr);
  }
}

function render(): void {
  renderDevice();
  renderToolbar();
  renderTracks();
}

// ---- actions ----------------------------------------------------------------

let busy = false;
function withBusy(fn: () => Promise<void>): void {
  if (busy) return;
  setStatus("Working…");
  busy = true;
  fn()
    .catch((err: unknown) => {
      setStatus(`Something went wrong: ${describe(err)}`);
    })
    .finally(() => {
      busy = false;
    });
}

function setStatus(msg: string): void {
  const s = document.querySelector<HTMLElement>("#statusMsg");
  if (s) s.textContent = msg;
}

function describe(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

// ---- event wiring -----------------------------------------------------------

function bind(): void {
  document.addEventListener("change", (e) => {
    const cb = (e.target as HTMLElement).closest<HTMLInputElement>("#searchBox");
    if (cb) {
      state.query = cb.value;
      renderTracks();
    }
  });

  document.addEventListener("click", async (e) => {
    const btn = (e.target as HTMLElement).closest("button");
    if (!btn) return;

    switch (btn.id) {
      case "refreshBtn":
        setStatus("Scanning for devices…");
        await scan();
        break;
      case "addBtn":
        withBusy(pickAndAdd);
        break;
      case "exportBtn":
        withBusy(pickAndExport);
        break;
      case "removeBtn":
        withBusy(removeSelected);
        break;
    }
  });

  document.addEventListener("change", (e) => {
    const cb = (e.target as HTMLElement).closest<HTMLInputElement>(".tick");
    if (cb) {
      const pid = Number(cb.dataset.pid);
      if (cb.checked) state.selected.add(pid);
      else state.selected.delete(pid);
    }
  });
}

// ---- device connection -------------------------------------------------------

let scanning = false;
let retryTimer: number | undefined;

// clear retry timer if any
function clearRetry(): void {
  if (retryTimer !== undefined) {
    window.clearTimeout(retryTimer);
    retryTimer = undefined;
  }
}

// retry scan soon if no device - picks up plug/unlock/trust without needing a click
function scheduleRetry(): void {
  if (state.device || retryTimer !== undefined) return;
  retryTimer = window.setTimeout(() => {
    retryTimer = undefined;
    void scan();
  }, 3000);
}

async function scan(): Promise<void> {
  if (scanning) return;
  scanning = true;
  try {
    renderDevice();
    setStatus("Scanning for devices…");
    const found = await api.listDevices();
    if (found.length === 0) {
      setStatus("No devices found - plug in and unlock your iPhone.");
      scheduleRetry();
      return;
    }
    const info = await api.connect(found[0].udid);
    state.device = info;
    clearRetry();
    setStatus(`Connected to ${info.name ?? info.udid}`);
    await load();
  } catch (err) {
    renderDevice();
    setStatus(`Connect failed: ${describe(err)}`);
    scheduleRetry();
  } finally {
    scanning = false;
  }
}

async function load(): Promise<void> {
  if (!state.device) return;
  try {
    state.tracks = await api.listTracks();
    state.selected.clear();
    render();
  } catch (err) {
    // phone probably unplugged/locked - clear state, auto retry will handle it
    state.device = null;
    state.tracks = [];
    render();
    setStatus(`Connection lost: ${describe(err)}`);
    scheduleRetry();
  }
}

async function pickAndAdd(): Promise<void> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const files = await open({ multiple: true, directory: false });
  const list = Array.isArray(files) ? files : files ? [files] : [];
  const paths = list.filter((p): p is string => typeof p === "string");
  if (paths.length === 0) {
    setStatus("Add cancelled");
    return;
  }
  const report = await api.addFiles(paths);
  const first = report.messages?.[0] ?? "";
  setStatus(
    `Added ${report.added}, skipped ${report.skipped}${first ? " - " + first : ""}`,
  );
  await load();
}

async function pickAndExport(): Promise<void> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const dir = await open({ directory: true });
  if (!dir || typeof dir !== "string") {
    setStatus("Export cancelled");
    return;
  }
  const report = await api.exportTracks(dir, state.query || undefined);
  setStatus(`Exported ${report.exported} track(s) to ${report.out_dir}`);
}

async function removeSelected(): Promise<void> {
  if (state.selected.size === 0) {
    setStatus("Select tracks to remove");
    return;
  }
  const report = await api.removeTracks([...state.selected]);
  setStatus(`Removed ${report.removed} track(s)`);
  await load();
}

// ---- tiny helpers ------------------------------------------------------------

function formatMs(ms: number | null): string {
  if (ms == null) return "";
  const total = Math.round(ms / 1000);
  const m = Math.floor(total / 60);
  const s = String(total % 60).padStart(2, "0");
  return `${m}:${s}`;
}

function formatBytes(b: number | null): string {
  if (b == null) return "";
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / (1024 * 1024)).toFixed(1)} MB`;
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!,
  );
}

function escapeAttr(s: string): string {
  return escapeHtml(s).replace(/`/g, "&#96;");
}

// ---- boot ------------------------------------------------------------------

shell();
bind();
render();
void scan();