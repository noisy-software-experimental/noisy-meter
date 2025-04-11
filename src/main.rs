extern crate anyhow;
extern crate cpal;

use chrono::{Timelike, Utc, DateTime};
use ringbuf::traits::Consumer;
use tokio::time::{interval, sleep, sleep_until, timeout, Duration, Instant, MissedTickBehavior};

mod umik_1;

use cpal::traits::StreamTrait;




#[tokio::main]
async fn main() {

    log4rs::init_file("./log.yml", Default::default()).unwrap();
    log::info!("Noisy Sound Level Meter Connector");

    if let Some((stream, mut consumer)) = umik_1::connect().await {
        log::info!("UMIK-1: connected");


        let now = Utc::now();
        let next_second = now.with_nanosecond(0).unwrap() + chrono::Duration::seconds(1);
        let diff = (next_second - now).num_nanoseconds().unwrap() as u64;
        sleep_until(Instant::now() + Duration::from_nanos(diff)).await;

        let mut leq_interval = interval(Duration::from_secs(1));
        leq_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
                leq_interval.tick().await;
                let data: Vec<f32> = consumer.pop_iter().take(48_000).collect();
                //println!("Data: {:?}", data.to_owned());
                println!("Size : {:?}", data.len());
            }

    } else {
        // Failed to connect
        eprintln!("Failed to connect to the audio device.");
    }
}
