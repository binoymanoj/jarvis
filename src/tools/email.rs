use crate::core::error::{JarvisError, Result};
use crate::tools::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::{debug, warn};

pub struct EmailManager {
    thunderbird_bin: Option<PathBuf>,
    browser_bin: PathBuf,
    xdg_open_bin: PathBuf,
}

impl Default for EmailManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailManager {
    pub fn new() -> Self {
        let thunderbird_bin = which::which("thunderbird").ok();
        let browser_bin = which::which("omarchy-launch-browser")
            .or_else(|_| which::which("xdg-open"))
            .unwrap_or_else(|_| PathBuf::from("xdg-open"));
        let xdg_open_bin = which::which("xdg-open").unwrap_or_else(|_| PathBuf::from("xdg-open"));

        Self {
            thunderbird_bin,
            browser_bin,
            xdg_open_bin,
        }
    }

    pub async fn draft_email(
        &self,
        recipient: &str,
        subject: &str,
        body: &str,
        client: &str,
    ) -> Result<String> {
        let clean_to = recipient.trim();
        let clean_su = subject.trim();
        let clean_body = body.trim();

        debug!("Drafting email to '{clean_to}' with subject '{clean_su}'");

        // Option 1: Gmail webmail
        if client.eq_ignore_ascii_case("gmail") {
            let enc_to = urlencoding::encode(clean_to);
            let enc_su = urlencoding::encode(clean_su);
            let enc_body = urlencoding::encode(clean_body);
            let gmail_url = format!(
                "https://mail.google.com/mail/?view=cm&fs=1&to={enc_to}&su={enc_su}&body={enc_body}"
            );

            let _ = Command::new(&self.browser_bin)
                .arg(&gmail_url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            return Ok(format!(
                "Drafted email to {clean_to} and opened Gmail compose window for review, sir."
            ));
        }

        // Option 2: Native Thunderbird CLI
        if let Some(ref tb) = self.thunderbird_bin {
            if client.eq_ignore_ascii_case("auto") || client.eq_ignore_ascii_case("thunderbird") {
                let compose_args =
                    format!("to='{clean_to}',subject='{clean_su}',body='{clean_body}'");
                let status = Command::new(tb)
                    .arg("-compose")
                    .arg(&compose_args)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;

                if let Ok(s) = status {
                    if s.success() {
                        return Ok(format!(
                            "Drafted email to {clean_to} with subject '{clean_su}'. Thunderbird compose window is open for your review, sir."
                        ));
                    }
                }
                warn!("Thunderbird compose failed, falling back to mailto link");
            }
        }

        // Option 3: mailto URL via xdg-open
        let enc_to = urlencoding::encode(clean_to);
        let enc_su = urlencoding::encode(clean_su);
        let enc_body = urlencoding::encode(clean_body);
        let mailto_url = format!("mailto:{enc_to}?subject={enc_su}&body={enc_body}");

        let _ = Command::new(&self.xdg_open_bin)
            .arg(&mailto_url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        Ok(format!(
            "Drafted email to {clean_to} and opened your default email client for review, sir."
        ))
    }
}

// -----------------------------------------------------------------------------
// Tool Implementations
// -----------------------------------------------------------------------------

pub struct DraftEmailTool {
    email: Arc<EmailManager>,
}

impl DraftEmailTool {
    pub fn new(email: Arc<EmailManager>) -> Self {
        Self { email }
    }
}

#[async_trait]
impl Tool for DraftEmailTool {
    fn name(&self) -> &'static str {
        "draft_email"
    }

    fn description(&self) -> &'static str {
        "Draft an email and open the compose window in Thunderbird or Gmail with prefilled subject and body for user review."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "OBJECT",
            "properties": {
                "recipient": {
                    "type": "STRING",
                    "description": "Recipient email address or contact name."
                },
                "subject": {
                    "type": "STRING",
                    "description": "Subject line of the email."
                },
                "body": {
                    "type": "STRING",
                    "description": "Body content of the email."
                },
                "client": {
                    "type": "STRING",
                    "description": "Email client to target: 'auto', 'thunderbird', or 'gmail'. Default is 'auto'."
                }
            },
            "required": ["recipient", "subject", "body"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let recipient = args["recipient"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("draft_email".into(), "recipient string is required".into())
        })?;
        let subject = args["subject"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("draft_email".into(), "subject string is required".into())
        })?;
        let body = args["body"].as_str().ok_or_else(|| {
            JarvisError::ToolParameter("draft_email".into(), "body string is required".into())
        })?;
        let client = args["client"].as_str().unwrap_or("auto");

        self.email
            .draft_email(recipient, subject, body, client)
            .await
    }
}
