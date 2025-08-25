use std::collections::HashMap;
// /cluster/command_bus.rs
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio::time::{timeout, Duration};
use crate::cluster::core::Cluster;
use crate::cluster::command::ClusterCommand;
use uuid::Uuid;

pub struct ClusterConfig {
    pub queue_cap: usize,          // p.ex. 100
    pub max_concurrency: usize,    // p.ex. 5
    pub heartbeat_interval_s: u64, // p.ex. 5
    pub heartbeat_timeout_s: u64,  // p.ex. 5
}

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: Uuid,
    pub status: JobStatus,
    pub command_dbg: String,
    pub enqueued_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
    pub err_msg: Option<String>,
}

impl JobInfo {
    fn new(id: Uuid, command_dbg: String) -> Self {
        Self {
            id,
            status: JobStatus::Pending,
            command_dbg,
            enqueued_at: SystemTime::now(),
            started_at: None,
            finished_at: None,
            err_msg: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    TimedOut,
}

#[derive(Debug)]
struct JobEnvelope {
    id: Uuid,
    cmd: ClusterCommand,
}

pub struct CommandBus {
    tx: mpsc::Sender<JobEnvelope>,
    jobs: Arc<Mutex<HashMap<Uuid, JobInfo>>>, 
}

impl CommandBus {
    pub fn new(cluster: &Arc<Cluster>, queue_cap: usize, max_concurrency: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<JobEnvelope>(queue_cap);
        let sem = Arc::new(Semaphore::new(max_concurrency));
        let jobs: Arc<Mutex<HashMap<Uuid, JobInfo>>> = Arc::new(Mutex::new(HashMap::new()));

        tokio::spawn({
            let cluster = Arc::clone(&cluster);
            let sem = Arc::clone(&sem);
            let jobs = Arc::clone(&jobs);
        
            async move {
                while let Some(JobEnvelope { id, cmd }) = rx.recv().await {
                    let Ok(permit) = sem.clone().acquire_owned().await else {
                        let mut j = jobs.lock().await;
                        if let Some(info) = j.get_mut(&id) {
                            info.status = JobStatus::Failed;
                            info.finished_at = Some(SystemTime::now());
                            info.err_msg = Some("Semaphore closed".into());
                        }
                        continue;
                    };
        
                    {
                        let mut j = jobs.lock().await;
                        if let Some(info) = j.get_mut(&id) {
                            info.status = JobStatus::Running;
                            info.started_at = Some(SystemTime::now());
                        }
                    }
        
                    let jobs_ref = Arc::clone(&jobs);
                    let cluster_ref = Arc::clone(&cluster);
        
                    tokio::spawn(async move {
                        // execute por referência (A: &Arc<Cluster>, B: &Cluster)
                        let fut = cmd.execute(&cluster_ref); // ou: cmd.execute(cluster_ref.as_ref())
        
                        match timeout(Duration::from_secs(60), fut).await {
                            Err(_) => {
                                let mut j = jobs_ref.lock().await;
                                if let Some(info) = j.get_mut(&id) {
                                    info.status = JobStatus::TimedOut;
                                    info.finished_at = Some(SystemTime::now());
                                    info.err_msg = Some(format!("Job {id} timeout (60s)"));
                                }
                            }
                            Ok(Err(e)) => {
                                let mut j = jobs_ref.lock().await;
                                if let Some(info) = j.get_mut(&id) {
                                    info.status = JobStatus::Failed;
                                    info.finished_at = Some(SystemTime::now());
                                    info.err_msg = Some(e);
                                }
                            }
                            Ok(Ok(())) => {
                                let mut j = jobs_ref.lock().await;
                                if let Some(info) = j.get_mut(&id) {
                                    info.status = JobStatus::Completed;
                                    info.finished_at = Some(SystemTime::now());
                                }
                            }
                        }
        
                        drop(permit);
                    });
                }
            }
        });

        Self { tx, jobs }
    }

    pub async fn enqueue(&self, cmd: ClusterCommand) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
    
        // registra Pending
        {
            let mut j = self.jobs.lock().await;
            j.insert(id, JobInfo::new(id, format!("{:?}", cmd)));
        }
    
        // envia
        let env = JobEnvelope { id, cmd };
        match self.tx.send(env).await {
            Ok(()) => Ok(id),
            Err(e) => {
                // rollback do registro
                let mut j = self.jobs.lock().await;
                j.remove(&id);
                Err(format!("enqueue failed: {e}"))
            }
        }
    }

    pub async fn try_enqueue(&self, cmd: ClusterCommand) -> Result<Uuid, ClusterCommand> {
        let id = Uuid::new_v4();
    
        // registra Pending
        {
            let mut j = self.jobs.lock().await;
            j.insert(id, JobInfo::new(id, format!("{:?}", cmd)));
        }
    
        // tenta enviar sem bloquear a fila (try_send)
        let env = JobEnvelope { id, cmd: cmd.clone() };
        match self.tx.try_send(env) {
            Ok(()) => Ok(id),
            Err(mpsc::error::TrySendError::Full(env)) |
            Err(mpsc::error::TrySendError::Closed(env)) => {
                // rollback do registro
                let mut j = self.jobs.lock().await;
                j.remove(&id);
                Err(env.cmd)
            }
        }
    }

    pub async fn list_jobs(&self) -> Vec<JobInfo> { 
        self.jobs.lock().await.values().cloned().collect()
    }

    pub async fn list_pending_jobs(&self) -> Vec<JobInfo> {
        self.jobs.lock().await
            .values()
            .filter(|j| j.status == JobStatus::Pending)
            .cloned()
            .collect()
    }
}
// opcional: Clone
impl Clone for CommandBus {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone(), jobs: Arc::clone(&self.jobs) }
    }
}