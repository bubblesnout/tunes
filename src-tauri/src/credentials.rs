//! Password storage in the OS secret store (Windows Credential Manager / Secret Service).
//! Subsonic auth tokens are derived from the password per request and never expire,
//! so storing the password securely is what keeps the user signed in indefinitely.

const SERVICE: &str = "Tunes";

fn entry(server: &str, user: &str) -> keyring::Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, &format!("{user}@{server}"))
}

pub fn save(server: &str, user: &str, password: &str) -> anyhow::Result<()> {
    entry(server, user)?.set_password(password)?;
    Ok(())
}

pub fn load(server: &str, user: &str) -> Option<String> {
    entry(server, user).ok()?.get_password().ok()
}

pub fn delete(server: &str, user: &str) {
    if let Ok(e) = entry(server, user) {
        let _ = e.delete_credential();
    }
}
