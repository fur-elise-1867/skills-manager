import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import SdsThemeProvider from "../SdsThemeProvider";
import AppearanceSettings from "./AppearanceSettings";

beforeEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.sdsTheme;
});

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.sdsTheme;
  vi.restoreAllMocks();
});

test("toggles theme using real provider, updates document dataset, and supports keyboard focus", () => {
  render(
    <SdsThemeProvider>
      <AppearanceSettings />
    </SdsThemeProvider>
  );

  const button = screen.getByRole("button", { name: "Switch to dark theme" });
  expect(button).toBeInTheDocument();
  expect(document.documentElement.dataset.sdsTheme).toBe("light");

  button.focus();
  expect(button).toHaveFocus();

  fireEvent.click(button);

  expect(document.documentElement.dataset.sdsTheme).toBe("dark");
  expect(localStorage.getItem("sds.theme")).toBe("dark");
  expect(screen.getByRole("button", { name: "Switch to light theme" })).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Switch to light theme" }));

  expect(document.documentElement.dataset.sdsTheme).toBe("light");
  expect(localStorage.getItem("sds.theme")).toBe("light");
  expect(screen.getByRole("button", { name: "Switch to dark theme" })).toBeInTheDocument();
});

test("remounting with real provider restores persisted sds.theme from localStorage", () => {
  localStorage.setItem("sds.theme", "dark");

  const view = render(
    <SdsThemeProvider>
      <AppearanceSettings />
    </SdsThemeProvider>
  );

  expect(document.documentElement.dataset.sdsTheme).toBe("dark");
  expect(screen.getByRole("button", { name: "Switch to light theme" })).toBeInTheDocument();

  view.unmount();

  render(
    <SdsThemeProvider>
      <AppearanceSettings />
    </SdsThemeProvider>
  );

  expect(document.documentElement.dataset.sdsTheme).toBe("dark");
  expect(screen.getByRole("button", { name: "Switch to light theme" })).toBeInTheDocument();
});

test("theme can still change within session when localStorage throws", () => {
  vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new DOMException("QuotaExceededError", "QuotaExceededError");
  });

  render(
    <SdsThemeProvider>
      <AppearanceSettings />
    </SdsThemeProvider>
  );

  const button = screen.getByRole("button", { name: "Switch to dark theme" });
  fireEvent.click(button);

  expect(document.documentElement.dataset.sdsTheme).toBe("dark");
  expect(screen.getByRole("button", { name: "Switch to light theme" })).toBeInTheDocument();
});
