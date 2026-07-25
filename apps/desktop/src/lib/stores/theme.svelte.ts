/**
 * Theme store. Three modes: 'light' | 'dark' | 'system'.
 *
 * - 'system' follows the OS via `prefers-color-scheme`
 * - the others force light or dark
 *
 * The resolved effective theme (`light` | `dark`) drives the `dark` class on
 * the `<html>` element, which Tailwind v4 reads via `@custom-variant dark`.
 */

import { browser } from "$app/environment";

export type ThemeMode = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "framepress:theme";

function read_initial_mode(): ThemeMode {
  if (!browser) return "system";
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") {
      return stored;
    }
  } catch {
    // ignore storage errors (private browsing etc.)
  }
  return "system";
}

function resolve(mode: ThemeMode): ResolvedTheme {
  if (mode === "light" || mode === "dark") return mode;
  if (!browser) return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function apply(resolved: ResolvedTheme) {
  if (!browser) return;
  const root = document.documentElement;
  root.classList.toggle("dark", resolved === "dark");
  root.style.colorScheme = resolved;
}

function create_theme_store() {
  let mode = $state<ThemeMode>(read_initial_mode());
  let resolved = $state<ResolvedTheme>(resolve(mode));

  // Stores are created at module initialization, outside a component's
  // lifecycle. A `$effect` here throws `effect_orphan` in Svelte 5 and turns
  // route navigation into the error page. A normal media-query listener keeps
  // the system mode responsive without depending on component ownership.
  if (browser) {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    mql.addEventListener("change", () => {
      if (mode === "system") {
        resolved = resolve("system");
        apply(resolved);
      }
    });
    apply(resolved);
  }

  return {
    get mode() {
      return mode;
    },
    get resolved() {
      return resolved;
    },
    set(next: ThemeMode) {
      mode = next;
      resolved = resolve(next);
      try {
        localStorage.setItem(STORAGE_KEY, next);
      } catch {
        // ignore
      }
      apply(resolved);
    },
    cycle() {
      const order: ThemeMode[] = ["light", "dark", "system"];
      const next = order[(order.indexOf(mode) + 1) % order.length];
      this.set(next);
    },
  };
}

export const theme = create_theme_store();
