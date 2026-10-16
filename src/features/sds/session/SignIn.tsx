import { useEffect, useRef, useState } from "react";
import { signIn } from "../api";
import type { SessionView } from "../contracts";
import { storeErrorMessage } from "../errors";

export default function SignIn({ onSignedIn }: { onSignedIn: (view: SessionView) => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const mounted = useRef(true);
  const busy = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  return <section className="sds-welcome" aria-labelledby="sds-signin-title">
    <h2 id="sds-signin-title">Sign in to Skills Store</h2>
    <p>Use your existing Store account to browse and install internal skills.</p>
    <form className="sds-form" onSubmit={async (event) => {
      event.preventDefault();
      if (busy.current) return;
      busy.current = true;
      const submittedPassword = password;
      setPassword("");
      setError("");
      setPending(true);
      try {
        const view = await signIn(email.trim(), submittedPassword);
        if (mounted.current) onSignedIn(view);
      } catch (failure) {
        if (mounted.current) setError(storeErrorMessage(failure));
      } finally {
        busy.current = false;
        if (mounted.current) setPending(false);
      }
    }}>
      <label htmlFor="sds-email">Email</label><input id="sds-email" type="email" autoComplete="username" value={email} required disabled={pending} onChange={(event) => setEmail(event.target.value)} />
      <label htmlFor="sds-password">Password</label><input id="sds-password" type="password" autoComplete="current-password" value={password} required disabled={pending} onChange={(event) => setPassword(event.target.value)} />
      {error && <p role="alert" className="sds-error">{error}</p>}
      <button className="sds-primary-button" disabled={pending} type="submit">{pending ? "Signing in…" : "Sign in"}</button>
    </form>
  </section>;
}
