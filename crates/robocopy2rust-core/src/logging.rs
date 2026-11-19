use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use tracing_subscriber::EnvFilter;

pub fn init_logging(level: &str, log_file: Option<&Path>) -> Result<()> {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("warn"));

    if let Some(path) = log_file {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let writer = std::sync::Mutex::new(file);
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_writer(move || -> Box<dyn Write> {
                match writer.lock() {
                    Ok(guard) => {
                        if let Ok(cloned) = guard.try_clone() {
                            Box::new(cloned)
                        } else {
                            Box::new(std::io::stderr())
                        }
                    }
                    Err(_) => Box::new(std::io::stderr()),
                }
            })
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .try_init();
    }

    Ok(())
}
