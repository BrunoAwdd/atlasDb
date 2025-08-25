use std::{sync::Arc, time::Duration};
use tokio::time::{interval, Instant};
use uuid::Uuid;

use crate::jobs::bus::CommandBus;
use crate::jobs::types::{JobKind, ScheduleSpec, ScheduledJob, JobMap};

pub struct Scheduler {
    pub(crate) jobs: JobMap,
}

impl Scheduler {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { jobs: JobMap::default() })
    }

    pub async fn enqueue_once(self: &Arc<Self>, cmd: crate::cluster::command::ClusterCommand) -> Uuid {
        self.enqueue_after(Duration::from_secs(0), cmd).await
    }

    pub async fn enqueue_after(self: &Arc<Self>, delay: Duration, cmd: crate::cluster::command::ClusterCommand) -> Uuid {
        let id = Uuid::new_v4();
        let job = ScheduledJob {
            id,
            cmd,
            spec: ScheduleSpec { kind: JobKind::OneOff, max_retries: 0, backoff_base: Duration::from_secs(0) },
            next_run: Instant::now() + delay,
            active: true,
            retries: 0,
        };
        let mut map = self.jobs.lock().await;
        map.insert(id, job);
        id
    }

    // ✅ agora com jitter e FixedInterval correto
    pub async fn enqueue_every(
        self: &Arc<Self>,
        interval: Duration,
        jitter: Option<Duration>,
        cmd: crate::cluster::command::ClusterCommand,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let job = ScheduledJob {
            id,
            cmd,
            spec: ScheduleSpec { kind: JobKind::FixedInterval { interval, jitter }, max_retries: 0, backoff_base: Duration::from_secs(1) },
            next_run: Instant::now() + interval,
            active: true,
            retries: 0,
        };
        let mut map = self.jobs.lock().await;
        map.insert(id, job);
        id
    }

    pub async fn cancel(&self, id: Uuid) -> bool {
        let mut map = self.jobs.lock().await;
        if let Some(j) = map.get_mut(&id) { j.active = false; return true; }
        false
    }

    pub async fn update_interval(&self, id: Uuid, new_interval: Duration, new_jitter: Option<Duration>) -> bool {
        let mut map = self.jobs.lock().await;
        if let Some(j) = map.get_mut(&id) {
            if let JobKind::FixedInterval { .. } = j.spec.kind {
                j.spec.kind = JobKind::FixedInterval { interval: new_interval, jitter: new_jitter };
                j.next_run = Instant::now() + new_interval;
                return true;
            }
        }
        false
    }

    async fn take_due(&self) -> Vec<ScheduledJob> {
        let now = Instant::now();
        let mut map = self.jobs.lock().await;
        let ready_ids: Vec<_> = map.iter().filter_map(|(id, j)| (j.active && now >= j.next_run).then_some(*id)).collect();
        let mut due = Vec::with_capacity(ready_ids.len());
        for id in ready_ids { if let Some(j) = map.remove(&id) { due.push(j) } }
        due
    }

    async fn reinsert(&self, job: ScheduledJob) {
        let mut map = self.jobs.lock().await;
        map.insert(job.id, job);
    }
}

pub fn spawn_scheduler(bus: CommandBus) -> Arc<Scheduler> {
    let sched = Scheduler::new();
    let s = Arc::clone(&sched);

    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(200));
        loop {
            tick.tick().await;
            let mut due = s.take_due().await;

            for mut job in due.drain(..) {
                let cmd = job.cmd.clone();
                let bus_clone = bus.clone();
                tokio::spawn(async move {
                    let _ = bus_clone.enqueue(cmd).await;
                });

                job.bump_next_run();
                if job.active { s.reinsert(job).await; }
            }
        }
    });

    sched
}
