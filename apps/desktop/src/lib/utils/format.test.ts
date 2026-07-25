import { describe, expect, it } from "vitest";

import { format_bytes } from "./format";

describe("format_bytes", () => {
  it("uses Finder-compatible decimal units", () => {
    expect(format_bytes(0)).toBe("0.00 B");
    expect(format_bytes(1000)).toBe("1.00 KB");
    expect(format_bytes(1_000_000)).toBe("1.00 MB");
    expect(format_bytes(1_000_000_000)).toBe("1.00 GB");
  });

  it("matches the decimal sizes of the uploaded leaderboard files", () => {
    expect(format_bytes(1_369_492)).toBe("1.37 MB");
    expect(format_bytes(63_272)).toBe("63.3 KB");
  });
});
