use std::{
    collections::{HashMap, VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_FAILURES_PER_IP_AND_EMAIL: usize = 5;
const MAX_FAILURES_PER_IP: usize = 25;

#[derive(Clone, Default)]
pub struct LoginRateLimiter {
    attempts: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl LoginRateLimiter {
    pub fn retry_after_seconds(&self, ip: &str, email: &str) -> Option<u64> {
        self.retry_after_at(ip, email, Instant::now())
    }

    pub fn record_failure(&self, ip: &str, email: &str) {
        let now = Instant::now();
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        prune(&mut attempts, now);
        attempts.entry(ip_key(ip)).or_default().push_back(now);
        attempts.entry(pair_key(ip, email)).or_default().push_back(now);
    }

    pub fn clear_pair(&self, ip: &str, email: &str) {
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        attempts.remove(&pair_key(ip, email));
    }

    fn retry_after_at(&self, ip: &str, email: &str, now: Instant) -> Option<u64> {
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        prune(&mut attempts, now);

        let ip_attempts = attempts.get(&ip_key(ip));
        let pair_attempts = attempts.get(&pair_key(ip, email));

        let oldest = if pair_attempts.map_or(0, VecDeque::len) >= MAX_FAILURES_PER_IP_AND_EMAIL {
            pair_attempts.and_then(|entries| entries.front())
        } else if ip_attempts.map_or(0, VecDeque::len) >= MAX_FAILURES_PER_IP {
            ip_attempts.and_then(|entries| entries.front())
        } else {
            None
        }?;

        Some(WINDOW.saturating_sub(now.duration_since(*oldest)).as_secs().max(1))
    }
}

fn ip_key(ip: &str) -> String {
    format!("ip:{ip}")
}

fn pair_key(ip: &str, email: &str) -> String {
    let mut hasher = DefaultHasher::new();
    ip.hash(&mut hasher);
    email.hash(&mut hasher);
    format!("pair:{:x}", hasher.finish())
}

fn prune(attempts: &mut HashMap<String, VecDeque<Instant>>, now: Instant) {
    attempts.retain(|_, entries| {
        while entries
            .front()
            .is_some_and(|timestamp| now.duration_since(*timestamp) >= WINDOW)
        {
            entries.pop_front();
        }
        !entries.is_empty()
    });
}

#[cfg(test)]
mod tests {
    use super::{LoginRateLimiter, MAX_FAILURES_PER_IP_AND_EMAIL};

    #[test]
    fn limits_an_ip_and_email_pair_and_can_clear_it() {
        let limiter = LoginRateLimiter::default();
        for _ in 0..MAX_FAILURES_PER_IP_AND_EMAIL {
            limiter.record_failure("192.0.2.1", "user@example.com");
        }

        assert!(
            limiter
                .retry_after_seconds("192.0.2.1", "user@example.com")
                .is_some()
        );

        limiter.clear_pair("192.0.2.1", "user@example.com");
        assert!(
            limiter
                .retry_after_seconds("192.0.2.1", "user@example.com")
                .is_none()
        );
    }

    #[test]
    fn does_not_mix_email_addresses_on_the_same_ip_below_ip_limit() {
        let limiter = LoginRateLimiter::default();
        for _ in 0..MAX_FAILURES_PER_IP_AND_EMAIL {
            limiter.record_failure("192.0.2.1", "first@example.com");
        }

        assert!(
            limiter
                .retry_after_seconds("192.0.2.1", "second@example.com")
                .is_none()
        );
    }
}
