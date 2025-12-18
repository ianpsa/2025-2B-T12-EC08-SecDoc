use base64::Engine;
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

/// Number of parallel FFmpeg workers
const WORKER_COUNT: usize = 4;

/// Decoded audio chunk with sequence number for ordering
pub struct DecodedChunk {
    pub seq: u64,
    pub path: PathBuf,
}

/// High-performance audio processor with worker pool
pub struct AudioProcessor {
    temp_dir: PathBuf,
    counter: AtomicU64,
    work_tx: Sender<WorkItem>,
    result_rx: Receiver<DecodedChunk>,
    // Reorder buffer to ensure sequential playback
    reorder_buffer: Arc<Mutex<BTreeMap<u64, PathBuf>>>,
    next_seq: Arc<AtomicU64>,
}

struct WorkItem {
    seq: u64,
    audio_bytes: Vec<u8>,
    format: String,
    output_path: PathBuf,
}

impl AudioProcessor {
    pub fn new() -> std::io::Result<Self> {
        let temp = tempfile::tempdir()?;
        let temp_dir = temp.path().to_path_buf();
        std::mem::forget(temp); // Keep temp dir alive

        let (work_tx, work_rx) = bounded::<WorkItem>(64);
        let (result_tx, result_rx) = bounded::<DecodedChunk>(64);

        // Spawn worker pool - native threads for true parallelism
        for _ in 0..WORKER_COUNT {
            let rx = work_rx.clone();
            let tx = result_tx.clone();
            
            thread::spawn(move || {
                while let Ok(item) = rx.recv() {
                    if let Some(path) = decode_sync(&item) {
                        let _ = tx.send(DecodedChunk { seq: item.seq, path });
                    }
                }
            });
        }

        Ok(Self {
            temp_dir,
            counter: AtomicU64::new(0),
            work_tx,
            result_rx,
            reorder_buffer: Arc::new(Mutex::new(BTreeMap::new())),
            next_seq: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Submit audio for async decoding - returns immediately
    pub fn submit(&self, audio_b64: &str, format: &str) -> Option<u64> {
        let seq = self.counter.fetch_add(1, Ordering::Relaxed);
        let audio_bytes = base64::engine::general_purpose::STANDARD
            .decode(audio_b64)
            .ok()?;

        let output_path = self.temp_dir.join(format!("o{}.wav", seq));

        self.work_tx
            .send(WorkItem {
                seq,
                audio_bytes,
                format: format.to_string(),
                output_path,
            })
            .ok()?;

        Some(seq)
    }

    /// Get next decoded chunk in sequence order (non-blocking)
    pub fn try_recv_ordered(&self) -> Option<PathBuf> {
        // First, collect any new results into reorder buffer
        while let Ok(chunk) = self.result_rx.try_recv() {
            self.reorder_buffer.lock().insert(chunk.seq, chunk.path);
        }

        // Check if next expected sequence is ready
        let expected = self.next_seq.load(Ordering::Relaxed);
        let mut buffer = self.reorder_buffer.lock();
        
        if let Some(path) = buffer.remove(&expected) {
            self.next_seq.fetch_add(1, Ordering::Relaxed);
            Some(path)
        } else {
            None
        }
    }

    /// Blocking wait for next chunk in order
    pub fn recv_ordered(&self) -> Option<PathBuf> {
        loop {
            // Try non-blocking first
            if let Some(path) = self.try_recv_ordered() {
                return Some(path);
            }

            // Block on channel for new result
            match self.result_rx.recv() {
                Ok(chunk) => {
                    self.reorder_buffer.lock().insert(chunk.seq, chunk.path);
                }
                Err(_) => return None, // Channel closed
            }
        }
    }

    /// Get number of pending items
    pub fn pending_count(&self) -> usize {
        self.reorder_buffer.lock().len()
    }

    pub fn cleanup(&self, path: &PathBuf) {
        let _ = std::fs::remove_file(path);
    }

    pub fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }
}

/// Synchronous FFmpeg decode - runs in worker thread
fn decode_sync(item: &WorkItem) -> Option<PathBuf> {
    let input_path = item.output_path.with_extension(&item.format);
    
    // Write input file
    std::fs::write(&input_path, &item.audio_bytes).ok()?;

    // Fast FFmpeg conversion with optimized flags
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel", "error",
            "-threads", "1",          // Single thread per worker (workers are parallel)
            "-i", input_path.to_str()?,
            "-ar", "16000",           // 16kHz for robot
            "-ac", "1",               // Mono
            "-acodec", "pcm_s16le",   // Raw PCM - fastest
            "-f", "wav",
            item.output_path.to_str()?,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;

    // Cleanup input immediately
    let _ = std::fs::remove_file(&input_path);

    if status.success() && item.output_path.exists() {
        Some(item.output_path.clone())
    } else {
        None
    }
}

impl Drop for AudioProcessor {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }
}
