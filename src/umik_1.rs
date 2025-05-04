extern crate anyhow;
extern crate cpal;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::error::Error;

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Device, Stream};
use ringbuf::{SharedRb,HeapRb};
use ringbuf::{storage::Heap, traits::{Producer, Split}, wrap::caching::Caching};
use std::sync::Arc;

use crate::domain::types::MicCalibrationData;

pub async fn connect() -> Option<(Stream, Caching<Arc<SharedRb<Heap<f32>>>, false, true>, MicCalibrationData)> {

    let calibration_data = parse_calibration_file("./calibration_data/7161669.txt").unwrap();

    let device = get_device_handle()?;

    let mut config: cpal::StreamConfig = device.default_input_config().ok()?.into();
    config.buffer_size = cpal::BufferSize::Fixed(1024);

    let (producer, consumer) = initialise_ring_buffer(config.to_owned());

    let input_stream = initialise_input_stream(device, config, producer)?;

    if let Err(err) = input_stream.play() {
        log::warn!("UMIK-1: Failed to start the stream: {}", err);
        return None;
    }
    Some((input_stream, consumer, calibration_data))
}



fn get_device_handle() -> Option<Device>{
    let host = match
        cpal::host_from_id(cpal::available_hosts()
                        .into_iter()
                        .find(|id| *id == cpal::HostId::CoreAudio)
                        .expect("Core Audio not available")){

        Ok(host) => host,
        Err(_) => return None,
                        };

    let devices = match host.devices(){
        Ok(devices) => devices,
        Err(_) => return None,
    };

    for device in devices{
        let name = match device.name() {
            Ok(name) => name,
            Err(_) => continue,
        };
        if name  == "UMIK-1" {
            return Some(device)
        } else {
            continue;
        }
    }
    None
}


fn initialise_ring_buffer(config: cpal::StreamConfig) -> (Caching<Arc<SharedRb<Heap<f32>>>, true, false>, Caching<Arc<SharedRb<Heap<f32>>>, false, true>) {
        let latency_frames = (100.0 / 1_000.0) * config.sample_rate.0 as f32;
        let latency_samples = latency_frames as usize * config.channels as usize;

        let ring_buffer_size = config.sample_rate.0 as usize + latency_samples;
        let ring = HeapRb::<f32>::new(ring_buffer_size);
        let (mut producer, consumer) = ring.split();

        for _ in 0..latency_samples { producer.try_push(0.0).unwrap() };

        return (producer, consumer);
}


fn initialise_input_stream(
    device: Device,
    config: cpal::StreamConfig,
    mut producer: Caching<Arc<SharedRb<Heap<f32>>>, true, false>,
) -> Option<Stream> {
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            //println!("{:?}",sample);
            if producer.try_push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            //eprintln!("Output stream fell behind: try increasing latency");
        }
    };

    match device.build_input_stream(&config, input_data_fn, buffer_err_fn, None) {
        Ok(stream) => {
            // Attempt to start the stream
            if let Err(err) = stream.play() {
                 log::warn!("UMIK-1: Failed to start stream: {}", err);
                None
            } else {
                 log::info!("UMIK-1: Connected");
                Some(stream)
            }
        }
        Err(err) => {
            log::warn!("UMIK-1: Failed to build input stream: {}", err);
            None
        }
    }
}

fn buffer_err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}


fn parse_calibration_file<P: AsRef<Path>>(path: P) -> Result<MicCalibrationData, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = reader.lines();
    let first_line = lines.next().ok_or("File is empty")??;
    let sensitivity = extract_sensitivity(&first_line)?;

    let mut frequency = Vec::new();
    let mut response = Vec::new();

    for line in lines {
        let line = line?;
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.len() == 2 {
            let freq: f32 = parts[0].parse()?;
            let resp_db: f32 = parts[1].parse()?;
            let resp_linear = 10f32.powf(resp_db / 20.0);
            frequency.push(freq);
            response.push(resp_linear);
        }
    }

    Ok(MicCalibrationData {
        sensitivity,
        frequency,
        response,
    })
}



fn extract_sensitivity(line: &str) -> Result<f32, Box<dyn Error>> {
    let parts: Vec<&str> = line.split(',').collect();
    for part in parts {
        if part.contains("Sens Factor") {
            let value_part = part.split('=').nth(1).ok_or("Invalid sensitivity format")?;
            let value_str = value_part.trim().trim_end_matches("dB");
            let value: f32 = value_str.parse()?;
            return Ok(value);
        }
    }
    Err("Sensitivity not found".into())
}
