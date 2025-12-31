use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use password_hash::{PasswordHash, SaltString};
use rand_core::OsRng;

pub fn normalize_slug(input: &str) -> Option<String> {
    let slug = input.trim().to_lowercase().replace(' ', "-");
    if slug.is_empty() {
        return None;
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    Some(slug)
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| "Failed to hash password.".to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<(), String> {
    let parsed = PasswordHash::new(hash).map_err(|_| "Invalid password hash.".to_string())?;
    let argon2 = Argon2::default();
    argon2
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| "Invalid credentials.".to_string())
}
