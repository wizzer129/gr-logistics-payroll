//! Password reset tokens (EMAIL-SPEC.md §7): 32 random bytes, base64url in the emailed link.
//! Only the SHA-256 is stored. The token is high-entropy, so a fast hash is the right tool
//! here; argon2 is for low-entropy passwords.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// `(token for the link, hash to store)`.
pub fn generate() -> (String, Vec<u8>) {
	let mut bytes = [0u8; 32];
	getrandom::fill(&mut bytes).expect("the OS random number generator is available");
	let token = URL_SAFE_NO_PAD.encode(bytes);
	let hash = hash(&token);
	(token, hash)
}

pub fn hash(token: &str) -> Vec<u8> {
	Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tokens_are_url_safe_unique_and_hash_consistently() {
		let (a, a_hash) = generate();
		let (b, _) = generate();
		assert_ne!(a, b);
		assert_eq!(a.len(), 43, "32 bytes, base64url without padding");
		assert!(
			a.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
		);
		assert_eq!(hash(&a), a_hash);
		assert_eq!(a_hash.len(), 32);
	}
}
