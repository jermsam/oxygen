use color_eyre::eyre::{Result, eyre};
use std::fs::File;
use std::io::{Read, Write, Cursor, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use hound;

// Import the correct vorbis-encoder crate
use vorbis_encoder::Encoder;

pub struct AudioCodec;

impl AudioCodec {
    // Constants for Vorbis encoding
    const SAMPLE_RATE: u32 = 44100; // Standard sample rate for Vorbis
    const CHANNELS: u32 = 1;        // Mono for voice recording
    const QUALITY: f32 = 0.4;       // Good quality for voice (0.0 to 1.0)
    
    /// Encode raw PCM audio samples to Vorbis format and save to a file
    pub fn encode_to_vorbis(samples: &[f32], sample_rate: u32) -> Result<PathBuf> {
        // Resample to 44.1kHz if needed (standard for Vorbis)
        let resampled = if sample_rate != Self::SAMPLE_RATE {
            println!("Resampling from {}Hz to {}Hz for Vorbis encoding", sample_rate, Self::SAMPLE_RATE);
            resample(samples, sample_rate, Self::SAMPLE_RATE)?
        } else {
            samples.to_vec()
        };
        
        // Convert f32 samples to i16 (required by vorbis-encoder)
        let i16_samples: Vec<i16> = resampled.iter()
            .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();
        
        // Create a temporary file for output
        let mut output_file = NamedTempFile::new()?;
        
        // Create a Vorbis encoder with the correct API
        let mut encoder = Encoder::new(
            Self::CHANNELS,
            Self::SAMPLE_RATE as u64,
            Self::QUALITY
        ).map_err(|e| eyre!("Failed to create Vorbis encoder: {}", e))?;
        
        // Encode the audio
        let encoded_data = encoder.encode(&i16_samples)
            .map_err(|e| eyre!("Failed to encode audio: {}", e))?;
        
        // Write the encoded data to the file
        output_file.write_all(&encoded_data)?;
        
        // Flush any remaining data
        let flush_data = encoder.flush()
            .map_err(|e| eyre!("Failed to flush encoder: {}", e))?;
        output_file.write_all(&flush_data)?;
        
        // Finalize the file
        output_file.flush()?;
        let path = output_file.into_temp_path();
        let permanent_path = path.keep()?;
        
        Ok(permanent_path)
    }
    
    /// Decode Vorbis audio from a file to raw PCM samples
    pub fn decode_from_vorbis(file_path: &Path) -> Result<(Vec<f32>, u32)> {
        // For now, we'll use hound to decode the file
        // In a real implementation, you would use a Vorbis decoder like lewton
        
        // Open the file
        let mut file = File::open(file_path)?;
        
        // Read the file content
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        
        // Use WAV decoding as a fallback
        let mut reader = hound::WavReader::new(Cursor::new(content))
            .map_err(|_| eyre!("Failed to decode audio file"))?;
        
        // Get sample rate
        let sample_rate = reader.spec().sample_rate;
        
        // Read samples
        let samples: Vec<f32> = reader.samples::<i16>()
            .map(|s| s.map_err(|e| eyre!("Failed to read sample: {}", e)))
            .map(|s| s.map(|sample| sample as f32 / 32768.0))
            .collect::<Result<Vec<f32>>>()?;
        
        Ok((samples, sample_rate))
    }
    
    /// Encode audio samples to a binary blob for storage in a database
    pub fn encode_to_blob(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
        // Resample to 44.1kHz if needed
        let resampled = if sample_rate != Self::SAMPLE_RATE {
            resample(samples, sample_rate, Self::SAMPLE_RATE)?
        } else {
            samples.to_vec()
        };
        
        // Convert f32 samples to i16 (required by vorbis-encoder)
        let i16_samples: Vec<i16> = resampled.iter()
            .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();
        
        // Create a buffer for the encoded data
        let mut encoded_data = Vec::new();
        
        // Write a simple header with magic bytes, version, sample rate, and channels
        encoded_data.extend_from_slice(b"OXVB"); // Magic bytes: OXygen VorBis
        encoded_data.extend_from_slice(&[1]); // Version
        encoded_data.extend_from_slice(&Self::SAMPLE_RATE.to_le_bytes());
        encoded_data.extend_from_slice(&[Self::CHANNELS as u8]); // Mono
        
        // Create a Vorbis encoder
        let mut encoder = Encoder::new(
            Self::CHANNELS,
            Self::SAMPLE_RATE as u64,
            Self::QUALITY
        ).map_err(|e| eyre!("Failed to create Vorbis encoder: {}", e))?;
        
        // Encode the audio
        let vorbis_data = encoder.encode(&i16_samples)
            .map_err(|e| eyre!("Failed to encode audio: {}", e))?;
        
        // Get flush data
        let flush_data = encoder.flush()
            .map_err(|e| eyre!("Failed to flush encoder: {}", e))?;
        
        // Combine encoded data and flush data
        let mut combined_data = Vec::with_capacity(vorbis_data.len() + flush_data.len());
        combined_data.extend_from_slice(&vorbis_data);
        combined_data.extend_from_slice(&flush_data);
        
        // Write the Vorbis data size and the data itself
        encoded_data.extend_from_slice(&(combined_data.len() as u32).to_le_bytes());
        encoded_data.extend_from_slice(&combined_data);
        
        Ok(encoded_data)
    }
    
    /// Decode audio from a binary blob to samples
    pub fn decode_from_blob(blob: &[u8]) -> Result<(Vec<f32>, u32)> {
        // Check for our magic bytes
        if blob.len() < 10 || &blob[0..4] != b"OXVB" {
            return Err(eyre!("Invalid Vorbis blob format"));
        }
        
        // Read header
        let version = blob[4];
        if version != 1 {
            return Err(eyre!("Unsupported blob version: {}", version));
        }
        
        // Read sample rate
        let mut sample_rate_bytes = [0u8; 4];
        sample_rate_bytes.copy_from_slice(&blob[5..9]);
        let sample_rate = u32::from_le_bytes(sample_rate_bytes);
        
        // Read channels
        let channels = blob[9];
        if channels != 1 {
            return Err(eyre!("Only mono audio is supported"));
        }
        
        // Read Vorbis data size
        let mut size_bytes = [0u8; 4];
        size_bytes.copy_from_slice(&blob[10..14]);
        let vorbis_data_size = u32::from_le_bytes(size_bytes) as usize;
        
        // Check if we have enough data
        if 14 + vorbis_data_size > blob.len() {
            return Err(eyre!("Blob data is truncated"));
        }
        
        // Extract the Vorbis data
        let vorbis_data = &blob[14..14 + vorbis_data_size];
        
        // For now, we'll use a simple approach: save to a temporary file and decode
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(vorbis_data)?;
        temp_file.flush()?;
        
        // Use WAV decoding as a fallback
        let (samples, _) = Self::decode_from_wav(temp_file.path())?;
        
        Ok((samples, sample_rate))
    }
    
    /// For backward compatibility: encode to WAV format
    pub fn encode_to_wav(samples: &[f32], sample_rate: u32) -> Result<PathBuf> {
        // Create a temporary file for the WAV
        let temp_file = NamedTempFile::new()?;
        let file_path = temp_file.path().to_path_buf();
        
        // Create a WAV writer
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        
        let mut writer = hound::WavWriter::create(file_path.clone(), spec)?;
        
        // Write samples
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        
        // Finalize the WAV file
        writer.finalize()?;
        
        // Keep the temporary file
        let permanent_path = temp_file.into_temp_path().keep()?;
        
        Ok(permanent_path)
    }
    
    /// For backward compatibility: decode from WAV format
    pub fn decode_from_wav(file_path: &Path) -> Result<(Vec<f32>, u32)> {
        // Open the WAV file
        let mut reader = hound::WavReader::open(file_path)?;
        
        // Get the spec
        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        
        // Read samples
        let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
            reader.samples::<f32>().collect::<std::result::Result<Vec<f32>, _>>()?
        } else {
            // Convert integer samples to float
            reader.samples::<i32>()
                .map(|s| s.map(|s| s as f32 / i32::MAX as f32))
                .collect::<std::result::Result<Vec<f32>, _>>()?
        };
        
        Ok((samples, sample_rate))
    }
}

// Helper function to resample audio
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }
    
    let ratio = to_rate as f64 / from_rate as f64;
    let new_len = (samples.len() as f64 * ratio).ceil() as usize;
    let mut resampled = Vec::with_capacity(new_len);
    
    for i in 0..new_len {
        let src_idx = i as f64 / ratio;
        let src_idx_floor = src_idx.floor() as usize;
        let src_idx_ceil = src_idx.ceil() as usize;
        
        if src_idx_ceil >= samples.len() {
            // We've reached the end of the input
            break;
        }
        
        if src_idx_floor == src_idx_ceil {
            // Exact sample
            resampled.push(samples[src_idx_floor]);
        } else {
            // Linear interpolation
            let t = src_idx - src_idx_floor as f64;
            let sample = samples[src_idx_floor] * (1.0 - t as f32) + samples[src_idx_ceil] * t as f32;
            resampled.push(sample);
        }
    }
    
    Ok(resampled)
}
