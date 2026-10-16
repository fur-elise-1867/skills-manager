const messages: Record<string, string> = {
  AUTH_FAILED: "Incorrect email or password. Check your details and try again.",
  ACCESS_DENIED: "You do not have permission to access this content.",
  ACCOUNT_UNAVAILABLE: "This account is unavailable. Contact your Store administrator.",
  STORE_UNAVAILABLE: "Skills Store is unavailable. Check your connection and try again.",
  SESSION_EXPIRED: "Your session has expired. Sign in again to access Skills Store.",
  CREDENTIAL_UNAVAILABLE: "Windows credential storage is unavailable. Your session could not be saved.",
  INVALID_CONFIGURATION: "Enter a valid Store URL. Use HTTPS, or loopback HTTP in local development.",
  REVIEW_REQUIRED: "Your connection or account changed. Review this action and try again.",
  RATE_LIMITED: "Too many requests. Wait before trying again.",
  INVALID_RESPONSE: "Skills Store returned an unsupported response. Contact your administrator.",
};

export function storeErrorMessage(error: unknown): string {
  const code = typeof error === "object" && error !== null && "code" in error ? String(error.code) : "";
  return messages[code] ?? "Unable to complete the Store request. Please try again.";
}
