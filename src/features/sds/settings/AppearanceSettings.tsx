import { useContext, type ReactElement } from "react";
import { SdsThemeContext } from "../theme";

export default function AppearanceSettings(): ReactElement {
  const { theme, toggle } = useContext(SdsThemeContext);
  return (
    <section className="sds-welcome" aria-labelledby="sds-appearance-title">
      <h2 id="sds-appearance-title">Appearance</h2>
      <p>Choose light or dark theme for the interface.</p>
      <div className="sds-review-actions">
        <button
          type="button"
          className="sds-secondary-button"
          onClick={toggle}
          aria-label={theme === "light" ? "Switch to dark theme" : "Switch to light theme"}
        >
          {theme === "light" ? "Switch to dark theme" : "Switch to light theme"}
        </button>
      </div>
    </section>
  );
}
