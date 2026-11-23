use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_runner::RunResult;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn render_html(result: &RunResult) -> String {
    let status_badge = if result.cancelled {
        ("CANCELLED", "#b45309")
    } else if result.exit_info.is_success {
        ("SUCCESS", "#15803d")
    } else {
        ("FAILED", "#b91c1c")
    };

    let mode_badge = if result.dry_run {
        "DRY-RUN (/L)"
    } else {
        "LIVE EXECUTION"
    };

    let bit_items: String = result
        .exit_info
        .bit_descriptions
        .iter()
        .map(|b| format!("<li>{}</li>", esc(b)))
        .collect::<Vec<_>>()
        .join("");

    let action_rows: String = result
        .file_actions
        .iter()
        .map(|rec| {
            let size_txt = rec.size_bytes.map(format_bytes).unwrap_or_else(|| "-".to_string());
            let ts_txt = rec.timestamp.as_deref().unwrap_or("-");
            format!(
                "<tr data-kind=\"{}\"><td><span class=\"tag\">{}</span></td><td class=\"num\">{}</td><td>{}</td><td class=\"mono\">{}</td></tr>",
                esc(rec.kind.label()),
                esc(rec.kind.label()),
                esc(&size_txt),
                esc(ts_txt),
                esc(&rec.path)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let error_section = if result.errors.is_empty() {
        "<p class=\"muted\">No Robocopy errors reported.</p>".to_string()
    } else {
        let items: String = result
            .errors
            .iter()
            .map(|e| format!("<li><code>{}</code></li>", esc(e)))
            .collect::<Vec<_>>()
            .join("\n");
        format!("<ul class=\"errors\">{items}</ul>")
    };

    let speed_str = match (result.summary.speed_bytes_per_sec, result.summary.speed_mb_per_min) {
        (Some(bps), Some(mbm)) => format!("{:.0} B/s ({:.2} MB/min)", bps, mbm),
        (Some(bps), None) => format!("{:.0} B/s", bps),
        (None, Some(mbm)) => format!("{:.2} MB/min", mbm),
        _ => "-".to_string(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none';">
<title>Robocopy2Rust Audit Report - {title}</title>
<style>
  :root {{
    --bg: #f3f4f6;
    --card: #ffffff;
    --fg: #111827;
    --muted: #6b7280;
    --border: #d1d5db;
    --accent: #1d4ed8;
    --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }}
  body.dark {{
    --bg: #0f172a;
    --card: #1e293b;
    --fg: #e2e8f0;
    --muted: #94a3b8;
    --border: #334155;
    --accent: #60a5fa;
  }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font-family: "Segoe UI", system-ui, -apple-system, sans-serif; background: var(--bg); color: var(--fg); padding: 24px; }}
  .wrap {{ max-width: 1200px; margin: 0 auto; display: flex; flex-direction: column; gap: 16px; }}
  .card {{ background: var(--card); border: 1px solid var(--border); border-radius: 6px; padding: 16px; }}
  .header {{ display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap; }}
  .badge {{ display: inline-block; padding: 3px 9px; border-radius: 4px; font-size: 12px; font-weight: 700; color: #fff; background: {status_color}; }}
  .mode {{ display: inline-block; padding: 3px 9px; border-radius: 4px; font-size: 12px; font-weight: 600; border: 1px solid var(--border); }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 12px; }}
  .kpi-label {{ font-size: 12px; color: var(--muted); text-transform: uppercase; }}
  .kpi-val {{ font-size: 22px; font-weight: 700; margin-top: 4px; }}
  .cmd {{ font-family: var(--mono); font-size: 12px; background: rgba(127,127,127,0.1); padding: 10px; border-radius: 4px; overflow-x: auto; white-space: pre-wrap; word-break: break-all; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
  th, td {{ border-bottom: 1px solid var(--border); padding: 7px 8px; text-align: left; }}
  th {{ font-weight: 600; color: var(--muted); }}
  td.num, th.num {{ text-align: right; font-variant-numeric: tabular-nums; }}
  .mono {{ font-family: var(--mono); word-break: break-all; }}
  .tag {{ display: inline-block; padding: 2px 6px; border-radius: 3px; font-size: 11px; border: 1px solid var(--border); }}
  .toolbar {{ display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-bottom: 10px; }}
  input[type="text"], button {{ padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; background: var(--card); color: var(--fg); font-size: 13px; }}
  button {{ cursor: pointer; }}
  .muted {{ color: var(--muted); font-size: 13px; }}
  .errors {{ color: #dc2626; font-size: 13px; }}
</style>
</head>
<body>
<div class="wrap">
  <div class="card header">
    <div>
      <h1 style="margin:0 0 6px 0;font-size:20px;">Robocopy2Rust Audit Report &mdash; {title}</h1>
      <div class="muted">Source: <span class="mono">{source}</span> &rarr; Destination: <span class="mono">{dest}</span></div>
      <div class="muted">Started: {started} | Ended: {ended} | Duration: {duration}</div>
    </div>
    <div style="display:flex;gap:8px;align-items:center;">
      <span class="mode">{mode_badge}</span>
      <span class="badge">{status_text} ({exit_code})</span>
      <button onclick="document.body.classList.toggle('dark')">Toggle Theme</button>
    </div>
  </div>

  <div class="grid">
    <div class="card"><div class="kpi-label">Exit Code</div><div class="kpi-val">{exit_label}</div></div>
    <div class="card"><div class="kpi-label">Files Copied / Total</div><div class="kpi-val">{files_copied} / {files_total}</div></div>
    <div class="card"><div class="kpi-label">Bytes Copied</div><div class="kpi-val">{bytes_copied}</div></div>
    <div class="card"><div class="kpi-label">Failed / Extras</div><div class="kpi-val">{failed} / {extras}</div></div>
    <div class="card"><div class="kpi-label">Throughput</div><div class="kpi-val" style="font-size:16px;">{speed}</div></div>
  </div>

  <div class="card">
    <h2 style="margin:0 0 8px 0;font-size:15px;">Command Line &amp; Exit Bitmask</h2>
    <div class="cmd" id="cmdline">{cmd}</div>
    <ul style="margin:10px 0 0 18px;font-size:13px;">{bit_items}</ul>
  </div>

  <div class="card">
    <h2 style="margin:0 0 8px 0;font-size:15px;">Robocopy Summary Matrix</h2>
    <table>
      <thead><tr><th>Category</th><th class="num">Total</th><th class="num">Copied</th><th class="num">Skipped</th><th class="num">Mismatch</th><th class="num">FAILED</th><th class="num">Extras</th></tr></thead>
      <tbody>
        <tr><td>Dirs</td><td class="num">{d_tot}</td><td class="num">{d_cop}</td><td class="num">{d_skp}</td><td class="num">{d_mis}</td><td class="num">{d_fal}</td><td class="num">{d_ext}</td></tr>
        <tr><td>Files</td><td class="num">{f_tot}</td><td class="num">{f_cop}</td><td class="num">{f_skp}</td><td class="num">{f_mis}</td><td class="num">{f_fal}</td><td class="num">{f_ext}</td></tr>
        <tr><td>Bytes</td><td class="num">{b_tot}</td><td class="num">{b_cop}</td><td class="num">{b_skp}</td><td class="num">{b_mis}</td><td class="num">{b_fal}</td><td class="num">{b_ext}</td></tr>
      </tbody>
    </table>
  </div>

  <div class="card">
    <h2 style="margin:0 0 8px 0;font-size:15px;">Errors ({error_count})</h2>
    {error_section}
  </div>

  <div class="card">
    <h2 style="margin:0 0 8px 0;font-size:15px;">File &amp; Directory Actions ({action_count})</h2>
    <div class="toolbar">
      <input type="text" id="filterBox" placeholder="Filter by class or path..." oninput="filterRows()" style="min-width:260px;">
    </div>
    <table>
      <thead><tr><th>Class</th><th class="num">Size</th><th>Timestamp</th><th>Path</th></tr></thead>
      <tbody id="actionBody">{action_rows}</tbody>
    </table>
  </div>

  <div class="card muted">
    Air-gapped pure-Rust report generated by <strong>robocopy2rust</strong> &bull; Log SHA-256: <span class="mono">{sha256}</span>
  </div>
</div>
<script>
function filterRows() {{
  var q = document.getElementById('filterBox').value.toLowerCase();
  var rows = document.getElementById('actionBody').getElementsByTagName('tr');
  for (var i = 0; i < rows.length; i++) {{
    var txt = rows[i].innerText.toLowerCase();
    rows[i].style.display = txt.indexOf(q) !== -1 ? '' : 'none';
  }}
}}
</script>
</body>
</html>
"#,
        title = esc(&result.job_name),
        source = esc(&result.source),
        dest = esc(&result.destination),
        started = esc(&result.started_at),
        ended = esc(&result.ended_at),
        duration = esc(&format_duration(result.duration_secs)),
        mode_badge = esc(mode_badge),
        status_text = status_badge.0,
        status_color = status_badge.1,
        exit_code = result.exit_code,
        exit_label = esc(&result.exit_info.short_label),
        files_copied = result.summary.files.copied,
        files_total = result.summary.files.total,
        bytes_copied = esc(&format_bytes(result.summary.bytes.copied)),
        failed = result.summary.files.failed,
        extras = result.summary.files.extras,
        speed = esc(&speed_str),
        cmd = esc(&result.command_line),
        bit_items = bit_items,
        d_tot = result.summary.dirs.total,
        d_cop = result.summary.dirs.copied,
        d_skp = result.summary.dirs.skipped,
        d_mis = result.summary.dirs.mismatch,
        d_fal = result.summary.dirs.failed,
        d_ext = result.summary.dirs.extras,
        f_tot = result.summary.files.total,
        f_cop = result.summary.files.copied,
        f_skp = result.summary.files.skipped,
        f_mis = result.summary.files.mismatch,
        f_fal = result.summary.files.failed,
        f_ext = result.summary.files.extras,
        b_tot = esc(&format_bytes(result.summary.bytes.total)),
        b_cop = esc(&format_bytes(result.summary.bytes.copied)),
        b_skp = esc(&format_bytes(result.summary.bytes.skipped)),
        b_mis = esc(&format_bytes(result.summary.bytes.mismatch)),
        b_fal = esc(&format_bytes(result.summary.bytes.failed)),
        b_ext = esc(&format_bytes(result.summary.bytes.extras)),
        error_count = result.errors.len(),
        error_section = error_section,
        action_count = result.file_actions.len(),
        action_rows = action_rows,
        sha256 = esc(&result.log_sha256),
    )
}
