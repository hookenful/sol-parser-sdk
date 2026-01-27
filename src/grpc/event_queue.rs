use crate::DexEvent;
use crossbeam_queue::ArrayQueue;
use std::time::Duration;
use tokio::sync::Notify;

/// Lock-free queue with async Notify for non-blocking consumers.
pub struct EventQueue {
    queue: ArrayQueue<DexEvent>,
    notify: Notify,
}

impl EventQueue {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self { queue: ArrayQueue::new(capacity), notify: Notify::new() }
    }

    #[inline(always)]
    pub fn push(&self, event: DexEvent) -> Result<(), DexEvent> {
        match self.queue.push(event) {
            Ok(()) => {
                self.notify.notify_one();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    #[inline(always)]
    pub fn pop(&self) -> Option<DexEvent> {
        self.queue.pop()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }

    /// Wait for the next event without busy-waiting.
    pub async fn recv(&self) -> DexEvent {
        loop {
            let notified = self.notify.notified();
            if let Some(event) = self.queue.pop() {
                return event;
            }
            notified.await;
        }
    }

    /// Wait for the next event with a timeout.
    pub async fn recv_timeout(&self, timeout: Duration) -> Option<DexEvent> {
        loop {
            let notified = self.notify.notified();
            if let Some(event) = self.queue.pop() {
                return Some(event);
            }
            if tokio::time::timeout(timeout, notified).await.is_err() {
                return None;
            }
        }
    }
}
