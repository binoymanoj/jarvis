use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, warn};

pub struct ShellExecutor {
    default_timeout: Duration,
    max_output_chars: usize,
}

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new(15.0, 2000)
    }
}

impl ShellExecutor {
    pub fn new(default_timeout_secs: f64, max_output_chars: usize) -> Self {
        Self {
            default_timeout: Duration::from_secs_f64(default_timeout_secs),
            max_output_chars,
        }
    }

    pub async fn execute_command(
        &self,
        command: &str,
        custom_timeout: Option<f64>,
    ) -> Result<String> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Ok("No command provided.".to_string());
        }

        let run_timeout = custom_timeout
            .map(Duration::from_secs_f64)
            .unwrap_or(self.default_timeout);

        debug!(
            "Executing shell command (timeout={:?}): {}",
            run_timeout, trimmed
        );

        let mut child = Command::new("/bin/bash")
            .arg("-c")
            .arg(trimmed)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let mut stdout_pipe = child.stdout.take().unwrap();
        let mut stderr_pipe = child.stderr.take().unwrap();

        let read_future = async {
            let mut stdout_buf = Vec::new();
            let mut stderr_buf = Vec::new();
            let (_, _, status) = tokio::join!(
                tokio::io::AsyncReadExt::read_to_end(&mut stdout_pipe, &mut stdout_buf),
                tokio::io::AsyncReadExt::read_to_end(&mut stderr_pipe, &mut stderr_buf),
                child.wait()
            );
            (stdout_buf, stderr_buf, status)
        };

        let result = timeout(run_timeout, read_future).await;

        match result {
            Ok((stdout_bytes, stderr_bytes, Ok(status))) => {
                let stdout = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
                let stderr = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
                let code = status.code().unwrap_or(-1);

                let mut parts = Vec::new();
                if !stdout.is_empty() {
                    parts.push(stdout);
                }
                if !stderr.is_empty() {
                    parts.push(format!("[stderr]:\n{stderr}"));
                }

                let combined = parts.join("\n").trim().to_string();

                if combined.is_empty() {
                    if status.success() {
                        return Ok("Command executed successfully (no output).".to_string());
                    } else {
                        return Ok(format!(
                            "Command exited with return code {code} (no output)."
                        ));
                    }
                }

                if combined.len() > self.max_output_chars {
                    let half = self.max_output_chars / 2;
                    let truncated = format!(
                        "{}\n\n... [output truncated: {} characters total] ...\n\n{}",
                        &combined[..half],
                        combined.len(),
                        &combined[combined.len() - half..]
                    );
                    return Ok(truncated);
                }

                Ok(combined)
            }
            Ok((_, _, Err(e))) => Err(JarvisError::Io(e)),
            Err(_) => {
                let _ = child.kill().await;
                warn!("Shell command timed out: {trimmed}");
                Ok(format!(
                    "Command timed out after {} seconds.",
                    run_timeout.as_secs()
                ))
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct ExecuteCommandTool {
    executor: Arc<ShellExecutor>,
}

impl ExecuteCommandTool {
    pub fn new(executor: Arc<ShellExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl Tool for ExecuteCommandTool {
    fn name(&self) -> &'static str {
        "execute_command"
    }

    fn description(&self) -> &'static str {
        "Execute any bash command directly on the Linux system and return its terminal output."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "command": {
                    "type": "STRING",
                    "description": "The exact bash command line to run."
                },
                "timeout": {
                    "type": "NUMBER",
                    "description": "Optional timeout in seconds before aborting (default: 15.0)."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let command = args["command"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter(
                "execute_command".into(),
                "command string is required".into(),
            )
        })?;
        let timeout_secs = args["timeout"].as_f64();

        self.executor.execute_command(command, timeout_secs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_executor_echo() {
        let executor = ShellExecutor::default();
        let out = executor
            .execute_command("echo 'jarvis test'", None)
            .await
            .unwrap();
        assert_eq!(out, "jarvis test");
    }

    #[tokio::test]
    async fn test_shell_executor_timeout() {
        let executor = ShellExecutor::new(0.5, 500);
        let out = executor.execute_command("sleep 2", None).await.unwrap();
        assert!(out.contains("timed out"));
    }
}
