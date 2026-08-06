use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DriveIdError {
    #[error("drive id must contain at least one letter or digit")]
    Empty,
}

/// Normalizes a free-form drive identifier into kebab-case: lowercases,
/// collapses any run of non-alphanumeric characters (spaces, underscores,
/// punctuation) into a single `-`, and trims leading/trailing `-`.
pub fn normalize_drive_id(input: &str) -> Result<String, DriveIdError> {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = true; // suppress a leading dash
    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        Err(DriveIdError::Empty)
    } else {
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_replaces_spaces() {
        assert_eq!(normalize_drive_id("My Documents").unwrap(), "my-documents");
    }

    #[test]
    fn collapses_repeated_separators() {
        assert_eq!(normalize_drive_id("My   Work_Drive!!").unwrap(), "my-work-drive");
    }

    #[test]
    fn trims_leading_and_trailing_punctuation() {
        assert_eq!(normalize_drive_id("  -Default- ").unwrap(), "default");
    }

    #[test]
    fn already_kebab_case_is_unchanged() {
        assert_eq!(normalize_drive_id("work-drive").unwrap(), "work-drive");
    }

    #[test]
    fn all_punctuation_is_empty() {
        assert_eq!(normalize_drive_id("   !!! "), Err(DriveIdError::Empty));
    }

    #[test]
    fn empty_string_is_empty() {
        assert_eq!(normalize_drive_id(""), Err(DriveIdError::Empty));
    }
}
