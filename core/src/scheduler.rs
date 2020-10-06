use bitflags::_core::cmp::Ordering;
use binary_heap_plus::{BinaryHeap, MinComparator};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
pub enum EventType {
    NONE        = 255,
    VBLANK      = 0,
    OamSearch   = 1,
    LcdTransfer = 2,
    HBLANK      = 3,
    VblankWait = 4,
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct Event {
    pub timestamp: u64,
    pub event_type: EventType,
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
    pub current_time: u64,
}

impl Scheduler {
    #[inline]
    pub fn new() -> Self {
        let mut result = Self{ event_queue: BinaryHeap::with_capacity_min(64), current_time: 0 };
        result.event_queue.push(Event{ timestamp: 0, event_type: EventType::NONE });
        result
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
    pub fn push_event(&mut self, event_type: EventType, timestamp: u64) {
        self.event_queue.push(Event{ timestamp, event_type });
    }

    pub fn remove_event_type(&mut self, event_type: EventType) {
        self.event_queue = BinaryHeap::from_vec(self.event_queue.clone().into_iter().filter(|e| e.event_type != event_type).collect());
    }

    #[inline]
    pub fn add_cycles(&mut self, delta_cycles: u64) {
        self.current_time += delta_cycles;
    }
}