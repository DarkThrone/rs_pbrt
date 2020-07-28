// std
use std::sync::Arc;
// pbrt
use crate::core::geometry::{Point2f, Point2i};
use crate::core::paramset::ParamSet;
use crate::core::pbrt::Float;
use crate::core::rng::Rng;
use crate::core::sampler::Sampler;
use crate::core::sampling::{latin_hypercube, shuffle, stratified_sample_1d, stratified_sample_2d};

pub struct StratifiedSampler {
    pub samples_per_pixel: i64,
    pub x_pixel_samples: i32,
    pub y_pixel_samples: i32,
    pub jitter_samples: bool,
    // inherited from class PixelSampler (see sampler.h)
    pub samples_1d: Vec<Vec<Float>>,
    pub samples_2d: Vec<Vec<Point2f>>,
    pub current_1d_dimension: i32,
    pub current_2d_dimension: i32,
    pub rng: Rng,
    // inherited from class Sampler (see sampler.h)
    pub current_pixel: Point2i,
    pub current_pixel_sample_index: i64,
    pub samples_1d_array_sizes: Vec<i32>,
    pub samples_2d_array_sizes: Vec<i32>,
    pub sample_array_1d: Vec<Vec<Float>>,
    pub sample_array_2d: Vec<Vec<Point2f>>,
    pub array_1d_offset: usize,
    pub array_2d_offset: usize,
}

