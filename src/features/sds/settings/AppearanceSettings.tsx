import { useContext, type ReactElement } from "react";
import { SdsThemeContext } from "../theme";

export default function AppearanceSettings(): ReactElement {
  const { theme, toggle } = useContext(SdsThemeContext);
  const themeActionLabel = theme === "light" ? "Switch to dark theme" : "Switch to light theme";
  return (
    <section className="sds-welcome" aria-labelledby="sds-appearance-title">
      <h2 id="sds-appearance-title">Appearance</h2>
      <p>Choose light or dark theme for the interface.</p>
      <div className="sds-review-actions">
        <button
          type="button"
          className="sds-secondary-button"
          onClick={toggle}
          aria-label={themeActionLabel}
        >
          {themeActionLabel}
        </button>
      </div>
    </section>
  );
}
