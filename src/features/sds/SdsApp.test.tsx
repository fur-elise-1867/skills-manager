import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import SdsApp from "./SdsApp";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command !== "runtime_profile") throw new Error(`Unexpected upstream command: ${command}`);
    return { kind: "sds", environment: "internal", fixtureMode: false } as never;
  });
});

test("rejects_runtime_profile_mismatch", async () => {
  vi.mocked(invoke).mockResolvedValue({ kind: "upstream", environment: "internal", fixtureMode: false });
  render(<SdsApp />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Desktop profile mismatch");
  expect(screen.queryByRole("navigation")).not.toBeInTheDocument();
});

test("does_not_mount_upstream_context", async () => {
  render(<SdsApp />);
  await screen.findByRole("navigation");
  expect(vi.mocked(invoke).mock.calls.map(([command]) => command)).toEqual(["runtime_profile"]);
});

test("renders_keyboard_accessible_shell", async () => {
  render(<SdsApp />);
  await screen.findByRole("navigation");
  const skip = screen.getByRole("link", { name: "Skip to content" });
  skip.focus();
  expect(skip).toHaveFocus();
  expect(screen.getByRole("button", { name: "Skills Store" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Library" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Switch to dark theme" }));
  await waitFor(() => expect(localStorage.getItem("sds.theme")).toBe("dark"));
  expect(localStorage.getItem("theme")).toBeNull();
});

test("rejects_backend_environment_and_fixture_mismatch", async () => {
  vi.mocked(invoke).mockResolvedValue({ kind: "sds", environment: "development", fixtureMode: true });
  render(<SdsApp />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Desktop profile mismatch");
});
