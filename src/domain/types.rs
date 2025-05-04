use interp::{interp_slice, InterpMode};

pub struct MicCalibrationData {
   pub sensitivity: f32,
   pub frequency: Vec<f32>,
   pub response: Vec<f32>,
}

impl MicCalibrationData {
    pub fn interpolate(&self, target_freqs: &[f32]) -> Vec<f32> {
        interp_slice(
            &self.frequency,
            &self.response,
            target_freqs,
            &InterpMode::Extrapolate,
        )
    }
}

#[derive(Clone)]
pub struct Weightings {
    pub a_weighting: Vec<f32>,
    pub c_weighting: Vec<f32>,
    pub cal_weighting:  Vec<f32>,
}
