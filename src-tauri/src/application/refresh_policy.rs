use std::time::{Duration, Instant};

use super::RefreshReason;

pub const SNAPSHOT_CACHE_TTL: Duration = Duration::from_secs(30);
pub const RESUME_STALE_AFTER: Duration = Duration::from_secs(60);
pub const VISIBLE_RESYNC_INTERVAL: Duration = Duration::from_secs(5 * 60);
pub const HIDDEN_RESYNC_INTERVAL: Duration = Duration::from_secs(10 * 60);

const RETRY_DELAYS: [Duration; 6] = [
    Duration::from_secs(30),
    Duration::from_secs(60),
    Duration::from_secs(2 * 60),
    Duration::from_secs(5 * 60),
    Duration::from_secs(15 * 60),
    Duration::from_secs(30 * 60),
];

pub struct RefreshPolicy {
    last_attempt: Option<Instant>,
    last_success: Option<Instant>,
    next_retry: Option<Instant>,
    consecutive_failures: u32,
    automatic_refresh_enabled: bool,
}

impl Default for RefreshPolicy {
    fn default() -> Self {
        Self {
            last_attempt: None,
            last_success: None,
            next_retry: None,
            consecutive_failures: 0,
            automatic_refresh_enabled: true,
        }
    }
}

impl RefreshPolicy {
    pub fn should_skip(&self, reason: RefreshReason, now: Instant) -> bool {
        match reason {
            RefreshReason::Resume => {
                self.next_retry.is_none()
                    && self.last_success.is_some_and(|last| {
                        now.saturating_duration_since(last) <= RESUME_STALE_AFTER
                    })
            }
            RefreshReason::VisibleResync | RefreshReason::HiddenResync => {
                self.next_retry.is_none() && self.cache_is_fresh(now)
            }
            _ => false,
        }
    }

    pub fn begin_attempt(&mut self, now: Instant) {
        self.last_attempt = Some(now);
    }

    pub fn record_success(&mut self, now: Instant) {
        self.last_success = Some(now);
        self.next_retry = None;
        self.consecutive_failures = 0;
        self.automatic_refresh_enabled = true;
    }

    pub fn record_failure(&mut self, now: Instant, retryable: bool) -> Option<Duration> {
        if !retryable {
            self.next_retry = None;
            self.consecutive_failures = 0;
            self.automatic_refresh_enabled = false;
            return None;
        }

        let retry_index = usize::try_from(self.consecutive_failures)
            .unwrap_or(usize::MAX)
            .min(RETRY_DELAYS.len() - 1);
        let delay = RETRY_DELAYS[retry_index];
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.next_retry = now.checked_add(delay);
        self.automatic_refresh_enabled = true;
        Some(delay)
    }

    pub fn scheduled_reason(&self, now: Instant, visible: bool) -> Option<RefreshReason> {
        if !self.automatic_refresh_enabled {
            return None;
        }

        if let Some(next_retry) = self.next_retry {
            return (now >= next_retry).then_some(if visible {
                RefreshReason::VisibleResync
            } else {
                RefreshReason::HiddenResync
            });
        }

        let interval = if visible {
            VISIBLE_RESYNC_INTERVAL
        } else {
            HIDDEN_RESYNC_INTERVAL
        };
        self.last_attempt
            .filter(|last| now.saturating_duration_since(*last) >= interval)
            .map(|_| {
                if visible {
                    RefreshReason::VisibleResync
                } else {
                    RefreshReason::HiddenResync
                }
            })
    }

    pub fn cache_is_fresh(&self, now: Instant) -> bool {
        self.last_success
            .is_some_and(|last| now.saturating_duration_since(last) <= SNAPSHOT_CACHE_TTL)
    }

    pub fn retry_attempt(&self) -> u32 {
        self.consecutive_failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_and_resume_use_monotonic_freshness_thresholds() {
        let now = Instant::now();
        let mut policy = RefreshPolicy::default();
        policy.record_success(now);

        assert!(policy.cache_is_fresh(now + Duration::from_secs(30)));
        assert!(!policy.cache_is_fresh(now + Duration::from_secs(31)));
        assert!(policy.should_skip(RefreshReason::Resume, now + Duration::from_secs(60)));
        assert!(!policy.should_skip(RefreshReason::Resume, now + Duration::from_secs(61)));
        assert!(!policy.should_skip(RefreshReason::Manual, now + Duration::from_secs(1)));
        assert!(policy.should_skip(RefreshReason::VisibleResync, now + Duration::from_secs(30)));
        assert!(!policy.should_skip(RefreshReason::VisibleResync, now + Duration::from_secs(31)));
    }

    #[test]
    fn visible_and_hidden_resync_use_different_intervals() {
        let now = Instant::now();
        let mut policy = RefreshPolicy::default();
        policy.begin_attempt(now);

        assert_eq!(
            policy.scheduled_reason(now + VISIBLE_RESYNC_INTERVAL, true),
            Some(RefreshReason::VisibleResync)
        );
        assert_eq!(
            policy.scheduled_reason(now + VISIBLE_RESYNC_INTERVAL, false),
            None
        );
        assert_eq!(
            policy.scheduled_reason(now + HIDDEN_RESYNC_INTERVAL, false),
            Some(RefreshReason::HiddenResync)
        );
    }

    #[test]
    fn retry_delay_grows_and_success_resets_it() {
        let now = Instant::now();
        let mut policy = RefreshPolicy::default();

        assert_eq!(
            policy.record_failure(now, true),
            Some(Duration::from_secs(30))
        );
        assert_eq!(policy.retry_attempt(), 1);
        assert_eq!(
            policy.record_failure(now + Duration::from_secs(30), true),
            Some(Duration::from_secs(60))
        );
        assert_eq!(policy.retry_attempt(), 2);
        policy.record_success(now + Duration::from_secs(90));
        assert_eq!(policy.retry_attempt(), 0);
        assert_eq!(
            policy.record_failure(now + Duration::from_secs(91), true),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn non_retryable_failure_pauses_automatic_refresh_until_success() {
        let now = Instant::now();
        let mut policy = RefreshPolicy::default();
        policy.begin_attempt(now);
        assert_eq!(policy.record_failure(now, false), None);
        assert_eq!(
            policy.scheduled_reason(now + HIDDEN_RESYNC_INTERVAL, true),
            None
        );

        policy.record_success(now + HIDDEN_RESYNC_INTERVAL);
        policy.begin_attempt(now + HIDDEN_RESYNC_INTERVAL);
        assert_eq!(
            policy.scheduled_reason(now + HIDDEN_RESYNC_INTERVAL + VISIBLE_RESYNC_INTERVAL, true),
            Some(RefreshReason::VisibleResync)
        );
    }

    #[test]
    fn retry_deadline_takes_precedence_over_a_recent_success() {
        let now = Instant::now();
        let mut policy = RefreshPolicy::default();
        policy.record_success(now);
        policy.record_failure(now + Duration::from_secs(1), true);

        assert!(!policy.should_skip(RefreshReason::Resume, now + Duration::from_secs(2)));
        assert!(!policy.should_skip(RefreshReason::VisibleResync, now + Duration::from_secs(2)));
    }
}
