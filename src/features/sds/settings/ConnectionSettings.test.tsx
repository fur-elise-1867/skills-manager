import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import ConnectionSettings from "./ConnectionSettings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

test("saves the Store URL through the desktop boundary and returns its validated profile", async () => {
  const profile = { id: "profile-one", baseUrl: "http://127.0.0.1:8080/skm", environment: "development" };
  vi.mocked(invoke).mockResolvedValue(profile);
  const saved = vi.fn();
  render(<ConnectionSettings current={null} environment="development" onSaved={saved} />);
  fireEvent.change(screen.getByLabelText("Store URL"), { target: { value: profile.baseUrl } });
  fireEvent.click(screen.getByRole("button", { name: "Save connection" }));
  await waitFor(() => expect(saved).toHaveBeenCalledWith(profile));
  expect(invoke).toHaveBeenCalledWith("sds_set_connection", { baseUrl: profile.baseUrl });
});
