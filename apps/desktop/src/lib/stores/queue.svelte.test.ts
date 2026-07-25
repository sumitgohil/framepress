import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { QueueItem } from "$lib/ipc/types";

const mocks = vi.hoisted(() => ({
  cancelJob: vi.fn(),
  listen: vi.fn(),
  pauseQueue: vi.fn(),
  queueSnapshot: vi.fn(),
  resumeQueue: vi.fn(),
  toastError: vi.fn(),
  toastSuccess: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("$lib/ipc/commands", () => ({
  cancelJob: mocks.cancelJob,
  pauseQueue: mocks.pauseQueue,
  queueSnapshot: mocks.queueSnapshot,
  resumeQueue: mocks.resumeQueue,
}));
vi.mock("$lib/stores/toast.svelte", () => ({
  toast: { error: mocks.toastError, success: mocks.toastSuccess },
}));

const pending_item = {
  id: "job-1",
  input_path: "/images/photo.png",
  output_path: null,
  format: "png" as const,
  preset: "website" as const,
  source: "desktop",
  status: "pending" as const,
  original_bytes: null,
  optimized_bytes: null,
  engine: null,
  dssim: null,
  savings_pct: null,
  margin_pct: null,
  error_message: null,
  candidates_log: null,
  started_at: null,
  completed_at: null,
};

async function load_queue() {
  vi.resetModules();
  return import("./queue.svelte");
}

describe("queue store", () => {
  let event_handler: ((event: { payload: QueueItem }) => void) | undefined;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    mocks.queueSnapshot.mockResolvedValue([]);
    mocks.listen.mockImplementation(async (_event, handler) => {
      event_handler = handler;
      return mocks.unlisten;
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("subscribes before taking a snapshot and keeps initialization idempotent", async () => {
    const { queue } = await load_queue();
    await Promise.all([queue.init(), queue.init()]);

    expect(mocks.listen).toHaveBeenCalledTimes(1);
    expect(mocks.queueSnapshot).toHaveBeenCalledTimes(1);
    expect(queue.initialised).toBe(true);

    await vi.advanceTimersByTimeAsync(2_000);
    expect(mocks.queueSnapshot).toHaveBeenCalledTimes(2);

    queue.dispose();
    expect(mocks.unlisten).toHaveBeenCalledTimes(1);
  });

  it("inserts new event items first and replaces existing items in place", async () => {
    const { queue } = await load_queue();
    await queue.init();

    event_handler?.({ payload: pending_item });
    event_handler?.({
      payload: { ...pending_item, status: "running", started_at: 1 },
    });

    expect(queue.items).toEqual([
      { ...pending_item, status: "running", started_at: 1 },
    ]);
    expect(queue.active_count).toBe(1);
    queue.dispose();
  });

  it("announces a completion only once and tracks terminal work", async () => {
    const { queue } = await load_queue();
    await queue.init();
    const completed = {
      ...pending_item,
      status: "completed" as const,
      engine: "oxipng" as const,
      savings_pct: 34,
      margin_pct: 12,
      output_path: "/images/photo-framepress.png",
      optimized_bytes: 66,
      completed_at: 2,
    };

    event_handler?.({ payload: completed });
    event_handler?.({ payload: completed });

    expect(mocks.toastSuccess).toHaveBeenCalledTimes(1);
    expect(mocks.toastSuccess).toHaveBeenCalledWith(
      "photo.png — saved 34%",
      "oxipng beat the runner-up by 12%",
    );
    expect(queue.completed_count).toBe(1);
    queue.dispose();
  });

  it("delegates cancel and pause controls to the typed IPC layer", async () => {
    const { queue } = await load_queue();
    await queue.init();

    await queue.cancel("job-1");
    await queue.set_paused(true);
    await queue.set_paused(false);

    expect(mocks.cancelJob).toHaveBeenCalledWith("job-1");
    expect(mocks.queueSnapshot).toHaveBeenCalledTimes(2);
    expect(mocks.pauseQueue).toHaveBeenCalledTimes(1);
    expect(mocks.resumeQueue).toHaveBeenCalledTimes(1);
    expect(queue.paused).toBe(false);
    queue.dispose();
  });
});
