use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::Notify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ConnectionRefused {
    #[error("server is full")]
    ServerFull,
    #[error("too many connections from this address")]
    TooManyFromAddress,
}

/// Counts live WebSocket connections, overall and per client address. A slot is taken before
/// the upgrade and released when the connection ends (or the upgrade fails). An IPv6 client
/// counts by its /64: one subscriber usually gets a whole /64 and could take every slot
/// otherwise.
#[derive(Debug)]
pub(crate) struct ConnectionLimiter {
    max_total: usize,
    max_per_ip: usize,
    total: AtomicUsize,
    per_ip: Mutex<HashMap<IpAddr, usize>>,
    all_closed: Notify,
}

/// One live connection; dropping it gives the slot back.
#[derive(Debug)]
pub(crate) struct ConnectionSlot {
    limiter: Arc<ConnectionLimiter>,
    ip: IpAddr,
}

impl ConnectionLimiter {
    pub(crate) fn new(max_total: usize, max_per_ip: usize) -> Self {
        Self {
            max_total,
            max_per_ip,
            total: AtomicUsize::new(0),
            per_ip: Mutex::new(HashMap::new()),
            all_closed: Notify::new(),
        }
    }

    pub(crate) fn acquire(
        self: &Arc<Self>,
        ip: IpAddr,
    ) -> Result<ConnectionSlot, ConnectionRefused> {
        let ip = address_block(ip);
        // Increment first and roll back, so concurrent upgrades cannot all pass the check.
        if self.total.fetch_add(1, Ordering::AcqRel) >= self.max_total {
            self.release_total();
            return Err(ConnectionRefused::ServerFull);
        }
        let mut per_ip = self.per_ip.lock().unwrap_or_else(PoisonError::into_inner);
        let count = per_ip.entry(ip).or_insert(0);
        if *count >= self.max_per_ip {
            if *count == 0 {
                per_ip.remove(&ip);
            }
            drop(per_ip);
            self.release_total();
            return Err(ConnectionRefused::TooManyFromAddress);
        }
        *count += 1;
        Ok(ConnectionSlot {
            limiter: self.clone(),
            ip,
        })
    }

    fn release_total(&self) {
        if self.total.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.all_closed.notify_waiters();
        }
    }

    /// Resolves once no connection is left.
    pub(crate) async fn all_closed(&self) {
        loop {
            let notified = self.all_closed.notified();
            tokio::pin!(notified);
            // Registered before the check, so a release in between is not missed.
            notified.as_mut().enable();
            if self.total.load(Ordering::Acquire) == 0 {
                return;
            }
            notified.await;
        }
    }
}

/// The address a client is counted by: IPv4 as it is, IPv6 by its /64 prefix.
fn address_block(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(Ipv6Addr::from_bits(v6.to_bits() & !u128::from(u64::MAX))),
        },
    }
}

impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        let mut per_ip = self
            .limiter
            .per_ip
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = per_ip.get_mut(&self.ip) {
            *count -= 1;
            if *count == 0 {
                per_ip.remove(&self.ip);
            }
        }
        drop(per_ip);
        self.limiter.release_total();
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    const A: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    const B: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

    #[tokio::test]
    async fn limits_per_address_and_overall_and_releases_on_drop() {
        let limiter = Arc::new(ConnectionLimiter::new(3, 2));
        let first = limiter.acquire(A).unwrap();
        let _second = limiter.acquire(A).unwrap();
        assert_eq!(
            limiter.acquire(A).unwrap_err(),
            ConnectionRefused::TooManyFromAddress
        );
        let _third = limiter.acquire(B).unwrap();
        assert_eq!(
            limiter.acquire(B).unwrap_err(),
            ConnectionRefused::ServerFull
        );
        drop(first);
        let _again = limiter.acquire(A).unwrap();
    }

    #[tokio::test]
    async fn ipv6_clients_count_by_their_64_block() {
        let limiter = Arc::new(ConnectionLimiter::new(10, 2));
        let address = |last: u16| IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 1, 2, 0, 0, 0, last));
        let _first = limiter.acquire(address(1)).unwrap();
        let _second = limiter.acquire(address(2)).unwrap();
        assert_eq!(
            limiter.acquire(address(3)).unwrap_err(),
            ConnectionRefused::TooManyFromAddress
        );
        let next_block = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 1, 3, 0, 0, 0, 1));
        assert!(limiter.acquire(next_block).is_ok());
    }

    #[tokio::test]
    async fn all_closed_waits_for_the_last_slot() {
        let limiter = Arc::new(ConnectionLimiter::new(3, 3));
        let slot = limiter.acquire(A).unwrap();
        let waiter = tokio::spawn({
            let limiter = limiter.clone();
            async move { limiter.all_closed().await }
        });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        drop(slot);
        waiter.await.unwrap();
    }
}
