use binary_heap_plus::{BinaryHeap, MinComparator};
use bitflags::_core::cmp::Ordering;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
#[repr(u8)]
pub enum EventType {
    None = 255,
    Vblank = 0,
    OamSearch = 1,
    LcdTransfer = 2,
    Hblank = 3,
    VblankWait = 4,
    TimerOverflow = 7,
    TimerPostOverflow = 8,
    TimerTick = 9,
    DMARequested = 10,
    DMATransferComplete = 11,
    GDMARequested = 12,
    GDMATransferComplete = 13,
    Y153TickToZero = 14,
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
            event_type: EventType::None,
        });
        result
    }

    /// Returns a `Some(&Event)` if there is an event available which has a timestamp
    /// which is at or below the `current_time` for the `Scheduler`
    #[inline(always)]
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
        self.add_event(Event { timestamp, event_type });
    }

    pub fn push_relative(&mut self, event_type: EventType, relative_timestamp: u64) {
        self.add_event(Event {
            timestamp: self.current_time + relative_timestamp,
            event_type,
        });
    }

    /// Add an event to the `Scheduler`.
    /// This function is best used when we want to avoid an allocation for a new event,
    /// say in the `pop_closest()` loop for the scheduler. Instead we can then reuse that event
    /// and push it back in here.
    pub fn push_full_event(&mut self, event: Event) {
        self.add_event(event)
    }

    #[inline(always)]
    fn add_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }

    /// Update a specific event type to be scheduled at `current_time + relative_timestamp`
    /// instead of whatever time it had before.
    ///
    /// Currently VERY inefficient due to lacking binary heap implementation
    pub fn update_event_type(&mut self, event_type: EventType, relative_timestamp: u64) {
        self.remove_event_type(event_type);
        self.push_relative(event_type, relative_timestamp);
    }

    pub fn remove_event_type(&mut self, event_type: EventType) {
        // Very inefficient way of doing this, but until we start needing to do more dynamic
        // removal of events it doesn't really matter.
        let mut current_vec = std::mem::replace(&mut self.event_queue, BinaryHeap::new_min()).into_vec();
        current_vec.retain(|e| e.event_type != event_type);
        self.event_queue = BinaryHeap::from_vec(current_vec);
    }

    #[inline]
    pub fn add_cycles(&mut self, delta_cycles: u64) {
        self.current_time += delta_cycles;
    }
}
