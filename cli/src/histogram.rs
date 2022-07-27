use std::fmt::{self, Write};

#[derive(Debug)]
pub struct Histogram {
    min: f64,
    max: f64,
    bins: usize,
    values: Vec<usize>,
}

impl Histogram {
    pub fn new(data: &[f64], min: f64, max: f64, bins: usize) -> Histogram {
        let mut values: Vec<usize> = vec![0; bins];

        let step = (max - min) / bins as f64;
        let scale = 100.0;

        for &y in data.iter() {
            if y < min || y > max {
                continue;
            }

            let mut bucket_id = ((y * scale).floor() / (step * scale)) as usize;

            // Account for packages with a "perfect" (i.e. 1.0) score
            // This is generally unlikely but possible with packages that have
            //  not yet had analytics run on them
            // Also account for scores on the edge 10, 20, 30...
            if y != 0.0 && (y * 100.0) % 10.0 == 0.0 {
                bucket_id -= 1;
            }

            if bucket_id < values.len() {
                values[bucket_id as usize] += 1;
            }
        }
        Histogram { min, max, bins, values }
    }

    fn buckets(&self) -> Vec<(f64, f64)> {
        let step = (self.max - self.min) / self.bins as f64;
        let mut buckets: Vec<(f64, f64)> = Vec::new();

        let mut acc = self.min;
        while acc < self.max {
            buckets.push((acc, acc + step));
            acc += step;
        }
        buckets.pop();
        buckets
    }
}

impl fmt::Display for Histogram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Scale package count to cell width for the histogram's bar.
        let max = *self.values.iter().max().unwrap_or(&1) as f32;
        let scale = |count| (56.0 * f32::log2(count as f32 * 2.) / f32::log2(max * 2.)) as usize;

        let mut histogram = String::new();
        for (count, (min, max)) in self.values.iter().rev().zip(self.buckets().iter().rev()) {
            let bar_min = if *min < f64::EPSILON { 0 } else { (100. * min).round() as u32 + 1 };
            let bar_max = (100. * max).round() as u32;

            let bar = "█".repeat(scale(*count));

            let _ = write!(histogram, "\n{:>4} - {:<4} [{:>5}] {}", bar_min, bar_max, count, bar);
        }

        write!(f, "{:^10} {:>8}", "Score", "Count")?;
        write!(f, "{}", histogram)
    }
}
