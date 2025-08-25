use std::{time::Duration, collections::HashMap};
use rand::{thread_rng, Rng};
use tokio::{sync::Mutex, time::{Instant}};
use uuid::Uuid;
use crate::cluster::command::ClusterCommand;

#[derive(Debug, Clone)]
pub enum JobKind {
    OneOff,
    FixedInterval { interval: Duration, jitter: Option<Duration> },
}

#[derive(Debug, Clone)]
pub struct ScheduleSpec {
    pub kind: JobKind,
    pub max_retries: u32,
    pub backoff_base: Duration,
}

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub id: Uuid,
    pub cmd: ClusterCommand,
    pub spec: ScheduleSpec,
    pub next_run: Instant,
    pub active: bool,
    pub retries: u32,
}

impl ScheduledJob {
    pub fn bump_next_run(&mut self) {
        match self.spec.kind {
            JobKind::OneOff => self.active = false,
            JobKind::FixedInterval { interval, jitter } => {
                let mut next = Instant::now() + interval;
                if let Some(j) = jitter {
                    // jitter ∈ [0, j]
                    let max_ms = j.as_millis() as u128;
                    let extra_ms = if max_ms == 0 { 0 } else { thread_rng().gen_range(0..=max_ms) } as u64;
                    next += Duration::from_millis(extra_ms);
                }
                self.next_run = next;
            }
        }
    }
}

// (opcional) estado compartilhado se quiser externar depois
pub type JobMap = Mutex<HashMap<Uuid, ScheduledJob>>;
