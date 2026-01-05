use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use sha2::Sha256;
use tokio::sync::mpsc::UnboundedSender;

use crate::core::AppState;

const CHANNEL_BOUND: usize = 4; // Buffer up to 16MB in memory

#[derive(Debug, Clone, PartialEq)]
pub struct FlashProgress {
    pub bytes_written: u64,
    pub total_bytes: u64,
    pub speed_mbps: f64,
    pub percent: f64,
}

pub struct Flasher {
    client: Client,
}

impl Flasher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn flash(
        &self,
        url: String,
        device_path: String,
        progress_tx: UnboundedSender<AppState>,
    ) -> Result<()> {
        // 1. Pre-flight check
        let head_resp = self.client.head(&url).send().await?;
        if !head_resp.status().is_success() {
            return Err(anyhow!("Failed to access URL: {}", head_resp.status()));
        }

        let total_size = head_resp
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| anyhow!("Could not retrieve content length from URL"))?;

        // 2. Open device
        #[cfg(unix)]
        let mut file = OpenOptions::new()
            .write(true)
            .read(false)
            .open(&device_path)
            .context(format!("Failed to open device {}", device_path))?;
        
        // TODO: Windows implementation

        // 3. Setup Producer-Consumer channels
        // We use a sync channel for backpressure handling
        let (data_tx, data_rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = sync_channel(CHANNEL_BOUND);
        
        // 4. Spawn Consumer (Writer Thread)
        // We use a dedicated thread for blocking IO to avoid blocking the async runtime
        let writer_path = device_path.clone();
        let writer_handle = thread::spawn(move || -> Result<()> {
            let mut written = 0u64;
            let start = Instant::now();
            let mut last_progress_update = Instant::now();

            for chunk in data_rx {
                file.write_all(&chunk)
                    .context("Failed to write to device")?;
                
                written += chunk.len() as u64;

                // Sync periodically or handle in producer? 
                // Actually, the main app state update should probably happen here or be managed by the producer if we want to bubble up progress.
                // But since this is a blocking thread, we should probably just do the writing.
                // Wait, we need to send progress updates back to the UI.
                
                // Since this runs in a separate thread, we can't easily use the UnboundedSender from tokio directly without it being thread-safe (which it is).
                // But we need to calculate speed etc.
            }

            // Sync disk
            file.sync_all().context("Failed to sync device")?;
            
            Ok(())
        });

        // 5. Producer (Downloader)
        let mut stream = self.client.get(&url).send().await?.bytes_stream();
        
        let start_time = Instant::now();
        let mut bytes_processed = 0u64;
        let mut last_update_time = Instant::now();

        while let Some(item) = stream.next().await {
            let chunk = item.context("Error downloading chunk")?;
            let chunk_len = chunk.len();
            
            // Send to writer (blocking if full)
            // convert Bytes to Vec<u8>
            data_tx.send(chunk.to_vec()).map_err(|_| anyhow!("Writer thread dropped"))?;

            bytes_processed += chunk_len as u64;
            
            // Update Progress
            let now = Instant::now();
            if now.duration_since(last_update_time).as_millis() > 100 {
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                let speed_mbps = (bytes_processed as f64 / 1_000_000.0) / elapsed_secs;
                let percent = (bytes_processed as f64 / total_size as f64) * 100.0;

                let progress = FlashProgress {
                    bytes_written: bytes_processed,
                    total_bytes: total_size,
                    speed_mbps,
                    percent,
                };

                // Ignore send errors (e.g. if app closed)
                let _ = progress_tx.send(AppState::Flashing(progress));
                last_update_time = now;
            }
        }
        
        // Drop tx to signal EOF to writer
        drop(data_tx);

        // Wait for writer to finish
        match writer_handle.join() {
            Ok(result) => result?,
            Err(e) => return Err(anyhow!("Writer thread panicked: {:?}", e)),
        }

        Ok(())
    }
}
