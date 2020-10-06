use bitflags::_core::cmp::Ordering;
use binary_heap_plus::{BinaryHeap, MinComparator};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
enum EventType {
    NONE        = 255,
    VBLANK      = 0,
    OamSearch   = 1,
    LcdTransfer = 2,
    HBLANK      = 3,
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct Event {
    timestamp: u64,
    event_type: EventType,
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

#[derive(Debug)]
pub struct Scheduler {
    // Want the smallest timestamp first, so MinComparator
    event_queue: BinaryHeap<Event, MinComparator>,
    current_time: u64,
}

impl Scheduler {
    #[inline]
    pub fn new() -> Self {
        Self{ event_queue: BinaryHeap::with_capacity_min(64), current_time: 0 }
    }

    /// Returns a `Some(&Event)` if there is an event available which has a timestamp
    /// which is at or below the `current_time` for the `Scheduler`
    #[inline]
    pub fn pop_closest(&mut self) -> Option<Event> {
         if let Some(event) = self.event_queue.peek() {
             if event.timestamp <= self.current_time {
                 return self.event_queue.pop();
             }
         }
        None
    }

    /// Add an event to the `Scheduler`.
    #[inline]
    pub fn push_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    #[inline]
    pub fn add_cycles(&mut self, delta_cycles: u64) {
        self.current_time += delta_cycles;
    }
}