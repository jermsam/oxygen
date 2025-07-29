use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Raw mono audio clips
use color_eyre::eyre::{Result, eyre};
use cpal::{ChannelCount, FromSample, Sample, StreamConfig, Device, SampleFormat, Stream};

#[derive(Debug, Clone)]
pub struct AudioClip {
    samples: Vec<f32>,
    sample_rate: u32, // 48khz and
    playback_position: usize, // Track playback position
}

impl AudioClip {
    pub fn record() -> Result<AudioClip> {
        // Setup input device
        let (device, config) = setup_audio_device(true)?;
        let sample_rate = config.sample_rate().0 as u32;
        
        let samples = Vec::new();
        let audio_clip = AudioClip {
            samples,
            sample_rate,
            playback_position: 0,
        };
        println!("Beginning recording");
        let clip = Arc::new(Mutex::new(Some(audio_clip)));
        let clip2 = clip.clone();

        let channels = config.channels();
        let sample_format = config.sample_format();
        let stream_config = config.into();

        // Build input stream
        let stream = build_audio_stream(
            &device, 
            &stream_config, 
            channels, 
            &clip2, 
            true,
            sample_format,
        )?;

        stream.play()?;
        // let recording go for roughly three seconds
        std::thread::sleep(std::time::Duration::from_secs(3));
        drop(stream);
        let clip = clip.lock().unwrap().take().unwrap();
        println!("Finished recording");
        println!("Recording length: {} seconds", clip.samples.len() as f32 / clip.sample_rate as f32);
        Ok(clip)
    }
    
    pub fn play(&self) -> Result<()> {
        println!("Playing audio clip");
        // Setup output device
        let (device, config) = setup_audio_device(false)?;

        println!("Beginning playback");
        let clip = Arc::new(Mutex::new(Some(self.clone())));
        let clip2 = clip.clone();

        let channels = config.channels();
        let sample_format = config.sample_format();
        let stream_config = config.into();

        // Build output stream
        let stream = build_audio_stream(
            &device, 
            &stream_config, 
            channels, 
            &clip2, 
            false,
            sample_format,
        )?;

        stream.play()?;
        
        // Calculate playback duration based on sample count and sample rate
        let playback_duration = std::time::Duration::from_secs_f32(
            self.samples.len() as f32 / self.sample_rate as f32
        );
        
        // Add a small buffer to ensure all audio is played
        let buffer_duration = std::time::Duration::from_millis(500);
        
        println!("Playback duration: {:?}", playback_duration);
        std::thread::sleep(playback_duration + buffer_duration);
        println!("Playback complete");

        Ok(())
    }
}

// Common function to set up audio device
fn setup_audio_device(is_input: bool) -> Result<(Device, cpal::SupportedStreamConfig)> {
    let host = cpal::default_host();
    let device = if is_input {
        host.default_input_device().ok_or(eyre!("No input device available"))?
    } else {
        host.default_output_device().ok_or(eyre!("No output device available"))?
    };
    
    let device_type = if is_input { "input" } else { "output" };
    println!("Default {} device: {:?}", device_type, device.name());
    
    let config = if is_input {
        device.default_input_config()?
    } else {
        device.default_output_config()?
    };
    
    Ok((device, config))
}

// Common error function
fn create_error_fn() -> impl FnMut(cpal::StreamError) + Send + 'static {
    |err| {
        eprintln!("an error occurred on stream: {}", err);
    }
}

// Build audio stream based on sample format
fn build_audio_stream(
    device: &Device,
    config: &StreamConfig,
    channels: ChannelCount,
    clip: &ClipHandle,
    is_input: bool,
    sample_format: SampleFormat,
) -> Result<Stream> {
    let err_fn = create_error_fn();
    let clip = clip.clone();
    
    let stream = match sample_format {
        SampleFormat::F32 => {
            if is_input {
                device.build_input_stream(
                    config,
                    move |data, _: &_| write_input_data::<f32>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            } else {
                device.build_output_stream(
                    config,
                    move |data, _: &_| write_output_data::<f32>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            }
        },
        SampleFormat::I16 => {
            if is_input {
                device.build_input_stream(
                    config,
                    move |data, _: &_| write_input_data::<i16>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            } else {
                device.build_output_stream(
                    config,
                    move |data, _: &_| write_output_data::<i16>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            }
        },
        SampleFormat::U16 => {
            if is_input {
                device.build_input_stream(
                    config,
                    move |data, _: &_| write_input_data::<u16>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            } else {
                device.build_output_stream(
                    config,
                    move |data, _: &_| write_output_data::<u16>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            }
        },
        _ => {
            if is_input {
                device.build_input_stream(
                    config,
                    move |data, _: &_| write_input_data::<f32>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            } else {
                device.build_output_stream(
                    config,
                    move |data, _: &_| write_output_data::<f32>(data, channels, &clip),
                    err_fn,
                    None,
                )?
            }
        },
    };
    
    Ok(stream)
}

type ClipHandle = Arc<Mutex<Option<AudioClip>>>;
fn write_input_data<T>(input: &[T], channels: ChannelCount, writer: &ClipHandle)
where
    T: Sample,
    f32: FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(clip) = guard.as_mut() {
            for frame in input.chunks(channels.into()) {
               clip.samples.push(f32::from_sample(frame[0]));
            }
        }
    }
}

fn write_output_data<T>(output: &mut[T], channels: ChannelCount, writer: &ClipHandle)
where
    T: Sample + FromSample<f32>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(clip) = guard.as_mut() {
            for frame in output.chunks_mut(channels as usize) {
                // Get the next sample from our recording (mono)
                let next_sample = if clip.playback_position < clip.samples.len() {
                    let sample_value = clip.samples[clip.playback_position];
                    clip.playback_position += 1;
                    sample_value
                } else {
                    // If we run out of samples, use silence
                    0.0
                };
                
                // Apply the same mono sample to all channels (typically left and right for stereo)
                for sample in frame.iter_mut() {
                    *sample = T::from_sample(next_sample);
                }
            }
        }
    }
}