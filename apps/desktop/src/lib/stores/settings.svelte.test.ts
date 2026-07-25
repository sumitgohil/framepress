import { beforeEach, describe, expect, it, vi } from "vitest";

async function load_settings(browser = true) {
  vi.resetModules();
  vi.doMock("$app/environment", () => ({ browser }));
  return import("./settings.svelte");
}

describe("settings store", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("uses safe defaults when storage is unavailable or malformed", async () => {
    localStorage.setItem("framepress:settings", "{not-json");
    const { settings } = await load_settings();

    expect(settings.value).toEqual({
      default_preset: "website",
      output_behavior: "sidecar",
    });
  });

  it("merges persisted values with defaults and persists updates", async () => {
    localStorage.setItem(
      "framepress:settings",
      JSON.stringify({ default_preset: "email" }),
    );
    const { settings } = await load_settings();

    expect(settings.value).toEqual({
      default_preset: "email",
      output_behavior: "sidecar",
    });

    settings.set({ output_behavior: "in-place" });

    expect(settings.value).toEqual({
      default_preset: "email",
      output_behavior: "in-place",
    });
    expect(
      JSON.parse(localStorage.getItem("framepress:settings") ?? "{}"),
    ).toEqual(settings.value);
  });

  it("resets both the in-memory state and persisted value", async () => {
    const { settings } = await load_settings();
    settings.set({
      default_preset: "social_media",
      output_behavior: "in-place",
    });

    settings.reset();

    expect(settings.value).toEqual({
      default_preset: "website",
      output_behavior: "sidecar",
    });
    expect(
      JSON.parse(localStorage.getItem("framepress:settings") ?? "{}"),
    ).toEqual(settings.value);
  });

  it("does not access local storage during server rendering", async () => {
    localStorage.setItem(
      "framepress:settings",
      JSON.stringify({ default_preset: "email" }),
    );
    const { settings } = await load_settings(false);

    expect(settings.value).toEqual({
      default_preset: "website",
      output_behavior: "sidecar",
    });
  });
});
