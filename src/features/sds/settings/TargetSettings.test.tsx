import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import TargetSettings from "./TargetSettings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

test("chooses an existing folder through Settings and displays only the backend resolved destination", async () => {
  const target = { agentKey: "aipro", workspaceId: "global", path: "C:\\Selected\\skills" };
  vi.mocked(invoke).mockImplementation(async (command) => command === "sds_get_target_settings"
    ? { overrideRoot: null, target: null, error: { code: "TARGET_NOT_READY" }, locked: false }
    : { overrideRoot: target.path, target, error: null, locked: false });
  vi.mocked(open).mockResolvedValue(target.path);
  render(<TargetSettings />);
  expect(await screen.findByText(/AIPro is not ready/)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Choose skills folder" }));
  expect(await screen.findByText(target.path)).toBeVisible();
  expect(invoke).toHaveBeenCalledWith("sds_set_target_override", { overrideRoot: target.path });
  expect(open).toHaveBeenCalledWith(expect.objectContaining({ directory: true, multiple: false }));
  fireEvent.click(screen.getByRole("button", { name: "Use default destination" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("sds_set_target_override", { overrideRoot: null }));
});

test("fixture destinations cannot be changed and choosing a folder never installs AIPro", async () => {
  vi.mocked(invoke).mockResolvedValue({ overrideRoot: null, target: { agentKey: "aipro", workspaceId: "global", path: "C:\\fixture\\agent-home\\.aipro\\skills" }, error: null, locked: true });
  render(<TargetSettings />);
  await screen.findByText("C:\\fixture\\agent-home\\.aipro\\skills");
  expect(await screen.findByRole("button", { name: "Choose skills folder" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Use default destination" })).toBeDisabled();
});
