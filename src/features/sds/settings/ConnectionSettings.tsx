import type { ProfileInfo, StoreProfile } from "../contracts";
import { useState } from "react";
import { saveConnection } from "../api";
import { storeErrorMessage } from "../errors";

export default function ConnectionSettings({ current, environment, onSaved }: { current: StoreProfile | null; environment: ProfileInfo["environment"]; onSaved: (profile: StoreProfile) => void }) {
  const [url, setUrl] = useState(current?.baseUrl ?? "");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  return <section className="sds-welcome" aria-labelledby="sds-connection-title">
    <h2 id="sds-connection-title">Skills Store connection</h2>
    <p>Enter the Store address provided by your administrator, including its application path.</p>
    <form className="sds-form" onSubmit={async (event) => {
      event.preventDefault(); if (pending) return;
      setPending(true); setError("");
      try { onSaved(await saveConnection(url.trim())); }
      catch (failure) { setError(storeErrorMessage(failure)); }
      finally { setPending(false); }
    }}>
      <label htmlFor="sds-store-url">Store URL</label>
      <input id="sds-store-url" type="url" value={url} required disabled={pending} placeholder={environment === "development" ? "http://127.0.0.1:8080/skm" : "https://store.example/skm"} onChange={(event) => setUrl(event.target.value)} />
      <p className="sds-field-help">{environment === "development" ? "HTTP is permitted only for loopback addresses in this local development build." : "This build requires HTTPS with a trusted certificate."} Changing the Store address signs you out.</p>
      {error && <p className="sds-error" role="alert">{error}</p>}
      <button className="sds-primary-button" type="submit" disabled={pending}>{pending ? "Saving…" : "Save connection"}</button>
    </form>
  </section>;
}
