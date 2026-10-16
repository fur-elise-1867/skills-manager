import { createContext } from "react";

export type SdsTheme = "light" | "dark";
export const SdsThemeContext = createContext({ theme: "light" as SdsTheme, toggle: () => {} });
