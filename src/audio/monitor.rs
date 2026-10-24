use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

/// Basic stereo input resampling; deliberately independent of synthesis.
pub(super) struct Monitor {
    input: Arc<ArrayQueue<[f32; 2]>>,
    ratio: f64,
    fraction: f64,
    previous: [f32; 2],
    following: [f32; 2],
}

impl Monitor {
    pub fn new(input: Arc<ArrayQueue<[f32; 2]>>, input_rate: u32, output_rate: u32) -> Self {
        Self {
            input,
            ratio: input_rate as f64 / output_rate as f64,
            fraction: 0.0,
            previous: [0.0; 2],
            following: [0.0; 2],
        }
    }

    pub fn next(&mut self) -> [f32; 2] {
        self.fraction += self.ratio;
        while self.fraction >= 1.0 {
            self.previous = self.following;
            self.following = self.input.pop().unwrap_or([0.0; 2]);
            self.fraction -= 1.0;
        }
        std::array::from_fn(|i| {
            self.previous[i] + (self.following[i] - self.previous[i]) * self.fraction as f32
        })
    }
}
