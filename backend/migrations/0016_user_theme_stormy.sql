-- A fifth theme, "Stormy Coral" (`stormy`). Widens 0015's CHECK; the frontend owns the palette.
ALTER TABLE users DROP CONSTRAINT users_theme_check;
ALTER TABLE users ADD CONSTRAINT users_theme_check
	CHECK (theme IN ('default', 'catppuccin', 'dracula', 'autumn', 'stormy'));
