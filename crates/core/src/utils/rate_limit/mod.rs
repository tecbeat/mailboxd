//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use dashmap::DashMap;
use governor::{
    clock::{QuantaClock, QuantaInstant},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    DefaultKeyedRateLimiter, NotUntil, Quota, RateLimiter,
};
use std::{
    net::IpAddr,
    num::NonZero,
    sync::{Arc, LazyLock},
    time::Duration,
};

use crate::users::acl::RateLimit;

pub static RATE_LIMITER_MANAGER: LazyLock<UserRateLimiter> = LazyLock::new(UserRateLimiter::new);

pub struct UserRateLimiter {
    limiters: Arc<
        DashMap<
            u64,
            (
                Arc<RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware>>,
                RateLimit,
            ),
        >,
    >,
}

impl UserRateLimiter {
    pub fn new() -> Self {
        UserRateLimiter {
            limiters: Arc::new(DashMap::new()),
        }
    }

    pub async fn check(
        &self,
        user_id: u64,
        limit: RateLimit,
    ) -> Result<(), NotUntil<QuantaInstant>> {
        let limiter = self.get_or_update_limiter(user_id, limit).await;
        limiter.check()
    }

    async fn get_or_update_limiter(
        &self,
        user_id: u64,
        limit: RateLimit,
    ) -> Arc<RateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware>> {
        self.limiters
            .entry(user_id)
            .and_modify(|(existing_limiter, current_limit)| {
                if current_limit.interval != limit.interval || current_limit.quota != limit.quota {
                    let quota = Quota::with_period(Duration::from_secs(limit.interval))
                        .unwrap()
                        .allow_burst(NonZero::new(limit.quota).unwrap());
                    *existing_limiter = Arc::new(RateLimiter::direct_with_clock(
                        quota,
                        QuantaClock::default(),
                    ));
                    *current_limit = RateLimit {
                        interval: limit.interval,
                        quota: limit.quota,
                    };
                }
            })
            .or_insert({
                let quota = Quota::with_period(Duration::from_secs(limit.interval))
                    .unwrap()
                    .allow_burst(NonZero::new(limit.quota).unwrap());
                (
                    Arc::new(RateLimiter::direct_with_clock(
                        quota,
                        QuantaClock::default(),
                    )),
                    limit,
                )
            })
            .value()
            .0
            .clone()
    }
}

/// Attempts per client IP before login requests are rejected.
const LOGIN_BURST: u32 = 10;
/// Time for one spent login attempt to be replenished.
const LOGIN_REPLENISH_PERIOD: Duration = Duration::from_secs(600);
/// Tracked clients above which idle entries are evicted.
const LOGIN_LIMITER_EVICT_THRESHOLD: usize = 1024;

pub static LOGIN_RATE_LIMITER: LazyLock<LoginRateLimiter> =
    LazyLock::new(|| LoginRateLimiter::with_quota(LOGIN_BURST, LOGIN_REPLENISH_PERIOD));

/// Per-client-IP limiter for password login attempts.
pub struct LoginRateLimiter {
    limiter: DefaultKeyedRateLimiter<IpAddr>,
}

impl LoginRateLimiter {
    pub fn with_quota(burst: u32, replenish_period: Duration) -> Self {
        let quota = Quota::with_period(replenish_period)
            .expect("login replenish period must be non-zero")
            .allow_burst(NonZero::new(burst).expect("login burst must be non-zero"));
        LoginRateLimiter {
            limiter: RateLimiter::keyed(quota),
        }
    }

    /// Record a login attempt from `ip`; returns `false` when it exceeds the
    /// quota and must be rejected.
    pub fn check(&self, ip: IpAddr) -> bool {
        if self.limiter.len() > LOGIN_LIMITER_EVICT_THRESHOLD {
            self.limiter.retain_recent();
        }
        self.limiter.check_key(&ip).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    const CLIENT_A: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    const CLIENT_B: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));

    #[test]
    fn login_limiter_rejects_attempts_beyond_the_burst() {
        let limiter = LoginRateLimiter::with_quota(3, Duration::from_secs(600));
        for _ in 0..3 {
            assert!(limiter.check(CLIENT_A));
        }
        assert!(!limiter.check(CLIENT_A));
    }

    #[test]
    fn login_limiter_tracks_clients_independently() {
        let limiter = LoginRateLimiter::with_quota(1, Duration::from_secs(600));
        assert!(limiter.check(CLIENT_A));
        assert!(!limiter.check(CLIENT_A));
        assert!(limiter.check(CLIENT_B));
    }
}
