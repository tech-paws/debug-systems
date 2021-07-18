use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::thread;

use lazy_static::lazy_static;

pub const PERFORMANCE_RECORDS_CAPACITY: usize = 512;
pub const PERFORMANCE_COUNTER_LOG_SIZE: usize = 120; // max entires
pub const PERFORMANCE_COUNTER_STATE_SIZE: usize = 60; // max entires

lazy_static! {
    pub static ref PROFILE_STATE: Mutex<ProfileState> = Mutex::new(ProfileState::default());
}

pub struct ProfileState {
    pub snapshot_interval: usize,
    pub frame_timer: Instant,
    pub frame_elapsed: Duration,
    pub frame_counter: usize,
    pub snapshot_counter: usize,
    pub performance_counter_states: Vec<PerformanceCounterState>,
    pub performance_counter_log: Vec<PerformanceCounterStatistics>,
    pub timed_blocks: HashMap<u64, TimedBlock>,
    // TODO: Make proper id managment
    pub last_timed_block_id: u64,
}

impl Default for ProfileState {
    fn default() -> Self {
        let snapshot_interval = 3;

        ProfileState {
            frame_counter: 0,
            snapshot_counter: 0,
            snapshot_interval,
            performance_counter_states: vec![
                PerformanceCounterState::default();
                PERFORMANCE_COUNTER_STATE_SIZE
            ],
            performance_counter_log: vec![
                PerformanceCounterStatistics::default();
                PERFORMANCE_COUNTER_LOG_SIZE
            ],
            frame_timer: Instant::now(),
            frame_elapsed: Duration::from_nanos(0),
            timed_blocks: HashMap::new(),
            last_timed_block_id: 0,
        }
    }
}

#[derive(Clone)]
pub struct PerformanceCounterState {
    pub records: Vec<ClocsDebugRecord>,
}

#[derive(Clone, Default, Debug)]
pub struct PerformanceCounterStatisticsRecord {
    pub name: &'static str,
    pub file_name: &'static str,
    pub line: u32,
    pub sum_elapsed: Duration,
    pub sum_hits: u32,
    pub sum_hits_over_elapsed: u128,
    pub hits: u32,
    pub percent: f32,
    pub thread_id: String,
}

