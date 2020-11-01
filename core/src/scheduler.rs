use std::cmp::{Ordering, Reverse};
use std::collections::VecDeque;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EventType {
    NONE = 255,
    VBLANK = 0,
    OamSearch = 1,
    LcdTransfer = 2,
    HBLANK = 3,
    VblankWait = 4,
    APUFrameSequencer = 5,
    APUSample = 6,
    TimerOverflow = 7,
    TimerPostOverflow = 8,
    TimerTick = 9,
    DMARequested = 10,
    DMATransferComplete = 11,
    GDMARequested = 12,
    GDMATransferComplete = 13,
    Y153TickToZero = 14,
}

#[derive(Debug, Copy, Clone, Eq, Hash, PartialOrd, PartialEq)]
pub struct Event {
    pub timestamp: u64,
    pub event_type: EventType,
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
    event_queue: VecDeque<Event>,
    pub current_time: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut result = Self {
            event_queue: VecDeque::with_capacity(20),
            current_time: 0,
        };
        result.event_queue.push_front(Event {
            timestamp: 0,
            event_type: EventType::NONE,
        });
        result
    }

    /// Returns a `Some(&Event)` if there is an event available which has a timestamp
    /// which is at or below the `current_time` for the `Scheduler`
    pub fn pop_closest(&mut self) -> Option<Event> {
        if let Some(event) = self.event_queue.front() {
            if event.timestamp <= self.current_time {
                return self.event_queue.pop_front();
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

    fn add_event(&mut self, event: Event) {
        if let Some(back_event) = self.event_queue.back() {
            // If the time it happens is later than the latest we currently have, just insert at the back
            if back_event.timestamp <= event.timestamp {
                self.event_queue.push_back(event);
            } else {
                let mut place_to_insert= self.event_queue.len()-1;
                for (i, event_iter) in self.event_queue.iter().enumerate() {
                    if event_iter.timestamp >= event.timestamp {
                        place_to_insert = i;
                        break;
                    }
                }
                self.event_queue.insert(place_to_insert, event);
            }
        } else {
            self.event_queue.push_front(event);
        }
    }

    pub fn remove_event_type(&mut self, event_type: EventType) {
        self.event_queue.retain(|event| event.event_type != event_type);
    }

    #[inline]
    pub fn add_cycles(&mut self, delta_cycles: u64) {
        self.current_time += delta_cycles;
    }
}
