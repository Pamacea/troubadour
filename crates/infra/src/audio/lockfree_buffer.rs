//! Lock-free ring buffer for real-time audio processing
//!
//! This implementation uses crossbeam's atomic utilities for wait-free
//! synchronization between the audio thread and the main thread.
//!
//! Performance characteristics:
//! - Lock-free (no mutex contention)
//! - Wait-free for single producer/consumer
//! - Cache-friendly sequential access
//! - No allocations in hot path

use crossbeam::utils::CachePadded;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Lock-free single-producer single-consumer ring buffer
///
/// Uses cache-padded counters to prevent false sharing between cores.
pub struct LockFreeRingBuffer {
    /// Buffer storage (heap-allocated for size)
    buffer: Vec<f32>,

    /// Write position (cache-padded to prevent false sharing)
    write_pos: Arc<CachePadded<AtomicUsize>>,

    /// Read position (cache-padded to prevent false sharing)
    read_pos: Arc<CachePadded<AtomicUsize>>,

    /// Buffer capacity (must be power of 2 for fast modulo)
    capacity: usize,

    /// Mask for fast modulo operation (capacity - 1)
    mask: usize,
}

impl LockFreeRingBuffer {
    /// Create a new lock-free ring buffer
    ///
    /// Capacity will be rounded up to the next power of 2 for efficiency.
    pub fn with_capacity(mut capacity: usize) -> Self {
        // Round up to next power of 2
        if !capacity.is_power_of_two() {
            capacity = capacity.next_power_of_two();
        }

        Self {
            buffer: vec![0.0; capacity],
            write_pos: Arc::new(CachePadded::new(AtomicUsize::new(0))),
            read_pos: Arc::new(CachePadded::new(AtomicUsize::new(0))),
            capacity,
            mask: capacity - 1,
        }
    }

    /// Write samples to the buffer (producer)
    ///
    /// Returns the number of samples actually written.
    /// This is lock-free and wait-free.
    pub fn write(&mut self, samples: &[f32]) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);

        // Calculate available space
        let available = self.available_write_internal(write_pos, read_pos);
        let to_write = samples.len().min(available);

        // Write samples in a single pass
        for i in 0..to_write {
            let pos = (write_pos + i) & self.mask;
            unsafe {
                // SAFETY: We've calculated available space correctly
                *self.buffer.get_unchecked_mut(pos) = samples[i];
            }
        }

        // Update write position (release semantics ensures writes are visible)
        self.write_pos.store(write_pos + to_write, Ordering::Release);

        to_write
    }

    /// Read samples from the buffer (consumer)
    ///
    /// Returns the number of samples actually read.
    /// This is lock-free and wait-free.
    pub fn read(&self, buffer: &mut [f32]) -> usize {
        let read_pos = self.read_pos.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);

        // Calculate available samples
        let available = self.available_read_internal(read_pos, write_pos);
        let to_read = buffer.len().min(available);

        // Read samples in a single pass
        for i in 0..to_read {
            let pos = (read_pos + i) & self.mask;
            unsafe {
                // SAFETY: We've calculated available samples correctly
                buffer[i] = *self.buffer.get_unchecked(pos);
            }
        }

        // Update read position (release semantics)
        self.read_pos.store(read_pos + to_read, Ordering::Release);

        to_read
    }

    /// Get available write space (internal version with known positions)
    #[inline]
    fn available_write_internal(&self, write_pos: usize, read_pos: usize) -> usize {
        // One slot is kept empty to distinguish full from empty
        if write_pos >= read_pos {
            self.capacity - (write_pos - read_pos) - 1
        } else {
            read_pos - write_pos - 1
        }
    }

    /// Get available read samples (internal version with known positions)
    #[inline]
    fn available_read_internal(&self, read_pos: usize, write_pos: usize) -> usize {
        if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            self.capacity - (read_pos - write_pos)
        }
    }

    /// Get available write space
    pub fn available_write(&self) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        self.available_write_internal(write_pos, read_pos)
    }

    /// Get available read samples
    pub fn available_read(&self) -> usize {
        let read_pos = self.read_pos.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        self.available_read_internal(read_pos, write_pos)
    }

    /// Clear the buffer (reset positions)
    pub fn clear(&self) {
        self.write_pos.store(0, Ordering::Release);
        self.read_pos.store(0, Ordering::Release);
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        let read_pos = self.read_pos.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        read_pos == write_pos
    }

    /// Get current fill level
    pub fn len(&self) -> usize {
        let read_pos = self.read_pos.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            self.capacity - (read_pos - write_pos)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfree_basic() {
        let mut buffer = LockFreeRingBuffer::with_capacity(16);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let mut output = vec![0.0; 4];

        assert_eq!(buffer.write(&input), 4);
        assert_eq!(buffer.available_read(), 4);
        assert_eq!(buffer.read(&mut output), 4);
        assert_eq!(output, input);
    }

    #[test]
    fn test_lockfree_wraparound() {
        let mut buffer = LockFreeRingBuffer::with_capacity(8);

        // Write 6 samples
        let input1 = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert_eq!(buffer.write(&input1), 6);

        // Read 4 samples
        let mut output1 = vec![0.0; 4];
        assert_eq!(buffer.read(&mut output1), 4);
        assert_eq!(output1, vec![1.0, 2.0, 3.0, 4.0]);

        // Write 6 more samples (should wrap around)
        let input2 = vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
        assert_eq!(buffer.write(&input2), 5); // Only 5 slots available

        // Read remaining
        let mut output2 = vec![0.0; 10];
        assert_eq!(buffer.read(&mut output2), 7);
        assert_eq!(output2[..7], vec![5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]);
    }

    #[test]
    fn test_lockfree_capacity_rounding() {
        // Capacity should be rounded up to power of 2
        let buffer = LockFreeRingBuffer::with_capacity(10);
        assert_eq!(buffer.capacity(), 16);
    }

    #[test]
    fn test_lockfree_empty_full() {
        let mut buffer = LockFreeRingBuffer::with_capacity(8);

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);

        let input = vec![1.0; 7];
        buffer.write(&input);

        assert_eq!(buffer.len(), 7);
    }
}
