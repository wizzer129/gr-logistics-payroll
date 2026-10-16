// Mirrors the backend's rule (backend/src/models/auth.rs PASSWORD_MIN_CHARS/MAX_CHARS).
export const PASSWORD_MIN_CHARS = 12;
export const PASSWORD_MAX_CHARS = 128;

export function isValidPasswordLength(password: string): boolean {
	// Count characters like the backend does, not UTF-16 code units.
	const length = [...password].length;
	return length >= PASSWORD_MIN_CHARS && length <= PASSWORD_MAX_CHARS;
}
