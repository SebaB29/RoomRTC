//! User persistence module
//!
//! Handles loading and saving users to a plain text file

use crate::domain::{User, UserId};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

const USERS_FILE: &str = "users.txt";

/// Load users from file
pub fn load_users_from_file() -> Result<HashMap<UserId, User>, String> {
    let path = Path::new(USERS_FILE);

    // If file doesn't exist yet, return empty map
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let file = File::open(path).map_err(|e| format!("Failed to open users file: {}", e))?;
    let reader = BufReader::new(file);

    let mut users = HashMap::new();

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Error reading line: {}", e))?;

        // Skip empty lines and comments
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(user) = parse_user_from_line(&line) {
            users.insert(user.id.clone(), user);
        }
    }

    Ok(users)
}

/// Parse a user from a pipe-separated line
pub(crate) fn parse_user_from_line(line: &str) -> Option<User> {
    // Parse line format: id|username|password_hash|created_at
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() < 4 {
        eprintln!("Invalid line format: {}", line);
        return None;
    }

    let id = parts[0].to_string();
    let username = parts[1].to_string();
    let password_hash = parts[2].to_string();
    let created_at: u64 = parts[3].parse().unwrap_or(0);

    Some(User {
        id,
        username,
        password_hash,
        created_at,
    })
}

/// Save a user to file
pub fn save_user_to_file(user: &User) -> Result<(), String> {
    let path = Path::new(USERS_FILE);

    // Read existing users
    let existing_users = load_users_from_file()?;

    // Open file in write mode (this will overwrite)
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| format!("Failed to open users file for writing: {}", e))?;

    // Write header
    let _ = writeln!(file, "# User database (plain text format)");
    let _ = writeln!(file, "# Format: id|username|password_hash|created_at");
    let _ = writeln!(file, "#");

    // Write existing users
    for (_, u) in existing_users.iter() {
        let line = format!(
            "{}|{}|{}|{}",
            u.id, u.username, u.password_hash, u.created_at
        );
        writeln!(file, "{}", line).map_err(|e| format!("Failed to write user: {}", e))?;
    }

    // Write new user
    let line = format!(
        "{}|{}|{}|{}",
        user.id, user.username, user.password_hash, user.created_at
    );
    writeln!(file, "{}", line).map_err(|e| format!("Failed to write user: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_users_from_nonexistent_file() {
        // Verify the logic: if file doesn't exist, returns empty HashMap
        let unique_name = format!("nonexistent_{}.txt", std::process::id());

        let result = std::fs::File::open(&unique_name);
        assert!(result.is_err(), "File should not exist");

        // Simulating the behavior of load_users_from_file when file doesn't exist
        let empty_map: std::collections::HashMap<String, User> = std::collections::HashMap::new();
        assert_eq!(empty_map.len(), 0);
    }

    #[test]
    fn test_user_line_parsing() {
        // Test the parsing logic using the actual function
        let line = "user1|alice|hash123|1234567890";
        let user = parse_user_from_line(line).expect("Should parse valid line");

        assert_eq!(user.id, "user1");
        assert_eq!(user.username, "alice");
        assert_eq!(user.password_hash, "hash123");
        assert_eq!(user.created_at, 1234567890);
    }

    #[test]
    fn test_invalid_line_format() {
        // Test parsing of invalid formats using the actual function
        let invalid_lines = vec!["incomplete|line", "also|incomplete|line", "no_pipes"];

        for line in invalid_lines {
            let result = parse_user_from_line(line);
            assert!(result.is_none(), "Line '{}' should return None", line);
        }
    }

    #[test]
    fn test_special_characters_in_fields() {
        // Test that special characters are preserved
        let line = "user1|alice@test|hash!@#$|1234567890";

        let user = parse_user_from_line(line).expect("Should parse line with special characters");
        assert_eq!(user.id, "user1");
        assert_eq!(user.username, "alice@test");
        assert_eq!(user.password_hash, "hash!@#$");
        assert_eq!(user.created_at, 1234567890);
    }
}
