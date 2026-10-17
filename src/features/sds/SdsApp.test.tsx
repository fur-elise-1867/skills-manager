import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import SdsApp from "./SdsApp";
import taxonomy from "../../../contracts/sds-desktop/v1/fixtures/taxonomy.json";
import catalog from "../../../contracts/sds-desktop/v1/fixtures/catalog.json";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === "runtime_profile") return { kind: "sds", environment: "internal", fixtureMode: false } as never;
    if (command === "sds_get_connection") return null as never;
    if (command === "sds_library") return [] as never;
    if (command === "sds_session_status") return { status: "signedOut", user: null } as never;
    if (command === "sds_get_target_settings") return { overrideRoot: null, target: null, error: { code: "TARGET_NOT_READY" }, locked: false } as never;
    throw new Error(`Unexpected upstream command: ${command}`);
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
  await screen.findByRole("heading", { name: "AIPro destination" });
  await waitFor(() => expect(vi.mocked(invoke).mock.calls.map(([command]) => command).sort()).toEqual(["runtime_profile", "sds_get_connection", "sds_get_target_settings", "sds_session_status"]));
});

test("configures a real Store profile and signs in through the SDS shell", async () => {
  let connection: unknown = null;
  let session = { status: "signedOut", user: null } as unknown;
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === "runtime_profile") return { kind: "sds", environment: "internal", fixtureMode: false } as never;
    if (command === "sds_get_connection") return connection as never;
    if (command === "sds_session_status") return session as never;
    if (command === "sds_get_target_settings") return { overrideRoot: null, target: null, error: { code: "TARGET_NOT_READY" }, locked: false } as never;
    if (command === "sds_set_connection") { connection = { id: "profile-one", baseUrl: "https://store.example.invalid/skm", environment: "internal" }; return connection as never; }
    if (command === "sds_login") { session = { status: "authenticated", user: { id: "1001", displayName: "Example Employee" } }; return session as never; }
    if (command === "sds_catalog_taxonomy") return taxonomy.data as never;
    if (command === "sds_catalog_search") return catalog.data as never;
    throw new Error(`Unexpected upstream command: ${command}`);
  });
  render(<SdsApp />);
  fireEvent.change(await screen.findByLabelText("Store URL"), { target: { value: "https://store.example.invalid/skm" } });
  fireEvent.click(screen.getByRole("button", { name: "Save connection" }));
  await screen.findByRole("button", { name: "Sign in" });
  fireEvent.change(screen.getByLabelText("Email"), { target: { value: "employee@example.invalid" } });
  fireEvent.change(screen.getByLabelText("Password"), { target: { value: "synthetic-password" } });
  fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
  expect(await screen.findByText("Example Employee")).toBeInTheDocument();
  await screen.findByRole("button",{name:"View Jira Weekly Report"});
  fireEvent.click(screen.getByRole("button",{name:"Settings"}));
  fireEvent.click(screen.getByRole("button",{name:"Save connection"}));
  await waitFor(()=>expect(screen.getByText("Example Employee")).toBeInTheDocument());
  expect(screen.queryByRole("button",{name:"Sign in"})).not.toBeInTheDocument();
});

test("renders_keyboard_accessible_shell", async () => {
  render(<SdsApp />);
  await screen.findByRole("navigation");
  const skip = screen.getByRole("link", { name: "Skip to content" });
  skip.focus();
  expect(skip).toHaveFocus();
  expect(screen.getByRole("button", { name: "Skills Store" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Library" })).toBeEnabled();
  fireEvent.click(screen.getByRole("button", { name: "Switch to dark theme" }));
  await waitFor(() => expect(localStorage.getItem("sds.theme")).toBe("dark"));
  expect(localStorage.getItem("theme")).toBeNull();
});

test("opens local Library without a Store connection or signed-in session",async()=>{
  render(<SdsApp/>);
  await screen.findByRole("heading",{name:"AIPro destination"});
  fireEvent.click(screen.getByRole("button",{name:"Library"}));
  expect(await screen.findByText("No downloaded releases yet.")).toBeInTheDocument();
  expect(screen.getByRole("heading",{name:"Library",level:1})).toBeInTheDocument();
  expect(vi.mocked(invoke).mock.calls.map(([command])=>command)).toContain("sds_library");
  expect(vi.mocked(invoke).mock.calls.map(([command])=>command)).not.toContain("sds_catalog_search");
});

test("rejects_backend_environment_and_fixture_mismatch", async () => {
  vi.mocked(invoke).mockResolvedValue({ kind: "sds", environment: "development", fixtureMode: true });
  render(<SdsApp />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Desktop profile mismatch");
});
