pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.2} KB", b / KB)
    } else if b < GB {
        format!("{:.2} MB", b / MB)
    } else if b < TB {
        format!("{:.2} GB", b / GB)
    } else {
        format!("{:.2} TB", b / TB)
    }
}

pub fn format_duration(secs: f64) -> String {
    if secs < 1.0 {
        format!("{:.0} ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{secs:.2} s")
    } else {
        let total = secs as u64;
        let mins = total / 60;
        let rem = secs - (mins as f64 * 60.0);
        if mins < 60 {
            format!("{mins}m {rem:.1}s")
        } else {
            let hrs = mins / 60;
            let rem_mins = mins % 60;
            format!("{hrs}h {rem_mins}m {rem:.0}s")
        }
    }
}

/// Parses a Robocopy numeric token with optional `[KMGTP]` suffix (case-insensitive)
/// as printed in Robocopy summaries (e.g. `1234`, `4.88 k`, `1.5m`, `20G`) or throttling flags.
pub fn parse_robocopy_size_token(raw: &str) -> Option<u64> {
    let cleaned: String = raw.trim().chars().filter(|c| *c != ',').collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut split_idx = trimmed.len();
    for (idx, ch) in trimmed.char_indices() {
        if !ch.is_ascii_digit() && ch != '.' {
            split_idx = idx;
            break;
        }
    }

    let num_part = trimmed[..split_idx].trim();
    let suffix_part = trimmed[split_idx..].trim();
    let val: f64 = num_part.parse().ok()?;
    let mult: f64 = match suffix_part.to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((val * mult).round() as u64)
}

pub fn is_valid_kmg_spec(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let last = trimmed.chars().last().unwrap_or('\0');
    let digits = if matches!(last.to_ascii_uppercase(), 'K' | 'M' | 'G') {
        &trimmed[..trimmed.len() - last.len_utf8()]
    } else {
        trimmed
    };
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_token_parsing() {
        assert_eq!(parse_robocopy_size_token("512"), Some(512));
        assert_eq!(parse_robocopy_size_token("1,024"), Some(1024));
        assert_eq!(parse_robocopy_size_token("2.5 m"), Some(2_621_440));
        assert_eq!(parse_robocopy_size_token("10G"), Some(10_737_418_240));
        assert!(is_valid_kmg_spec("64K"));
        assert!(is_valid_kmg_spec("100M"));
        assert!(is_valid_kmg_spec("4096"));
        assert!(!is_valid_kmg_spec("64X"));
    }
}
