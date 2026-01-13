use std::time::Instant;

/// CPU time measurement structure
#[derive(Debug, Clone)]
pub struct CpuTime {
    pub user_time: f64,   // User CPU time in seconds
    pub system_time: f64, // System (kernel) CPU time in seconds
    pub wall_time: f64,   // Wall clock time in seconds
    pub start_time: Instant,
}

impl CpuTime {
    pub fn new() -> Self {
        CpuTime {
            user_time: 0.0,
            system_time: 0.0,
            wall_time: 0.0,
            start_time: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.user_time = 0.0;
        self.system_time = 0.0;
        self.wall_time = 0.0;
    }

    pub fn stop(&mut self) {
        self.wall_time = self.start_time.elapsed().as_secs_f64();
    }

    pub fn cpu_total(&self) -> f64 {
        self.user_time + self.system_time
    }

    pub fn cpu_percent(&self) -> f64 {
        if self.wall_time > 0.0 {
            (self.cpu_total() / self.wall_time) * 100.0
        } else {
            0.0
        }
    }

    /// Get memory usage in MB (Linux specific)
    #[cfg(target_os = "linux")]
    pub fn get_memory_mb() -> f64 {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        if let Ok(file) = File::open("/proc/self/status") {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(l) = line {
                    if l.starts_with("VmRSS:") {
                        // Parse "VmRSS:     12345 kB"
                        let parts: Vec<&str> = l.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<f64>() {
                                return kb / 1024.0;
                            }
                        }
                    }
                }
            }
        }
        0.0
    }

    #[cfg(not(target_os = "linux"))]
    pub fn get_memory_mb() -> f64 {
        0.0
    }
}

/// RTT statistics
#[derive(Debug, Clone)]
pub struct RttStats {
    pub rtts: Vec<f64>,
    pub count: usize,
    pub avg: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub total_time: f64,
}

impl RttStats {
    pub fn new(rtts: Vec<f64>) -> Self {
        let count = rtts.len();
        let (avg, median, min, max) = if count > 0 {
            let sum: f64 = rtts.iter().sum();
            let avg = sum / count as f64;
            let min = *rtts.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
            let max = *rtts.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

            // Calculate median
            let mut sorted = rtts.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = if count % 2 == 0 {
                (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
            } else {
                sorted[count / 2]
            };

            (avg, median, min, max)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        RttStats {
            rtts,
            count,
            avg,
            median,
            min,
            max,
            total_time: 0.0,
        }
    }

    pub fn calculate_throughput(&self) -> f64 {
        if self.total_time > 0.0 {
            (self.count * 2) as f64 / self.total_time // Each ping-pong is 2 messages
        } else {
            0.0
        }
    }

    /// Print CPU time statistics with throughput metrics
    pub fn print_cpu_time(cpu: &CpuTime, ping_pong_count: usize) {
        let cpu_total = cpu.cpu_total();
        let cpu_percent = cpu.cpu_percent();
        let msg_per_sec = if cpu.wall_time > 0.0 {
            (ping_pong_count * 2) as f64 / cpu.wall_time
        } else {
            0.0
        };
        let mem_mb = CpuTime::get_memory_mb();

        println!("├──────────────────────────────────────────────────────────────┤");
        println!("│ Throughput Statistics:                                       │");
        println!("│   Total time:   {:8.3} sec                                 │", cpu.wall_time);
        println!("│   Messages:     {:8} (ping+pong)                         │", ping_pong_count * 2);
        println!("│   Throughput:   {:8.1} msg/sec                             │", msg_per_sec);
        println!("├──────────────────────────────────────────────────────────────┤");
        println!("│ CPU Time Statistics:                                         │");
        println!("│   User time:    {:8.3} sec                                 │", cpu.user_time);
        println!("│   System time:  {:8.3} sec                                 │", cpu.system_time);
        println!("│   CPU total:    {:8.3} sec                                 │", cpu_total);
        println!("│   Wall time:    {:8.3} sec                                 │", cpu.wall_time);
        println!("│   CPU usage:    {:7.1}%                                    │", cpu_percent);
        println!("├──────────────────────────────────────────────────────────────┤");
        println!("│ Memory Statistics:                                           │");
        println!("│   Memory (RSS): {:8.2} MB                                  │", mem_mb);
    }

    /// Print RTT statistics
    pub fn print_rtt_stats(&self) {
        println!("├──────────────────────────────────────────────────────────────┤");
        println!("│ RTT Statistics:                                              │");
        println!("│   Avg: {:7.3} ms | Median: {:7.3} ms                       │", self.avg, self.median);
        println!("│   Min: {:7.3} ms | Max:    {:7.3} ms                       │", self.min, self.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_time_creation() {
        let cpu = CpuTime::new();
        assert_eq!(cpu.user_time, 0.0);
        assert_eq!(cpu.system_time, 0.0);
        assert_eq!(cpu.wall_time, 0.0);
    }

    #[test]
    fn test_rtt_stats_calculation() {
        let rtts = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let stats = RttStats::new(rtts);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 50.0);
        assert_eq!(stats.avg, 30.0);
        assert_eq!(stats.median, 30.0);
    }

    #[test]
    fn test_rtt_stats_even_count() {
        let rtts = vec![10.0, 20.0, 30.0, 40.0];
        let stats = RttStats::new(rtts);

        assert_eq!(stats.median, 25.0); // (20 + 30) / 2
    }

    #[test]
    fn test_rtt_stats_empty() {
        let rtts: Vec<f64> = vec![];
        let stats = RttStats::new(rtts);

        assert_eq!(stats.count, 0);
        assert_eq!(stats.avg, 0.0);
        assert_eq!(stats.median, 0.0);
    }
}
