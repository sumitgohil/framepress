//! Cancellation tokens. Cheap, `Clone`-able, `Send + Sync`. The optimizer
//! checks the token between candidate scoring rounds; cancel is best-effort
//! because compression is CPU-bound and short.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// A simple cancel flag. One per job. Cheap to clone, no allocation.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a fresh, uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark this token as cancelled.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Whether cancel has been called. Cheap atomic load.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_propagates_via_clone() {
        let t = CancellationToken::new();
        let t2 = t.clone();
        assert!(!t.is_cancelled());
        assert!(!t2.is_cancelled());
        t.cancel();
        assert!(t.is_cancelled());
        assert!(t2.is_cancelled());
    }
}
