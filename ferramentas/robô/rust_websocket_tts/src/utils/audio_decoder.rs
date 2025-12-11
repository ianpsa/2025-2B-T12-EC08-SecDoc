use ffmpeg_next::{format, codec, media};
use ffmpeg_next::software::resampling;

pub fn decode_mp3_to_pcm(mp3_data: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    ffmpeg_next::init()?;
    
    // Create input context from memory buffer
    let mut input = format::input(&std::io::Cursor::new(mp3_data))?;
    
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
    
    Ok(pcm_output)
}
