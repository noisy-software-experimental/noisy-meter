
use num::complex::Complex;
use rustfft::FftPlanner;
use spectrum_analyzer::windows::hamming_window;

use crate::domain::types::{MicCalibrationData, Weightings};

pub fn process_raw_data(mut raw_time_domain_data: Vec<f32>, weightings: Weightings) -> Result<(f32,f32,f32),()>{
    if raw_time_domain_data.len() != 48_000 {
        println!("To few samples");
        return Err(());
    }

    let raw_freq_domain_data = to_freq_domain(raw_time_domain_data.clone());

    let cal_z_freq_domain_data = apply_weighting(raw_freq_domain_data, &weightings.cal_weighting);

    let cal_a_freq_domain_data = apply_weighting(cal_z_freq_domain_data.clone(), &weightings.a_weighting);
    let cal_c_freq_domain_data = apply_weighting(cal_z_freq_domain_data.clone(), &weightings.c_weighting);


    let cal_z_time_domain_data = to_time_domain(&mut cal_z_freq_domain_data.to_owned());
    let cal_a_time_domain_data = to_time_domain(&mut cal_a_freq_domain_data.to_owned());
    let cal_c_time_domain_data = to_time_domain(&mut cal_c_freq_domain_data.to_owned());

    let sensitivity_db: f32 = -1.359;

    //let spl_z = calculate_leq(&raw_time_domain_data) + sensitivity_db;
    let spl_z = calculate_leq(&cal_z_time_domain_data) + sensitivity_db;
    let spl_a = calculate_leq(&cal_a_time_domain_data);
    let spl_c = calculate_leq(&cal_c_time_domain_data);

    return Ok((spl_z, spl_a, spl_c));

}

fn to_freq_domain(mut raw_time_domain_data: Vec<f32>) -> Vec<Complex<f32>> {
    //apply_hamming_window(&mut raw_time_domain_data);
    let original_len = raw_time_domain_data.len();
    raw_time_domain_data.resize(original_len * 2, 0.0);
    let mut buffer: Vec<Complex<f32>> = raw_time_domain_data
        .iter()
        .map(|&x| Complex::new(x, 0.0))
        .collect();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());
    fft.process(&mut buffer);
    let downsampled_fft:  Vec<Complex<f32>> = buffer.into_iter().take(original_len).collect();
    let modified_fft: Vec<Complex<f32>> = downsampled_fft.into_iter()
            .map(|mut c| {
                c.re *= 1.414; // Multiply real part by sqrt(2)
                c.im *= 1.414; // Multiply imaginary part by sqrt(2)
                c // Return the modified complex number
            })
            .collect();
    modified_fft
}

fn to_time_domain(calibrated_freq_domain_data: &mut Vec<Complex<f32>>) -> Vec<f32> {
    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(calibrated_freq_domain_data.len());
    ifft.process(calibrated_freq_domain_data);
    println!("{:?}", calibrated_freq_domain_data.len());
    calibrated_freq_domain_data.iter().map(|c|  c.re / calibrated_freq_domain_data.len()  as f32 ).collect()


}

fn apply_weighting(
    mut raw_freq_domain_data: Vec<Complex<f32>>,
    weighting: &[f32], // use a reference to avoid unnecessary clone
) -> Vec<Complex<f32>> {
    let fft_size = raw_freq_domain_data.len();
    let half_size = weighting.len(); // expected to be fft_size / 2 + 1
    println!("raw_freq_domain_data length: {:?}" ,raw_freq_domain_data.len());
    println!("weighting: {:?}" , weighting.len());

    assert!(
        half_size <= fft_size,
        "Weighting length exceeds FFT size"
    );

    for (i, &gain) in weighting.iter().enumerate() {
        // Apply to positive frequency bin
        raw_freq_domain_data[i] *= gain;

        // Apply to the mirrored bin (negative frequency), except DC and Nyquist
        if i != 0 && i != fft_size / 2 {
            let mirror_index = fft_size - i;
            if mirror_index < fft_size {
                raw_freq_domain_data[mirror_index] *= gain;
            }
        }
    }

    raw_freq_domain_data
}


fn apply_hamming_window(data: &mut [f32]) {
    hamming_window(data);
}




fn calculate_leq(samples: &[f32]) -> f32 {
    println!("{:?}",samples[1]);
    // Reference pressure in Pascals (typically 20 µPa)
    let p_ref = 20e-6;

    // Calculate the mean square of the samples
    let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
    let mean_square = sum_squares / samples.len() as f32;

    // Calculate the root mean square (RMS) pressure
    let rms_pressure = mean_square.sqrt();

    // Calculate the sound pressure level (SPL) in dB
    let spl = 20.0 * (rms_pressure / p_ref).log10() + 33.6;
    spl
}
