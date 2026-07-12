/**
 * Vitest setup file. Loaded before every test file.
 *
 * Importing `@testing-library/svelte` here makes its matchers and helpers
 * globally available, and forces the jsdom environment to be ready before
 * components mount.
 */
import '@testing-library/svelte/vitest';