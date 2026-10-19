-- Each user's color theme for the web app, picked in Settings. Light/dark mode stays a
-- per-device choice (the toggle in the app bar); this is the palette both modes come from.
ALTER TABLE users
	ADD COLUMN theme text NOT NULL DEFAULT 'default'
		CHECK (theme IN ('default', 'catppuccin', 'dracula', 'autumn'));
