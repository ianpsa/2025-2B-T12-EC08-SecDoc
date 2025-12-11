use ffmpeg_next::{format, media};
use ffmpeg_next::software::resampling;
use std::io::Write;

/// Detect if data is raw PCM or encoded audio
pub fn is_raw_pcm(data: &[u8]) -> bool {
    // Check for common audio file signatures
    // MP3: starts with 0xFF 0xFB or ID3
    // WAV: starts with "RIFF"
    // If it doesn't match these patterns, assume raw PCM
    
    if data.len() < 4 {
        return true; // Too short to be encoded
    }
    
    // Check for MP3 sync word
    if data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
        return false; // MP3
    }
    
    // Check for ID3 tag
    if data.len() >= 3 && &data[0..3] == b"ID3" {
        return false; // MP3 with ID3
    }
    
    // Check for WAV/RIFF
    if &data[0..4] == b"RIFF" {
        return false; // WAV
    }
    
    // Check for OGG
    if &data[0..4] == b"OggS" {
        return false; // OGG
    }
    
    // Assume it's raw PCM
    true
}

/// Process audio data - handles both encoded and raw PCM
pub fn process_audio(audio_data: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if is_raw_pcm(&audio_data) {
        // Already PCM, return as-is
        println!("Detected raw PCM data ({} bytes), passing through", audio_data.len());
        Ok(audio_data)
    } else {
        // Encoded audio, decode it
        println!("Detected encoded audio ({} bytes), decoding...", audio_data.len());
        decode_to_pcm(audio_data)
    }
}

pub fn decode_to_pcm(audio_data: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    ffmpeg_next::init()?;
    
    // Detect file extension based on content
    let extension = if audio_data.len() >= 3 && &audio_data[0..3] == b"ID3" {
        "mp3"
    } else if audio_data.len() >= 4 && &audio_data[0..4] == b"RIFF" {
        "wav"
    } else if audio_data.len() >= 4 && &audio_data[0..4] == b"OggS" {
        "ogg"
    } else if audio_data[0] == 0xFF && (audio_data[1] & 0xE0) == 0xE0 {
        "mp3"
    } else {
        "audio" // Generic
    };
    
    // Write audio data to a temporary file since ffmpeg-next requires a file path
    let temp_path = format!("/tmp/audio_{}.{}", std::process::id(), extension);
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(&audio_data)?;
    drop(file); // Ensure file is closed
    
    // Create input context from file
    let mut input = format::input(&temp_path)?;
    
    // Find audio stream
    let input_stream = input
        .streams()
        .best(media::Type::Audio)
        .ok_or("No audio stream found")?;
    let stream_index = input_stream.index();
    
    // Get decoder
    let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(input_stream.parameters())?;
    let mut decoder = context_decoder.decoder().audio()?;
    
    // Setup resampler to PCM 16-bit
    let mut resampler = resampling::Context::get(
        decoder.format(),
        decoder.channel_layout(),
        decoder.rate(),
        format::Sample::I16(format::sample::Type::Packed),
        decoder.channel_layout(),
        decoder.rate(),
    )?;
    
    let mut pcm_output = Vec::new();
    
    // Decode packets
    for (stream, packet) in input.packets() {
        if stream.index() == stream_index {
            decoder.send_packet(&packet)?;
            let mut decoded_frame = ffmpeg_next::util::frame::Audio::empty();
            
            while decoder.receive_frame(&mut decoded_frame).is_ok() {
                let mut resampled = ffmpeg_next::util::frame::Audio::empty();
                resampler.run(&decoded_frame, &mut resampled)?;
                
                // Convert samples to bytes
                let data = resampled.data(0);
                pcm_output.extend_from_slice(data);
            }
        }
    }
    
    // Flush decoder
    decoder.send_eof()?;
    let mut decoded_frame = ffmpeg_next::util::frame::Audio::empty();
    while decoder.receive_frame(&mut decoded_frame).is_ok() {
        let mut resampled = ffmpeg_next::util::frame::Audio::empty();
        resampler.run(&decoded_frame, &mut resampled)?;
        let data = resampled.data(0);
        pcm_output.extend_from_slice(data);
    }
    
    // Clean up temporary file
    let _ = std::fs::remove_file(&temp_path);
    
    Ok(pcm_output)
}