impl StratifiedSampler {
    pub fn new(
        x_pixel_samples: i32,
        y_pixel_samples: i32,
        jitter_samples: bool,
        n_sampled_dimensions: i64,
    ) -> Self {
        let mut ss = StratifiedSampler {
            samples_per_pixel: (x_pixel_samples * y_pixel_samples) as i64,
            x_pixel_samples,
            y_pixel_samples,
            jitter_samples,
            samples_1d: Vec::new(),
            samples_2d: Vec::new(),
            current_1d_dimension: 0_i32,
            current_2d_dimension: 0_i32,
            rng: Rng::default(),
            current_pixel: Point2i::default(),
            current_pixel_sample_index: 0_i64,
            samples_1d_array_sizes: Vec::new(),
            samples_2d_array_sizes: Vec::new(),
            sample_array_1d: Vec::new(),
            sample_array_2d: Vec::new(),
            array_1d_offset: 0_usize,
            array_2d_offset: 0_usize,
        };
        for _i in 0..n_sampled_dimensions {
            let additional_1d: Vec<Float> = vec![0.0; ss.samples_per_pixel as usize];
            let additional_2d: Vec<Point2f> =
                vec![Point2f::default(); ss.samples_per_pixel as usize];
            ss.samples_1d.push(additional_1d);
            ss.samples_2d.push(additional_2d);
        }
        ss
    }
    pub fn clone_with_seed(&self, seed: u64) -> Arc<Sampler> {
        let mut ss = StratifiedSampler {
            samples_per_pixel: self.samples_per_pixel,
            x_pixel_samples: self.x_pixel_samples,
            y_pixel_samples: self.y_pixel_samples,
            jitter_samples: self.jitter_samples,
            samples_1d: self.samples_1d.clone(),
            samples_2d: self.samples_2d.clone(),
            current_1d_dimension: self.current_1d_dimension,
            current_2d_dimension: self.current_2d_dimension,
            rng: self.rng,
            current_pixel: self.current_pixel,
            current_pixel_sample_index: self.current_pixel_sample_index,
            samples_1d_array_sizes: self.samples_1d_array_sizes.to_vec(),
            samples_2d_array_sizes: self.samples_2d_array_sizes.to_vec(),
            sample_array_1d: self.sample_array_1d.to_vec(),
            sample_array_2d: self.sample_array_2d.to_vec(),
            array_1d_offset: self.array_1d_offset,
            array_2d_offset: self.array_2d_offset,
        };
        ss.reseed(seed);
        let sampler = Sampler::Stratified(ss);
        Arc::new(sampler)
    }
    pub fn create(params: &ParamSet) -> Arc<Sampler> {
        let jitter: bool = params.find_one_bool("jitter", true);
        let xsamp: i32 = params.find_one_int("xsamples", 4);
        let ysamp: i32 = params.find_one_int("ysamples", 4);
        let sd: i32 = params.find_one_int("dimensions", 4);
        // TODO: if (PbrtOptions.quickRender) nsamp = 1;
        Arc::new(Sampler::Stratified(StratifiedSampler::new(
            xsamp, ysamp, jitter, sd as i64,
        )))
    }
    // Sampler
    pub fn start_pixel(&mut self, p: Point2i) {
        // TODO: ProfilePhase _(Prof::StartPixel);
        // generate single stratified samples for the pixel
        for i in 0..self.samples_1d.len() {
            let samples: &mut [Float] = self.samples_1d[i].as_mut_slice();
            stratified_sample_1d(
                samples,
                self.x_pixel_samples * self.y_pixel_samples,
                &mut self.rng,
                self.jitter_samples,
            );
            shuffle(
                samples,
                self.x_pixel_samples * self.y_pixel_samples,
                1,
                &mut self.rng,
            );
        }
        for i in 0..self.samples_2d.len() {
            let samples: &mut [Point2f] = self.samples_2d[i].as_mut_slice();
            stratified_sample_2d(
                samples,
                self.x_pixel_samples,
                self.y_pixel_samples,
                &mut self.rng,
                self.jitter_samples,
            );
            shuffle(
                samples,
                self.x_pixel_samples * self.y_pixel_samples,
                1,
                &mut self.rng,
            );
        }
        // generate arrays of stratified samples for the pixel
        for i in 0..self.samples_1d_array_sizes.len() {
            for j in 0..self.samples_per_pixel {
                let count: i32 = self.samples_1d_array_sizes[i as usize];
                let samples: &mut [Float] =
                    &mut self.sample_array_1d[i][(j as usize * count as usize)..];
                stratified_sample_1d(samples, count, &mut self.rng, self.jitter_samples);
                shuffle(samples, count, 1, &mut self.rng);
            }
        }
        for i in 0..self.samples_2d_array_sizes.len() {
            for j in 0..self.samples_per_pixel {
                let count: u32 = self.samples_2d_array_sizes[i as usize] as u32;
                latin_hypercube(
                    &mut self.sample_array_2d[i as usize][(j as usize * count as usize)..],
                    count,
                    // 2,
                    &mut self.rng,
                );
            }
        }
        // PixelSampler::StartPixel(p);
        self.current_pixel = p;
        self.current_pixel_sample_index = 0_i64;
        // reset array offsets for next pixel sample
        self.array_1d_offset = 0_usize;
        self.array_2d_offset = 0_usize;
    }
    pub fn get_1d(&mut self) -> Float {
        // TODO: ProfilePhase _(Prof::GetSample);
        assert!(
            self.current_pixel_sample_index < self.samples_per_pixel,
            "current_pixel_sample_index = {}, samples_per_pixel = {}",
            self.current_pixel_sample_index,
            self.samples_per_pixel
        );
        if self.current_1d_dimension < self.samples_1d.len() as i32 {
            let sample: Float = self.samples_1d[self.current_1d_dimension as usize]
                [self.current_pixel_sample_index as usize];
            self.current_1d_dimension += 1;
            sample
        } else {
            self.rng.uniform_float()
        }
    }
    pub fn get_2d(&mut self) -> Point2f {
        // TODO: ProfilePhase _(Prof::GetSample);
        assert!(
            self.current_pixel_sample_index < self.samples_per_pixel,
            "current_pixel_sample_index = {}, samples_per_pixel = {}",
            self.current_pixel_sample_index,
            self.samples_per_pixel
        );
        if self.current_2d_dimension < self.samples_2d.len() as i32 {
            let sample: Point2f = self.samples_2d[self.current_2d_dimension as usize]
                [self.current_pixel_sample_index as usize];
            self.current_2d_dimension += 1;
            sample
        } else {
            // C++ call order for Point2f(rng.UniformFloat(), rng.UniformFloat());
            let y = self.rng.uniform_float();
            let x = self.rng.uniform_float();
            Point2f { x, y }
        }
    }
    pub fn request_2d_array(&mut self, n: i32) {
        assert_eq!(self.round_count(n), n);
        self.samples_2d_array_sizes.push(n);
        let size: usize = (n * self.samples_per_pixel as i32) as usize;
        let additional_points: Vec<Point2f> = vec![Point2f::default(); size];
        self.sample_array_2d.push(additional_points);
    }
    pub fn round_count(&self, count: i32) -> i32 {
        count
    }
    pub fn get_2d_array(&mut self, n: i32) -> Option<&[Point2f]> {
        if self.array_2d_offset == self.sample_array_2d.len() {
            return None;
        }
        assert_eq!(self.samples_2d_array_sizes[self.array_2d_offset], n);
        assert!(
            self.current_pixel_sample_index < self.samples_per_pixel,
            "self.current_pixel_sample_index ({}) < self.samples_per_pixel ({})",
            self.current_pixel_sample_index,
            self.samples_per_pixel
        );
        let start: usize = (self.current_pixel_sample_index * n as i64) as usize;
        let end: usize = start + n as usize;
        self.array_2d_offset += 1;
        Some(&self.sample_array_2d[self.array_2d_offset - 1][start..end])
    }
    pub fn get_2d_arrays(&mut self, n: i32) -> (Option<&[Point2f]>, Option<&[Point2f]>) {
        if self.array_2d_offset == self.sample_array_2d.len() {
            return (None, None);
        }
        assert_eq!(self.samples_2d_array_sizes[self.array_2d_offset], n);
        assert!(
            self.current_pixel_sample_index < self.samples_per_pixel,
            "self.current_pixel_sample_index ({}) < self.samples_per_pixel ({})",
            self.current_pixel_sample_index,
            self.samples_per_pixel
        );
        let start: usize = (self.current_pixel_sample_index * n as i64) as usize;
        let end: usize = start + n as usize;
        self.array_2d_offset += 1;
        let ret1 = &self.sample_array_2d[self.array_2d_offset - 1][start..end];
        // repeat code from above
        if self.array_2d_offset == self.sample_array_2d.len() {
            return (None, None);
        }
        assert_eq!(self.samples_2d_array_sizes[self.array_2d_offset], n);
        assert!(
            self.current_pixel_sample_index < self.samples_per_pixel,
            "self.current_pixel_sample_index ({}) < self.samples_per_pixel ({})",
            self.current_pixel_sample_index,
            self.samples_per_pixel
        );
        let start: usize = (self.current_pixel_sample_index * n as i64) as usize;
        let end: usize = start + n as usize;
        self.array_2d_offset += 1;
        let ret2 = &self.sample_array_2d[self.array_2d_offset - 1][start..end];
        // return tuple
        (Some(ret1), Some(ret2))
    }
    pub fn get_2d_array_vec(&mut self, n: i32) -> Vec<Point2f> {
        let mut samples: Vec<Point2f> = Vec::new();
        if self.array_2d_offset == self.sample_array_2d.len() {
            return samples;
        }
        assert_eq!(self.samples_2d_array_sizes[self.array_2d_offset], n);
        assert!(
            self.current_pixel_sample_index < self.samples_per_pixel,
            "self.current_pixel_sample_index ({}) < self.samples_per_pixel ({})",
            self.current_pixel_sample_index,
            self.samples_per_pixel
        );
        let start: usize = (self.current_pixel_sample_index * n as i64) as usize;
        let end: usize = start + n as usize;
        samples = self.sample_array_2d[self.array_2d_offset][start..end].to_vec();
        self.array_2d_offset += 1;
        samples
    }
    pub fn start_next_sample(&mut self) -> bool {
        self.current_1d_dimension = 0_i32;
        self.current_2d_dimension = 0_i32;
        // Sampler::StartNextSample()
        // reset array offsets for next pixel sample
        self.array_1d_offset = 0_usize;
        self.array_2d_offset = 0_usize;
        self.current_pixel_sample_index += 1_i64;
        self.current_pixel_sample_index < self.samples_per_pixel
    }
    pub fn reseed(&mut self, seed: u64) {
        self.rng.set_sequence(seed);
    }
    pub fn get_current_pixel(&self) -> Point2i {
        self.current_pixel
    }
    pub fn get_current_sample_number(&self) -> i64 {
        self.current_pixel_sample_index
    }
    pub fn get_samples_per_pixel(&self) -> i64 {
        self.samples_per_pixel
    }
}
