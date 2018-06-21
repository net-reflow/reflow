use std::time;
use std::fmt;
use std::sync::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct FailureCounter{
    fail_count: Arc<AtomicUsize>,
    max_count: usize,
    last_fail_time: RwLock<time::Instant>,
}

impl fmt::Debug for FailureCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let fails = self.fail_count.load(Ordering::Relaxed);
        write!(f, "FailureCounter({} fails)", fails)
    }
}

impl FailureCounter{
    pub fn new()-> FailureCounter {
        FailureCounter{
            fail_count: Arc::new(AtomicUsize::new(0)),
            max_count: 6,
            last_fail_time: RwLock::new(time::Instant::now()),
        }
    }

    pub fn should_wait(&self)-> bool {
        let fails = self.fail_count.load(Ordering::Relaxed);
        if fails == 0 {
            false
        } else {
            let waittimesec = 2u64.pow(fails as u32);
            let lft =
                self.last_fail_time.read().map(|rg| rg.clone());
            if let Ok(ft) = lft {
                let timesincefail = time::Instant::now().duration_since(ft).as_secs();
                let remainwait = waittimesec.checked_sub(timesincefail);
                match remainwait {
                    Some(t) => {
                        debug!("should wait {} seconds more", t);
                        true
                    }
                    None => {
                        self.update_attempt_time();
                        false
                    }
                }
            } else {
                true
            }
        }
    }

    pub fn log_success(&self) {
        self.fail_count.store(0, Ordering::Relaxed);
    }

    fn update_attempt_time(&self) {
        let mut ft =
            self.last_fail_time.write().unwrap();
        *ft = time::Instant::now();
    }

    pub fn log_failure(&self) {
        let fails = self.fail_count.load(Ordering::Relaxed);
        if fails < self.max_count {
            self.fail_count.compare_and_swap(fails, fails + 1, Ordering::Relaxed);
        }
        self.update_attempt_time();
    }
}
