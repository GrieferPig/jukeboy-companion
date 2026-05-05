import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  capabilities,
  hello,
  setCompanionTransport,
  snapshot,
  type CommandRequest,
  type CompanionEventPayload,
  type CompanionTransport,
} from "../../src/services/companion";

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((innerResolve) => {
    resolve = innerResolve;
  });
  return { promise, resolve };
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await new Promise<void>((resolve) => queueMicrotask(resolve));
}

describe("companion command queue", () => {
  beforeEach(() => {
    let activeCommands = 0;
    let maxActiveCommands = 0;
    const startedCommands: string[] = [];
    const completions = new Map<string, ReturnType<typeof deferred<void>>>();

    const transport: CompanionTransport = {
      listen: async (_handler: (payload: CompanionEventPayload) => void) => {
        return () => {};
      },
      async invoke<T>(command: string, _request?: CommandRequest): Promise<T> {
        activeCommands += 1;
        maxActiveCommands = Math.max(maxActiveCommands, activeCommands);
        startedCommands.push(command);

        const completion = deferred<void>();
        completions.set(command, completion);
        await completion.promise;

        activeCommands -= 1;
        return { command } as T;
      },
    };

    setCompanionTransport(transport);
    Object.assign(globalThis, {
      __companionQueueTest: {
        completions,
        getMaxActiveCommands: () => maxActiveCommands,
        getStartedCommands: () => [...startedCommands],
      },
    });
  });

  afterEach(() => {
    setCompanionTransport(null);
    Reflect.deleteProperty(globalThis, "__companionQueueTest");
  });

  it("runs queued commands one at a time in submission order", async () => {
    const queueTest = (globalThis as typeof globalThis & {
      __companionQueueTest: {
        completions: Map<string, ReturnType<typeof deferred<void>>>;
        getMaxActiveCommands: () => number;
        getStartedCommands: () => string[];
      };
    }).__companionQueueTest;

    const snapshotPromise = snapshot();
    const helloPromise = hello();
    const capabilitiesPromise = capabilities();

    await Promise.resolve();
    expect(queueTest.getStartedCommands()).toEqual(["companion_snapshot"]);
    expect(queueTest.getMaxActiveCommands()).toBe(1);

    queueTest.completions.get("companion_snapshot")?.resolve();
    await snapshotPromise;
    await flushMicrotasks();
    expect(queueTest.getStartedCommands()).toEqual([
      "companion_snapshot",
      "companion_hello",
    ]);
    expect(queueTest.getMaxActiveCommands()).toBe(1);

    queueTest.completions.get("companion_hello")?.resolve();
    await helloPromise;
    await flushMicrotasks();
    expect(queueTest.getStartedCommands()).toEqual([
      "companion_snapshot",
      "companion_hello",
      "companion_capabilities",
    ]);
    expect(queueTest.getMaxActiveCommands()).toBe(1);

    queueTest.completions.get("companion_capabilities")?.resolve();

    await expect(capabilitiesPromise).resolves.toEqual({ command: "companion_capabilities" });
  });
});