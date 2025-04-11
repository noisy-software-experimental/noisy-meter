extern crate anyhow;
extern crate cpal;

use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, Device, Stream};
use ringbuf::{SharedRb,HeapRb};
use ringbuf::{storage::Heap, traits::{Producer, Split}, wrap::caching::Caching};
use std::sync::Arc;



pub async fn connect() -> Option<(Stream, Caching<Arc<SharedRb<Heap<f32>>>, false, true>)> {
    // Get the default host

    // Retrieve the audio input device handle
    let device = get_device_handle()?;

    // Configure the device
    let mut config: cpal::StreamConfig = device.default_input_config().ok()?.into();
    config.buffer_size = cpal::BufferSize::Fixed(15);

    // Initialize the ring buffer
    let (producer, consumer) = initialise_ring_buffer(config.to_owned());

    // Initialize the input stream
    let input_stream = initialise_input_stream(device, config, producer)?;

    // Start the input stream
    if let Err(err) = input_stream.play() {
        log::warn!("UMIK-1: Failed to start the stream: {}", err);
        return None;
    }

    log::info!("UMIK-1: Connected and stream started successfully.");

    // Return the stream and consumer
    Some((input_stream, consumer))
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
    // Create a delay to ensure full 1s data arrives before next processing step
        let latency_frames = (10.00 / 1_000.0) * config.sample_rate.0 as f32;
        let latency_samples = latency_frames as usize * config.channels as usize;

    // The buffer to share samples
        let ring_buffer_size = config.sample_rate.0 as usize + latency_samples;
        let ring = HeapRb::<f32>::new(ring_buffer_size);
        let (mut producer, mut consumer) = ring.split();

    // Fill the samples with 0.0 equal to the length of the delay.
        for _ in 0..latency_samples { producer.try_push(0.0).unwrap() };

        return (producer, consumer);
}


fn initialise_input_stream(
    device: Device,
    config: cpal::StreamConfig,
    mut producer: Caching<Arc<SharedRb<Heap<f32>>>, true, false>,
) -> Option<Stream> {
    // Define the data callback function
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.try_push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("Output stream fell behind: try increasing latency");
        }
    };

    // Attempt to build the input stream
    match device.build_input_stream(&config, input_data_fn, buffer_err_fn, None) {
        Ok(stream) => {
            // Attempt to start the stream
            if let Err(err) = stream.play() {
                 log::warn!("UMIK-1: Failed to start stream: {}", err);
                None
            } else {
                 log::warn!("UMIK-1: Connected and stream started successfully.");
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
