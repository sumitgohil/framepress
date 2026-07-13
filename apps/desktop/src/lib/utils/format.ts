/**
 * Display formatters used by stat cards, queue rows, and history rows.
 */

/** Format a byte count as a human-readable string. */
export function format_bytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '—';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit_index = 0;

  while (value >= 1024 && unit_index < units.length - 1) {
    value /= 1024;
    unit_index += 1;
  }

  return `${value.toFixed(value >= 100 ? 0 : value >= 10 ? 1 : 2)} ${units[unit_index]}`;
}

/** Format a savings ratio (0..1) as a percentage. */
export function format_percent(value: number): string {
  if (!Number.isFinite(value)) return '—';
  return `${(value * 100).toFixed(0)}%`;
}

/** Format an absolute percentage value (e.g. 87) as a percentage string. */
export function format_pct(value: number): string {
  if (!Number.isFinite(value)) return '—';
  return `${value.toFixed(0)}%`;
}

/** Format a duration in ms as a compact human-readable string. */
export function format_duration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return '—';
  if (ms < 1000) return `${ms} ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = Math.round((ms % 60_000) / 1000);
  return `${minutes}m ${seconds}s`;
}

/** Format a Date as a relative "x minutes ago" string. */
export function format_relative(date: Date | number): string {
  const ts = typeof date === 'number' ? date : date.getTime();
  const diff = Date.now() - ts;
  const seconds = Math.round(diff / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days}d ago`;
  const d = typeof date === 'number' ? new Date(date) : date;
  return d.toLocaleDateString();
}
