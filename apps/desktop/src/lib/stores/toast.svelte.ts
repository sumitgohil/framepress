/**
 * Lightweight toast store. Powers the in-app completion notification
 * ("Optimization Complete — Saved 285 MB (87%)"). The visual toast component
 * subscribes to this store and renders entries.
 *
 * Phase 1: simple success/error/info variants. No confetti (deferred to
 * Phase 2 per the plan).
 */

import { browser } from "$app/environment";

export type ToastVariant = "success" | "error" | "info";

export type Toast = {
  id: string;
  variant: ToastVariant;
  title: string;
  description?: string;
  /** Optional CTA button — the "Show in Finder" affordance, for instance. */
  action?: { label: string; on_click: () => void };
  /** Auto-dismiss timeout in ms. `0` = sticky. */
  timeout_ms: number;
};

let toasts = $state<Toast[]>([]);

function next_id(): string {
  return Math.random().toString(36).slice(2, 11);
}

export const toast = {
  get items() {
    return toasts;
  },
  show(t: Omit<Toast, "id" | "timeout_ms"> & { timeout_ms?: number }) {
    const id = next_id();
    const entry: Toast = {
      id,
      timeout_ms: t.timeout_ms ?? 5000,
      variant: t.variant,
      title: t.title,
      description: t.description,
      action: t.action,
    };
    toasts = [...toasts, entry];
    if (browser && entry.timeout_ms > 0) {
      setTimeout(() => this.dismiss(id), entry.timeout_ms);
    }
    return id;
  },
  success(title: string, description?: string) {
    return this.show({ variant: "success", title, description });
  },
  error(title: string, description?: string) {
    return this.show({
      variant: "error",
      title,
      description,
      timeout_ms: 8000,
    });
  },
  info(title: string, description?: string) {
    return this.show({ variant: "info", title, description });
  },
  dismiss(id: string) {
    toasts = toasts.filter((t) => t.id !== id);
  },
  clear() {
    toasts = [];
  },
};
