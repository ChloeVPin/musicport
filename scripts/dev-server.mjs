#!/usr/bin/env node
// starts vite for tauri dev - needs port 1420 exactly
// kills leftover vite from dead sessions, otherwise port in use errors
// only reclaims our own vite, leaves other stuff alone

import { execFileSync, spawn } from "node:child_process";
import net from "node:net";
import { fileURLToPath } from "node:url";

const PORT = 1420;
const ROOT = fileURLToPath(new URL("..", import.meta.url));

// pids listening on port (via ss or lsof)
function pidsOnPort(port) {
  const candidates = [
    { cmd: "ss", args: ["-ltnp"] },
    { cmd: "lsof", args: ["-ti", `:${port}`] },
  ];
  for (const { cmd, args } of candidates) {
    try {
      const out = execFileSync(cmd, args, { encoding: "utf8" });
      const pids = new Set();
      for (const line of out.split("\n")) {
        for (const m of line.matchAll(/pid=(\d+)/g)) pids.add(Number(m[1]));
        const trimmed = line.trim();
        if (/^\d+$/.test(trimmed)) pids.add(Number(trimmed));
      }
      if (pids.size > 0) return [...pids];
    } catch {
      // tool missing or nothing to report - try the next one
    }
  }
  return [];
}

// check if pid looks like vite
function looksLikeVite(pid) {
  try {
    const cmd = execFileSync("ps", ["-p", String(pid), "-o", "cmd="], {
      encoding: "utf8",
    });
    return cmd.includes("vite");
  } catch {
    return false;
  }
}

// check if port is listening locally
function portIsListening(port) {
  return new Promise((resolve) => {
    const sock = net.connect({ port, host: "127.0.0.1" });
    sock.once("connect", () => {
      sock.destroy();
      resolve(true);
    });
    sock.once("error", () => resolve(false));
  });
}

// ---- reclaim a leftover musicport Vite server -------------------------------

if (await portIsListening(PORT)) {
  const pids = pidsOnPort(PORT);
  const stale = [];
  for (const pid of pids) {
    if (await looksLikeVite(pid)) stale.push(pid);
  }

  if (stale.length > 0) {
    console.log(
      `[dev-server] port ${PORT} is held by a leftover Vite server ` +
        `(pid ${stale.join(", ")}) from a previous session - stopping it.`,
    );
    for (const pid of stale) {
      try {
        process.kill(pid, "SIGTERM");
      } catch {
        // already gone
      }
    }
    await new Promise((r) => setTimeout(r, 800));
  } else if (pids.length > 0) {
    console.error(
      `[dev-server] port ${PORT} is already in use by a process that is not ` +
        `musicport's dev server (pid ${pids.join(", ") || "unknown"}).`,
    );
    console.error("Stop that process and run `npm run tauri:dev` again.");
    process.exit(1);
  } else {
    // Occupied but we couldn't identify the owner - let vite report it.
    console.error(
      `[dev-server] port ${PORT} is in use but the owning process could not ` +
        "be identified (can't read `ss`/`lsof`).",
    );
    process.exit(1);
  }
}

// ---- start vite --------------------------------------------------------------

console.log("[dev-server] starting Vite on http://localhost:1420");
const vite = spawn("npm", ["run", "dev", "--workspace", "ui"], {
  cwd: ROOT,
  stdio: "inherit",
});

// forward sigint/sigterm so ctrl+c cleans up vite
for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    if (!vite.killed) vite.kill(signal);
  });
}

if (vite.stdin) vite.stdin.end();
vite.on("exit", (code) => process.exit(code ?? 0));