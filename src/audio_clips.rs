use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Raw mono audio clips
use color_eyre::eyre::{Result, eyre};
use cpal::{ChannelCount, FromSample, Sample};

pub struct AudioClip {
    samples: Vec<f32>,
    sample_rate: u32 // 48khz and
}

impl AudioClip {
    pub fn record() -> Result<AudioClip>{
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(eyre!("No input device available"))?;
        println!("Default input device: {:?}", device.name());
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0 as u32;
        let samples = Vec::new();
        let audio_clip = AudioClip {
            samples,
            sample_rate
        };
        println!("Beginning recording");
        let clip = Arc::new(Mutex::new(Some(audio_clip)));
        let clip2 = clip.clone();

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        let channels = config.channels();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<f32>(data, channels, &clip2),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data, _: &_|  write_input_data::<i16>(data, channels, &clip2),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data, _: &_|  write_input_data::<u16>(data, channels, &clip2),
                err_fn,
                None,
            )?,
            _ => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<f32>(data, channels, &clip2),
                err_fn,
                None,
            )?,
        };

        stream.play()?;
        // let recording go for roughly three seconds
        std::thread::sleep(std::time::Duration::from_secs(3));
        drop(stream);
        let clip = clip.lock().unwrap().take().unwrap();
        println!("Finished recording");
        println!("Recording length: {} seconds", clip.samples.len() as f32 / clip.sample_rate as f32);
        Ok(clip)
    }
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