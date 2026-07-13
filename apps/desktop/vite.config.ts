import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';

// Tauri expects a fixed port and disables HMR over network by default.
// See https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(({ mode }) => ({
  plugins: [tailwindcss(), sveltekit()],
  // Component tests render in jsdom and need Svelte's browser build. Keeping
  // this test-only preserves SvelteKit's server build during dev/SSR.
  resolve: mode === 'test' ? { conditions: ['browser'] } : undefined,
  // Vite options tailored for Tauri development.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tell vite to ignore watching `src-tauri` so the dev loop is owned by
      // cargo-tauri, not vite.
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.{test,spec}.{js,ts}'],
    setupFiles: ['./src/test-setup.ts'],
  },
}));
