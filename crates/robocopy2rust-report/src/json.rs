use anyhow::Result;
use chrono::Utc;
use robocopy2rust_runner::RunResult;
use serde::Serialize;

#[derive(Serialize)]
struct JsonReportEnvelope<'a> {
    schema: &'static str,
    generated_at: String,
    result: &'a RunResult,
}

pub fn render_json(result: &RunResult) -> Result<String> {
    let envelope = JsonReportEnvelope {
        schema: "robocopy2rust.audit.v1",
        generated_at: Utc::now().to_rfc3339(),
        result,
    };
    Ok(serde_json::to_string_pretty(&envelope)?)
}
