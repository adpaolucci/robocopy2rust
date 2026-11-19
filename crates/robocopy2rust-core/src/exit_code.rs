use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitCodeSeverity {
    Success,
    SuccessWithChanges,
    SuccessWithNotices,
    CopyError,
    FatalError,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitCodeInfo {
    pub code: i32,
    pub severity: ExitCodeSeverity,
    pub short_label: String,
    pub summary: String,
    pub bit_descriptions: Vec<String>,
    pub is_success: bool,
}

/// Decodes Microsoft Robocopy's bitmask exit code (`0..=16`, or process/cancellation codes).
///
/// Robocopy treats exit codes `0..=7` as successful executions and `>= 8` as failures.
pub fn decode_exit_code(code: i32) -> ExitCodeInfo {
    if code < 0 {
        return ExitCodeInfo {
            code,
            severity: ExitCodeSeverity::Cancelled,
            short_label: "Cancelled / Terminated".to_string(),
            summary: format!("Process did not complete normally (exit code {code})."),
            bit_descriptions: vec!["Process terminated or cancelled by user before exit".to_string()],
            is_success: false,
        };
    }

    if code == 0 {
        return ExitCodeInfo {
            code: 0,
            severity: ExitCodeSeverity::Success,
            short_label: "0 - No Changes".to_string(),
            summary: "No files were copied. No failure was encountered. Source and destination are synchronized."
                .to_string(),
            bit_descriptions: vec!["0x00: No files copied; no failures; no extra or mismatched files".to_string()],
            is_success: true,
        };
    }

    let mut bits = Vec::new();
    if (code & 0x01) != 0 {
        bits.push("0x01 (1): One or more files were copied successfully (new or updated files).".to_string());
    }
    if (code & 0x02) != 0 {
        bits.push("0x02 (2): Some Extra files or directories were detected in destination.".to_string());
    }
    if (code & 0x04) != 0 {
        bits.push(
            "0x04 (4): Some Mismatched files or directories were detected (housekeeping check recommended)."
                .to_string(),
        );
    }
    if (code & 0x08) != 0 {
        bits.push("0x08 (8): Some files or directories could not be copied (retry limit exceeded).".to_string());
    }
    if (code & 0x10) != 0 {
        bits.push(
            "0x10 (16): Serious/fatal error. Usage error, invalid parameters, or insufficient access privileges."
                .to_string(),
        );
    }
    if (code & !0x1F) != 0 {
        bits.push(format!(
            "Non-standard OS/Robocopy exit bits detected (0x{:X}).",
            code & !0x1F
        ));
    }

    let is_success = (0..=7).contains(&code);
    let severity = if (code & 0x10) != 0 || code > 31 {
        ExitCodeSeverity::FatalError
    } else if (code & 0x08) != 0 {
        ExitCodeSeverity::CopyError
    } else if (code & 0x06) != 0 {
        ExitCodeSeverity::SuccessWithNotices
    } else {
        ExitCodeSeverity::SuccessWithChanges
    };

    let short_label = match code {
        1 => "1 - Files Copied".to_string(),
        2 => "2 - Extra Files".to_string(),
        3 => "3 - Copied + Extra".to_string(),
        4 => "4 - Mismatches".to_string(),
        5 => "5 - Copied + Mismatches".to_string(),
        6 => "6 - Extra + Mismatches".to_string(),
        7 => "7 - Copied + Extra + Mismatches".to_string(),
        8 => "8 - Copy Failures".to_string(),
        16 => "16 - Fatal Error".to_string(),
        _ if is_success => format!("{code} - Success (Bitmask)"),
        _ => format!("{code} - Error (Bitmask)"),
    };

    let summary = if is_success {
        format!("Success (code {code} <= 7): {}", bits.join(" "))
    } else {
        format!("Failure (code {code} >= 8): {}", bits.join(" "))
    };

    ExitCodeInfo {
        code,
        severity,
        short_label,
        summary,
        bit_descriptions: bits,
        is_success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_all_standard_exit_codes() {
        for code in 0..=7 {
            let info = decode_exit_code(code);
            assert!(info.is_success, "code {code} must be success");
        }
        for code in 8..=16 {
            let info = decode_exit_code(code);
            assert!(!info.is_success, "code {code} must be failure");
        }

        let three = decode_exit_code(3);
        assert_eq!(three.bit_descriptions.len(), 2);
        assert_eq!(three.severity, ExitCodeSeverity::SuccessWithNotices);

        let seven = decode_exit_code(7);
        assert_eq!(seven.bit_descriptions.len(), 3);

        let sixteen = decode_exit_code(16);
        assert_eq!(sixteen.severity, ExitCodeSeverity::FatalError);
    }
}
