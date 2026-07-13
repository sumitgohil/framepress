/**
 * Settings store. Persists user preferences to the OS app-config dir via the
 * Rust side (Branch 6 wires the actual file I/O). The Svelte side holds an
 * in-memory mirror and exposes the same shape regardless of whether settings
 * have been loaded from disk yet.
 */

import { browser } from '$app/environment';

import type { CompressionPreset } from '$lib/ipc/types';
import { PRESET_KEYS } from '$lib/ipc/types';

const STORAGE_KEY = 'tinydrop:settings';

export type SettingsState = {
  default_preset: CompressionPreset;
  output_behavior: 'sidecar' | 'in-place';
};

const DEFAULTS: SettingsState = {
  default_preset: 'website',
  output_behavior: 'sidecar',
};

function read_initial(): SettingsState {
  if (!browser) return { ...DEFAULTS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<SettingsState>;
    return { ...DEFAULTS, ...parsed };
  } catch {
    return { ...DEFAULTS };
  }
}

function write(state: SettingsState) {
  if (!browser) return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // ignore quota errors
  }
}

function create_settings_store() {
  let state = $state<SettingsState>(read_initial());

  return {
    get value() {
      return state;
    },
    set(patch: Partial<SettingsState>) {
      state = { ...state, ...patch };
      write(state);
    },
    reset() {
      state = { ...DEFAULTS };
      write(state);
    },
  };
}

export const settings = create_settings_store();

export const PRESET_LABELS: Record<CompressionPreset, string> = {
  lossless: 'Lossless',
  maximum_compression: 'Maximum Compression',
  developer_assets: 'Developer Assets',
  website: 'Website',
  email: 'Email',
  social_media: 'Social Media',
};

export const PRESET_DESCRIPTIONS: Record<CompressionPreset, string> = {
  lossless: 'No quality loss. Pixel-perfect output.',
  maximum_compression: 'Smallest file possible. Slow.',
  developer_assets: 'Icons, screenshots, UI assets. Lossless.',
  website: 'Tuned for fast-loading web pages.',
  email: 'Stays under common attachment size limits.',
  social_media: 'Punchy visuals for social platforms.',
};

export const DEFAULT_PRESET: CompressionPreset =
  PRESET_KEYS.find((k) => k === DEFAULTS.default_preset) ?? 'website';
