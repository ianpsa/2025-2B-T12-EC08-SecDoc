use anyhow::{Result, Context, anyhow};
use tracing::{info, warn, error};
use std::io::Cursor;

// Symphonia imports for audio decoding
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;


#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Vec<u8>,
    
    pub sample_rate: u32,
    
    pub channels: u16,
    
    pub bits_per_sample: u16,
}

impl DecodedAudio {

    pub fn duration_secs(&self) -> f64 {
        let total_samples = self.samples.len() / 2 / self.channels as usize;
        total_samples as f64 / self.sample_rate as f64
    }
    
    pub fn size_bytes(&self) -> usize {
        self.samples.len()
    }
}

/// Decode MP3 audio bytes to PCM format
/// 
/// # Arguments
/// * `mp3_bytes` - Raw MP3 file data (complete MP3 file in memory)
/// 
/// # Returns
/// * `Result<DecodedAudio>` - Decoded PCM samples ready for ROS2 publishing
/// 
/// # Example Flow:
/// ```
/// MP3 bytes → Symphonia decoder → PCM samples → ROS2 publisher
/// ```
pub fn decode_mp3_to_pcm(mp3_bytes: &[u8]) -> Result<DecodedAudio> {
    info!("Starting MP3 decode: {} bytes", mp3_bytes.len());

    let cursor = Cursor::new(mp3_bytes.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    
    // Create a format hint to help Symphonia detect MP3
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    
    // Probe the media source to detect format and codec
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .context("Failed to probe MP3 format")?;
    
    // Get the format reader
    let mut format_reader = probed.format;
    
    // Find the first audio track
    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("No audio tracks found in MP3"))?;
    
    // Extract audio parameters
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate
        .ok_or_else(|| anyhow!("Sample rate not found"))?;
    let channels = track.codec_params.channels
        .ok_or_else(|| anyhow!("Channel info not found"))?
        .count() as u16;
    
    info!("📊 Audio info: {}Hz, {} channels", sample_rate, channels);
    
    // Create a decoder for this track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create MP3 decoder")?;
    
    // Prepare buffer to collect all decoded samples
    let mut all_samples: Vec<i16> = Vec::new();
    
    // Decode loop - read packets and decode them one by one
    info!("🔄 Decoding MP3 packets...");
    
    loop {
        let packet = match format_reader.next_packet() {
            Ok(packet) => packet,
            Err(_) => {
                info!("✓ Reached end of MP3 stream");
                break;
            }
        };
        
        // Skip packets from other tracks
        if packet.track_id() != track_id {
            continue;
        }
        
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(e) => {
                warn!("Decode error: {}", e);
                continue;
            }
        };
        
        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;
                
        let mut sample_buf = SampleBuffer::<i16>::new(duration, spec);
        sample_buf.copy_interleaved_ref(decoded);
               
        all_samples.extend_from_slice(sample_buf.samples());
    }
    
    info!("✓ Decoded {} samples total", all_samples.len());
        
    let pcm_bytes = samples_to_bytes(&all_samples);
    
    info!("✓ Converted to {} PCM bytes", pcm_bytes.len());
    info!("🎉 MP3 decode complete!");
    
    Ok(DecodedAudio {
        samples: pcm_bytes,
        sample_rate,
        channels,
        bits_per_sample: 16,
    })
}

/// Helper: Convert i16 samples to bytes (little-endian format)
/// 
/// Each i16 is converted to 2 bytes in little-endian order:
/// - Example: i16 value 1000 → bytes [0xE8, 0x03]
fn samples_to_bytes(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|&sample| sample.to_le_bytes())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samples_to_bytes() {
        let samples = vec![0i16, 1i16, -1i16, 256i16];
        let bytes = samples_to_bytes(&samples);
        
        // Each i16 becomes 2 bytes
        assert_eq!(bytes.len(), samples.len() * 2);
        
        // Test little-endian conversion
        // 0i16 → [0x00, 0x00]
        assert_eq!(&bytes[0..2], &[0x00, 0x00]);
        
        // 1i16 → [0x01, 0x00]
        assert_eq!(&bytes[2..4], &[0x01, 0x00]);
    }
    
    #[test]
    fn test_decoded_audio_info() {
        let test_samples = vec![0u8; 16000 * 2]; // 1 second of 16kHz mono audio
        let audio = DecodedAudio {
            samples: test_samples,
            sample_rate: 16000,
            channels: 1,
            bits_per_sample: 16,
        };
        
        assert_eq!(audio.size_bytes(), 32000);
        assert!((audio.duration_secs() - 1.0).abs() < 0.01);
    }
}
