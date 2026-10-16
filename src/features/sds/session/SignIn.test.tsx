import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import SignIn from "./SignIn";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

test("clears the password immediately after submit and completes only after Store accepts login", async () => {
  let finish!: (value: unknown) => void;
  vi.mocked(invoke).mockImplementation(() => new Promise((resolve) => { finish = resolve as typeof finish; }));
  const signedIn = vi.fn();
  render(<SignIn onSignedIn={signedIn} />);
  fireEvent.change(screen.getByLabelText("Email"), { target: { value: "employee@example.invalid" } });
  const password = screen.getByLabelText("Password");
  fireEvent.change(password, { target: { value: "synthetic-password" } });
  fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
  expect(password).toHaveValue("");
  expect(signedIn).not.toHaveBeenCalled();
  finish({ status: "authenticated", user: { id: "1001", displayName: "Example Employee" } });
  await waitFor(() => expect(signedIn).toHaveBeenCalledWith({ status: "authenticated", user: { id: "1001", displayName: "Example Employee" } }));
});

test("auth failure retains the email, clears password and never displays raw server content", async () => {
  vi.mocked(invoke).mockRejectedValue({ code: "AUTH_FAILED", message: "synthetic-password synthetic-access-token" });
  const signedIn = vi.fn();
  render(<SignIn onSignedIn={signedIn} />);
  fireEvent.change(screen.getByLabelText("Email"), { target: { value: "employee@example.invalid" } });
  fireEvent.change(screen.getByLabelText("Password"), { target: { value: "synthetic-password" } });
  fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Incorrect email or password");
  expect(screen.getByLabelText("Email")).toHaveValue("employee@example.invalid");
  expect(screen.getByLabelText("Password")).toHaveValue("");
  expect(screen.queryByText(/synthetic-access-token/)).not.toBeInTheDocument();
  expect(signedIn).not.toHaveBeenCalled();
});
