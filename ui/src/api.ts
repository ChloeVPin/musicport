// typed wrappers for invoke - keeps api surface in one place

import { invoke } from "@tauri-apps/api/core";
import type {
  AddReport,
  DeviceInfo,
  DeviceListing,
  ExportReport,
  RemoveReport,
  Track,
} from "./types";

function cmd<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export const api = {
  connect: (udid?: string) =>
    cmd<DeviceInfo>("connect", udid == null ? {} : { udid }),

  listDevices: () => cmd<DeviceListing[]>("list_devices"),

  listTracks: (query?: string) =>
    cmd<Track[]>("list_tracks", query == null ? {} : { query }),

  addFiles: (files: string[], force = false) =>
    cmd<AddReport>("add_files", { files, force }),

  removeTracks: (pids: number[]) =>
    cmd<RemoveReport>("remove_tracks", { pids }),

  exportTracks: (outDir: string, query?: string) =>
    cmd<ExportReport>("export_tracks", {
      outDir,
      query: query ?? null,
    }),
};