use binary_heap_plus::{BinaryHeap, MinComparator};
use bitflags::_core::cmp::Ordering;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
#[repr(u8)]
pub enum EventType {
    NONE        = 255,
    VBLANK      = 0,
    OamSearch   = 1,
    LcdTransfer = 2,
    HBLANK      = 3,
    VblankWait  = 4,
    APUFrameSequencer = 5,
    APUSample = 6,
    TimerOverflow = 7,
    TimerPostOverflow = 8,
    DMATransferComplete = 9,
    DMARequested = 10,
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

impl Event {
    /// Update the current event with new data.
    ///
    /// `delta_timestamp` will add the given time to the current `Event`'s `timestamp`.
    pub fn update_self(mut self, new_event_type: EventType, delta_timestamp: u64) -> Self {
        self.timestamp += delta_timestamp;
        self.event_type = new_event_type;
        self
    }
}

#[derive(Debug)]
pub struct Scheduler {
    // Want the smallest timestamp first, so MinComparator
    event_queue: BinaryHeap<Event, MinComparator>,
    pub current_time: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut result = Self {
            event_queue: BinaryHeap::with_capacity_min(64),
            current_time: 0,
        };
        result.event_queue.push(Event {
            timestamp: 0,
            event_type: EventType::NONE,
        });
        result
    }

    /// Returns a `Some(&Event)` if there is an event available which has a timestamp
    /// which is at or below the `current_time` for the `Scheduler`
    pub fn pop_closest(&mut self) -> Option<Event> {
        if let Some(event) = self.event_queue.peek() {
            if event.timestamp <= self.current_time {
                return self.event_queue.pop();
            }
        }
        None
    }

    /// Add a new event to the `Scheduler`.
    pub fn push_event(&mut self, event_type: EventType, timestamp: u64) {
        self.event_queue.push(Event { timestamp, event_type });
    }

    pub fn push_relative(&mut self, event_type: EventType, relative_timestamp: u64) {
        self.event_queue.push(Event {
            timestamp: self.current_time + relative_timestamp,
            event_type,
        });
    }

    /// Add an event to the `Scheduler`.
    /// This function is best used when we want to avoid an allocation for a new event,
    /// say in the `pop_closest()` loop for the scheduler. Instead we can then reuse that event
    /// and push it back in here.
    pub fn push_full_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    pub fn remove_event_type(&mut self, event_type: EventType) {
        // Very inefficient way of doing this, but until we start needing to do more dynamic
        // removal of events it doesn't really matter.
        self.event_queue = BinaryHeap::from_vec(
            self.event_queue
                .clone()
                .into_iter()
                .filter(|e| e.event_type != event_type)
                .collect(),
        );
    }

    #[inline]
    pub fn add_cycles(&mut self, delta_cycles: u64) {
        self.current_time += delta_cycles;
    }
}
