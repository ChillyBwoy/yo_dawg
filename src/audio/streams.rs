use super::renderer::Renderer;
use anyhow::Result;
use cpal::traits::DeviceTrait;
use crossbeam_channel::Sender;
use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

pub(super) fn build_input<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    queue: Arc<ArrayQueue<[f32; 2]>>,
    errors: Sender<String>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            for frame in data.chunks_exact(channels) {
                let l = frame[0].to_sample::<f32>();
                let r = frame[if channels > 1 { 1 } else { 0 }].to_sample::<f32>();
                if queue.push([l, r]).is_err() {
                    // Bound latency if input/output clocks drift or the output stalls.
                    let _ = queue.pop();
                    let _ = queue.push([l, r]);
                }
            }
        },
        move |e| {
            let _ = errors.try_send(format!("Audio input: {e}"));
        },
        None,
    )?)
}

pub(super) fn build_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut renderer: Renderer,
    errors: Sender<String>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    Ok(device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for frame in data.chunks_mut(channels) {
                let [l, r] = renderer.next();
                for (c, sample) in frame.iter_mut().enumerate() {
                    let value = if channels == 1 {
                        (l + r) * 0.5
                    } else if c == 0 {
                        l
                    } else if c == 1 {
                        r
                    } else {
                        0.0
                    };
                    *sample = T::from_sample(value.clamp(-1.0, 1.0));
                }
            }
        },
        move |e| {
            let _ = errors.try_send(format!("Audio output: {e}"));
        },
        None,
    )?)
}