impl Default for PerformanceCounterState {
    fn default() -> Self {
        PerformanceCounterState {
            records: Vec::with_capacity(PERFORMANCE_RECORDS_CAPACITY),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PerformanceCounterStatistics {
    pub records: Vec<PerformanceCounterStatisticsRecord>,
}

impl Default for PerformanceCounterStatistics {
    fn default() -> Self {
        PerformanceCounterStatistics {
            records: Vec::with_capacity(PERFORMANCE_RECORDS_CAPACITY),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClocsDebugRecord {
    pub name: &'static str,
    pub file_name: &'static str,
    pub line: u32,
    pub elapsed: Duration,
    pub hits: u32,
    pub thread_id: thread::ThreadId,
}

impl Default for ClocsDebugRecord {
    fn default() -> Self {
        ClocsDebugRecord {
            name: "",
            file_name: "",
            line: 0,
            elapsed: Duration::from_nanos(0),
            hits: 0,
            thread_id: thread::current().id(),
        }
    }
}

#[derive(Clone)]
pub struct TimedBlock {
    manual_drop: bool,
    pub thread_id: thread::ThreadId,
    pub name: &'static str,
    pub file_name: &'static str,
    pub line: u32,
    pub timer: Instant,
}

impl TimedBlock {
    pub fn new(name: &'static str, file_name: &'static str, line: u32) -> TimedBlock {
        TimedBlock {
            name,
            file_name,
            line,
            manual_drop: false,
            thread_id: thread::current().id(),
            timer: Instant::now(),
        }
    }
}

impl Drop for TimedBlock {
    fn drop(&mut self) {
        if !self.manual_drop {
            drop_timed_block(self, &mut get_profile_state());
        }
    }
}

#[macro_export]
macro_rules! timed_block {
    ($name:expr) => {
        crate::debug_services::profile::TimedBlock::new($name, file!(), line!())
    };
}

pub fn get_profile_state<'a>() -> MutexGuard<'a, ProfileState> {
    PROFILE_STATE.lock().expect("failed to get profile state")
}

pub fn push_timed_block(name: &'static str, file_name: &'static str, line: u32) -> u64 {
    let profile = &mut get_profile_state();

    let block = TimedBlock {
        name,
        file_name,
        line,
        manual_drop: true,
        thread_id: thread::current().id(),
        timer: Instant::now(),
    };

    let id = profile.last_timed_block_id;

    profile.last_timed_block_id += 1;
    profile.timed_blocks.insert(id, block);

    id
}

pub fn drop_timed_block_by_id(id: u64) {
    let profile = &mut get_profile_state();
    let block = match profile.timed_blocks.get(&id) {
        Some(value) => value.clone(),
        None => {
            log::warn!("Couldn't drop block by id: {}", id);
            return;
        }
    };

    drop_timed_block(&block, profile);
    profile.timed_blocks.remove(&id);
}

pub fn drop_timed_block(timed_block: &TimedBlock, profile_state: &mut MutexGuard<ProfileState>) {
    let mut hits = 1;
    let mut elapsed = timed_block.timer.elapsed();
    let mut to_modify = false;
    let mut modify_idx: usize = 0;

    let frame_counter = profile_state.frame_counter;
    let records = &mut profile_state.performance_counter_states[frame_counter].records;

    // NOTE(sysint64): Right now this method is faster than Map
    for (i, c) in records.iter().enumerate() {
        if c.name == timed_block.name
            && c.file_name == timed_block.file_name
            && c.line == timed_block.line
        {
            hits += c.hits;
            elapsed += c.elapsed;
            to_modify = true;
            modify_idx = i;
        }
    }

    if to_modify {
        records[modify_idx] = ClocsDebugRecord {
            name: timed_block.name,
            file_name: timed_block.file_name,
            line: timed_block.line,
            thread_id: thread::current().id(),
            elapsed,
            hits,
        };
    }
    else {
        records.push(ClocsDebugRecord {
            name: timed_block.name,
            file_name: timed_block.file_name,
            line: timed_block.line,
            thread_id: thread::current().id(),
            elapsed,
            hits,
        });
    }
}

pub fn frame_start(profile_state: &mut MutexGuard<ProfileState>) {
    profile_state.frame_timer = Instant::now();
}

pub fn frame_end(profile_state: &mut MutexGuard<ProfileState>) {
    profile_state.frame_counter += 1;
    profile_state.frame_elapsed = profile_state.frame_timer.elapsed();

    let snapshot_interval = profile_state.snapshot_interval;

    if profile_state.frame_counter >= snapshot_interval {
        take_snapshot(profile_state);
        profile_state.frame_counter = 0;

        for i in 0..snapshot_interval {
            profile_state.performance_counter_states[i] = PerformanceCounterState::default();
        }
    }
}

fn take_snapshot(profile_state: &mut MutexGuard<ProfileState>) {
    profile_state.snapshot_counter += 1;

    if profile_state.snapshot_counter >= PERFORMANCE_COUNTER_LOG_SIZE {
        profile_state.snapshot_counter = 0;
    }

    let mut statistics: HashMap<String, PerformanceCounterStatisticsRecord> = HashMap::new();

    for state in profile_state.performance_counter_states.iter() {
        for record in state.records.iter() {
            let key = String::from(record.name) + record.file_name + &record.line.to_string();
            let element = statistics.entry(key).or_default();

            element.name = record.name;
            element.file_name = record.file_name;
            element.line = record.line;
            element.sum_elapsed += record.elapsed;
            element.sum_hits += record.hits;
            element.sum_hits_over_elapsed += record.elapsed.as_nanos() / record.hits as u128;
            element.hits += 1;
            element.thread_id = format!("{:?}", record.thread_id);
        }
    }

    let total_elapsed: u128 = statistics
        .values()
        .map(|record| record.sum_elapsed.as_nanos())
        .sum();

    for record in statistics.values_mut() {
        record.percent =
            (record.sum_elapsed.as_nanos() as f64 / total_elapsed as f64) as f32 * 100.0;
    }

    let mut records: Vec<PerformanceCounterStatisticsRecord> =
        statistics.values().cloned().collect();

    records.sort_by(|a, b| b.percent.partial_cmp(&a.percent).unwrap());

    let counter = profile_state.snapshot_counter;
    let snapshot = &mut profile_state.performance_counter_log[counter].records;

    snapshot.clear();
    snapshot.append(&mut records);
}

pub fn update_snapshot_interval(profile_state: &mut MutexGuard<ProfileState>, new_interval: usize) {
    if new_interval <= PERFORMANCE_COUNTER_STATE_SIZE {
        profile_state.snapshot_interval = new_interval;
    }
}
