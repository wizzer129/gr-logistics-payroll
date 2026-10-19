//! A user's own preferences. Not business records, so no audit row (like `set_user_locale`).

use sqlx::PgPool;
use uuid::Uuid;

use crate::theme::Theme;

pub async fn set_theme(pool: &PgPool, user_id: Uuid, theme: Theme) -> Result<(), sqlx::Error> {
	sqlx::query("UPDATE users SET theme = $2 WHERE id = $1")
		.bind(user_id)
		.bind(theme.as_str())
		.execute(pool)
		.await?;
	Ok(())
}
