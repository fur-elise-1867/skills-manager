import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { SdsThemeContext } from "./theme";
import type { SdsTheme } from "./theme";

export default function SdsThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<SdsTheme>(() => {
    try { return localStorage.getItem("sds.theme") === "dark" ? "dark" : "light"; }
    catch { return "light"; }
  });
  useEffect(() => {
    document.documentElement.dataset.sdsTheme = theme;
    try { localStorage.setItem("sds.theme", theme); } catch { /* A session can still change theme without disk persistence. */ }
  }, [theme]);
  return <SdsThemeContext.Provider value={{ theme, toggle: () => setTheme((value) => value === "light" ? "dark" : "light") }}>{children}</SdsThemeContext.Provider>;
}
