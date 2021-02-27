use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use crate::source::{i16_to_f32, i24_to_f32, i32_to_f32, i8_to_f32};
use crate::Source;

use hound::{SampleFormat, WavReader};

#[inline]
fn make_sample_format_str(sample_format: SampleFormat, bits_per_sample: u16) -> String {
    match sample_format {
        SampleFormat::Float => format!("FLOAT{}", bits_per_sample),
        SampleFormat::Int => format!("PCM{}", bits_per_sample),
    }
}

/// Decoder for the WAV format.
pub struct WavDecoder<R>
where
    R: Read + Seek,
{
    reader: SamplesIterator<R>,
    sample_rate: u32,
    channels: u16,
    sample_format_str: String,
}

impl<R> WavDecoder<R>
where
    R: Read + Seek,
{
    /// Attempts to decode the data as WAV.
    pub fn new(mut data: R) -> Result<WavDecoder<R>, R> {
        if !is_wave(data.by_ref()) {
            return Err(data);
        }

        let reader = WavReader::new(data).unwrap();
        let spec = reader.spec();
        let reader = SamplesIterator {
            reader,
            samples_read: 0,
        };

        Ok(WavDecoder {
            reader,
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            sample_format_str: make_sample_format_str(spec.sample_format, spec.bits_per_sample),
        })
    }
    pub fn into_inner(self) -> R {
        self.reader.reader.into_inner()
    }
}

struct SamplesIterator<R>
where
    R: Read + Seek,
{
    reader: WavReader<R>,
    samples_read: u32,
}

impl<R> Iterator for SamplesIterator<R>
where
    R: Read + Seek,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        let spec = self.reader.spec();
        match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Float, 32) => self.reader.samples().next().map(|value| {
                self.samples_read += 1;
                value.unwrap_or(0.0)
            }),
            (SampleFormat::Int, 32) => self.reader.samples().next().map(|value| {
                self.samples_read += 1;
                i32_to_f32(value.unwrap_or(0))
            }),
            (SampleFormat::Int, 16) => self.reader.samples().next().map(|value| {
                self.samples_read += 1;
                i16_to_f32(value.unwrap_or(0))
            }),
            (SampleFormat::Int, 24) => self.reader.samples().next().map(|value| {
                self.samples_read += 1;
                i24_to_f32(value.unwrap_or(0))
            }),
            (SampleFormat::Int, 8) => self.reader.samples().next().map(|value| {
                self.samples_read += 1;
                i8_to_f32(value.unwrap_or(0))
            }),
            (sample_format, bits_per_sample) => panic!(
                "Unimplemented wav spec: {:?}, {}",
                sample_format, bits_per_sample
            ),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.reader.len() - self.samples_read) as usize;
        (len, Some(len))
    }
}

impl<R> ExactSizeIterator for SamplesIterator<R> where R: Read + Seek {}

impl<R> Source for WavDecoder<R>
where
    R: Read + Seek,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        let ms = self.len() as u64 * 1000 / (self.channels as u64 * self.sample_rate as u64);
        Some(Duration::from_millis(ms))
    }

    #[inline]
    fn sample_format_str(&self) -> String {
        self.sample_format_str.clone()
    }
}

impl<R> Iterator for WavDecoder<R>
where
    R: Read + Seek,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        self.reader.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.reader.size_hint()
    }
}

impl<R> ExactSizeIterator for WavDecoder<R> where R: Read + Seek {}

/// Returns true if the stream contains WAV data, then resets it to where it was.
fn is_wave<R>(mut data: R) -> bool
where
    R: Read + Seek,
{
    let stream_pos = data.seek(SeekFrom::Current(0)).unwrap();

    if WavReader::new(data.by_ref()).is_err() {
        data.seek(SeekFrom::Start(stream_pos)).unwrap();
        return false;
    }

    data.seek(SeekFrom::Start(stream_pos)).unwrap();
    true
}
