-- Password reset links (EMAIL-SPEC.md §7). Only a SHA-256 of each token is stored, so a
-- database leak can't be turned into working reset links.
CREATE TABLE password_reset_tokens (
	id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
	user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
	token_hash bytea NOT NULL UNIQUE,
	expires_at timestamptz NOT NULL,
	-- Set when the token is used, and on every older unused token when a new one is issued.
	used_at timestamptz,
	created_at timestamptz NOT NULL DEFAULT now()
);

-- The per-user rate limit counts a user's recent tokens.
CREATE INDEX password_reset_tokens_user_created ON password_reset_tokens (user_id, created_at);
