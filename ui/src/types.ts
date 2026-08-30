// types from backend - keep in sync with core

export interface DeviceListing {
  udid: string;
  connection: string;
}

export interface DeviceInfo {
  udid: string;
  name: string | null;
  product_type: string | null;
  ios_version: string | null;
  build: string | null;
}

export interface Track {
  pid: number;
  title: string | null;
  artist: string | null;
  album: string | null;
  year: number | null;
  duration_ms: number | null;
  bit_rate: number | null;
  sample_rate: number | null;
  base_path: string;
  location: string;
  file_size: number | null;
  track_number: number | null;
  disc_number: number | null;
}

export interface AddReport {
  added: number;
  skipped: number;
  pids: number[];
  folder: string;
  messages: string[];
}

export interface RemoveReport {
  removed: number;
  messages: string[];
}

export interface ExportReport {
  exported: number;
  out_dir: string;
  messages: string[];
}