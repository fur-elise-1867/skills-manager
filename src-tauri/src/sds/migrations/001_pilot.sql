CREATE TABLE IF NOT EXISTS releases (
  cache_key TEXT PRIMARY KEY,
  profile_id TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  release_id TEXT NOT NULL,
  release_json TEXT NOT NULL,
  manifest_json TEXT NOT NULL,
  fingerprint TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(profile_id, skill_id, release_id)
);
CREATE TABLE IF NOT EXISTS deployments (
  deployment_id TEXT PRIMARY KEY,
  release_key TEXT NOT NULL REFERENCES releases(cache_key),
  target_key TEXT NOT NULL UNIQUE,
  target_json TEXT NOT NULL,
  manifest_json TEXT NOT NULL,
  fingerprint TEXT NOT NULL,
  mcp_json TEXT NOT NULL,
  installed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS operations (
  operation_id TEXT PRIMARY KEY,
  frozen_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  intent_json TEXT,
  runtime_epoch TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS catalog_cache (
  profile_id TEXT NOT NULL,
  account_id TEXT NOT NULL,
  query_key TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  fetched_at TEXT NOT NULL,
  PRIMARY KEY(profile_id, account_id, query_key)
);
PRAGMA user_version = 1;
