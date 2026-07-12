import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    // Tauri serves the app from a fixed origin; static adapter keeps the
    // output a pure SPA bundle with no Node server requirement.
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: false,
      strict: true,
    }),
  },
  compilerOptions: {
    // Svelte 5 — runes mode is opt-in per component, no global flag needed.
  },
};

export default config;