use crate::core::error::Result;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::{channel, Receiver};
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonSignal {
    Shutdown,    // SIGINT or SIGTERM
    ForceSubmit, // SIGUSR1: Force finalize current voice recording
    TogglePtt,   // SIGUSR2: Push-to-talk toggle / activate wake
    Interrupt,   // SIGHUP: Interrupt speech / cancel current action
}

pub struct SignalHandler {
    rx: Receiver<DaemonSignal>,
}

impl SignalHandler {
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel::<DaemonSignal>(16);

        // SIGINT
        let tx_int = tx.clone();
        tokio::spawn(async move {
            if let Ok(mut sig) = signal(SignalKind::interrupt()) {
                while sig.recv().await.is_some() {
                    info!("Received SIGINT (Shutdown signal)");
                    let _ = tx_int.send(DaemonSignal::Shutdown).await;
                }
            }
        });

        // SIGTERM
        let tx_term = tx.clone();
        tokio::spawn(async move {
            if let Ok(mut sig) = signal(SignalKind::terminate()) {
                while sig.recv().await.is_some() {
                    info!("Received SIGTERM (Shutdown signal)");
                    let _ = tx_term.send(DaemonSignal::Shutdown).await;
                }
            }
        });

        // SIGUSR1 (Force submit voice recording)
        let tx_usr1 = tx.clone();
        tokio::spawn(async move {
            if let Ok(mut sig) = signal(SignalKind::user_defined1()) {
                while sig.recv().await.is_some() {
                    debug!("Received SIGUSR1 (Force submit recording)");
                    let _ = tx_usr1.send(DaemonSignal::ForceSubmit).await;
                }
            }
        });

        // SIGUSR2 (PTT Toggle)
        let tx_usr2 = tx.clone();
        tokio::spawn(async move {
            if let Ok(mut sig) = signal(SignalKind::user_defined2()) {
                while sig.recv().await.is_some() {
                    debug!("Received SIGUSR2 (PTT toggle)");
                    let _ = tx_usr2.send(DaemonSignal::TogglePtt).await;
                }
            }
        });

        // SIGHUP (Emergency interrupt)
        let tx_hup = tx;
        tokio::spawn(async move {
            if let Ok(mut sig) = signal(SignalKind::hangup()) {
                while sig.recv().await.is_some() {
                    info!("Received SIGHUP (Interrupt / kill switch)");
                    let _ = tx_hup.send(DaemonSignal::Interrupt).await;
                }
            }
        });

        Ok(Self { rx })
    }

    /// Wait for the next incoming daemon signal
    pub async fn recv(&mut self) -> Option<DaemonSignal> {
        self.rx.recv().await
    }
}
