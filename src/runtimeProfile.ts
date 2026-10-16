import type { ProfileInfo } from "./features/sds/contracts";

export function assertRuntimeProfile(
  profile: ProfileInfo,
  expectedKind: ProfileInfo["kind"],
  expectedEnvironment?: ProfileInfo["environment"],
  expectedFixtures?: boolean,
) {
  if (profile.kind !== expectedKind ||
      (expectedEnvironment !== undefined && profile.environment !== expectedEnvironment) ||
      (expectedFixtures !== undefined && profile.fixtureMode !== expectedFixtures)) {
    throw new Error("Desktop profile mismatch. Start this application with its matching desktop runtime.");
  }
}
