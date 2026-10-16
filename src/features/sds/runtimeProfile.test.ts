import { expect, test } from "vitest";
import { assertRuntimeProfile } from "../../runtimeProfile";

test("rejects both frontend/backend profile mismatches before bootstrap", () => {
  expect(() => assertRuntimeProfile({ kind: "sds", environment: "internal", fixtureMode: false }, "upstream")).toThrow("Desktop profile mismatch");
  expect(() => assertRuntimeProfile({ kind: "upstream", environment: "internal", fixtureMode: false }, "sds")).toThrow("Desktop profile mismatch");
});

test("validates the environment and fixture flag when specified", () => {
  const profile = { kind: "sds", environment: "development", fixtureMode: false } as const;
  expect(() => assertRuntimeProfile(profile, "sds", "internal", false)).toThrow();
  expect(() => assertRuntimeProfile(profile, "sds", "development", true)).toThrow();
  expect(() => assertRuntimeProfile(profile, "sds", "development", false)).not.toThrow();
});
