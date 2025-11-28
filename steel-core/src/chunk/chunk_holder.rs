//! `ChunkHolder` manages chunk state and asynchronous generation tasks.
use futures::Future;
use parking_lot::{Mutex as ParkingMutex, RwLock as ParkingRwLock};
use replace_with::replace_with_or_abort;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use steel_utils::ChunkPos;
use tokio::select;
use tokio::sync::{oneshot, watch};

use crate::chunk::chunk_generation_task::{NeighborReady, StaticCache2D};
use crate::chunk::chunk_level::ChunkLevel;
use crate::{
    ChunkMap,
    chunk::{
        chunk_access::{ChunkAccess, ChunkStatus},
        chunk_generation_task::ChunkGenerationTask,
        chunk_pyramid::ChunkStep,
        level_chunk::LevelChunk,
    },
};

const STATUS_NONE: u8 = u8::MAX;

/// The result of a chunk operation.
pub enum ChunkResult {
    /// The chunk is not loaded.
    Unloaded,
    /// The chunk operation failed.
    Failed,
    /// The chunk operation succeeded.
    Ok(ChunkStatus),
}

/// Holds a chunk in a watch channel, allowing for concurrent access and state tracking.
///
/// NOTICE: It is very important to keep data and `chunk_result` in sync.
///
/// `ChunkResult::Unloaded` -> data is None
///
/// `ChunkResult::Failed` -> data is Anything and should not be used anymore
///
/// `ChunkResult::Ok(status except Full)` -> data is `Some(ChunkAccess::Proto(ProtoChunk))`
///
/// `ChunkResult::Ok(ChunkStatus::Full)` -> data is `Some(ChunkAccess::Full(LevelChunk))`
pub struct ChunkHolder {
    data: ParkingRwLock<Option<ChunkAccess>>,
    chunk_result: watch::Receiver<ChunkResult>,
    sender: watch::Sender<ChunkResult>,
    generation_task: ParkingMutex<Option<Arc<ChunkGenerationTask>>>,
    pos: ChunkPos,
    /// The current ticket level of the chunk.
    pub ticket_level: AtomicU8,
    /// The highest status that has started work.
    started_work: AtomicUsize,
    /// The highest status that generation is allowed to reach.
    highest_allowed_status: watch::Receiver<u8>,
    highest_allowed_status_sender: watch::Sender<u8>,
}

impl ChunkHolder {
    /// Gets the chunk position.
    pub fn get_pos(&self) -> ChunkPos {
        self.pos
    }

    /// Creates a new chunk holder.
    #[must_use]
    pub fn new(pos: ChunkPos, ticket_level: u8) -> Self {
        let (sender, receiver) = watch::channel(ChunkResult::Unloaded);
        let (highest_allowed_status_sender, highest_allowed_status_receiver) = watch::channel(
            ChunkLevel::generation_status(ticket_level)
                .map_or(STATUS_NONE, |s| s.get_index() as u8),
        );

        Self {
            data: ParkingRwLock::new(None),
            chunk_result: receiver,
            sender,
            generation_task: ParkingMutex::new(None),
            pos,
            ticket_level: AtomicU8::new(ticket_level),
            started_work: AtomicUsize::new(usize::MAX),
            highest_allowed_status: highest_allowed_status_receiver,
            highest_allowed_status_sender,
        }
    }

    /// Updates the highest allowed generation status based on the ticket level.
    pub fn update_highest_allowed_status(&self, ticket_level: u8) {
        let new_status = ChunkLevel::generation_status(ticket_level)
            .map_or(STATUS_NONE, |s| s.get_index() as u8);
        self.highest_allowed_status_sender.send_replace(new_status);
    }

    /// Checks if the given status is disallowed.
    pub fn is_status_disallowed(&self, status: ChunkStatus) -> bool {
        let allowed = *self.highest_allowed_status.borrow();
        if allowed == STATUS_NONE {
            return true;
        }
        status.get_index() > allowed as usize
    }

    /// Returns a future that completes when the chunk reaches the given status or is cancelled.
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    pub(crate) fn schedule_chunk_generation_task_b(
        self: Arc<Self>,
        status: ChunkStatus,
        chunk_map: Arc<ChunkMap>,
    ) {
        if self.is_status_disallowed(status) {
            return;
        }

        if self.try_chunk(status).is_some() {
            return;
        }

        let task = self.generation_task.lock();

        #[allow(clippy::unwrap_used)]
        if task.is_none() || status > task.as_ref().unwrap().target_status {
            drop(task);
            self.reschedule_chunk_task_b(status, chunk_map);
        }
    }

    /// Reschedules the chunk task to the given status.
    pub(crate) fn reschedule_chunk_task_b(&self, status: ChunkStatus, chunk_map: Arc<ChunkMap>) {
        let new_task = chunk_map.schedule_generation_task_b(status, self.pos);
        let mut old_task_guard = self.generation_task.lock();

        let old_task = old_task_guard.replace(new_task);
        drop(old_task_guard);

        if let Some(old_task) = old_task {
            old_task.mark_for_cancel();
        }
    }

    /// Gets access to the chunk if it has reached the given status.
    #[inline]
    pub fn try_chunk(&self, status: ChunkStatus) -> Option<&ParkingRwLock<Option<ChunkAccess>>> {
        match &*self.chunk_result.borrow() {
            ChunkResult::Ok(s) if status <= *s => Some(&self.data),
            _ => None,
        }
    }

    /// Waits until the chunk has reached the given status, then calls the function.
    pub fn await_chunk(
        &self,
        status: ChunkStatus,
    ) -> impl Future<Output = Option<&ParkingRwLock<Option<ChunkAccess>>>> {
        let mut subscriber = self.sender.subscribe();
        let mut status_subscriber = self.highest_allowed_status_sender.subscribe();
        async move {
            loop {
                {
                    let chunk_result = subscriber.borrow_and_update();
                    match &*chunk_result {
                        ChunkResult::Ok(s) if status <= *s => {
                            return Some(&self.data);
                        }
                        ChunkResult::Failed => {
                            return None;
                        }
                        _ => {}
                    }
                }

                if self.is_status_disallowed(status) {
                    return None;
                }

                select! {
                    val = subscriber.changed() => {
                        if val.is_err() {
                            log::error!("Failed to wait for chunk access");
                            return None;
                        }
                    }
                    val = status_subscriber.changed() => {
                        if val.is_err() {
                            log::error!("Failed to wait for highest allowed status");
                            return None;
                        }
                    }
                }
            }
        }
    }

    /// Gets the persisted status of the chunk.
    pub fn persisted_status(&self) -> Option<ChunkStatus> {
        match &*self.chunk_result.borrow() {
            ChunkResult::Ok(s) => Some(*s),
            _ => None,
        }
    }

    /// Applies a step to the chunk.
    ///
    /// # Panics
    /// Panics if the target status is not Empty and has no parent, or if the chunk status is invalid during generation.
    pub fn apply_step(
        self: Arc<Self>,
        step: Arc<ChunkStep>,
        chunk_map: Arc<ChunkMap>,
        cache: Arc<StaticCache2D<Arc<ChunkHolder>>>,
        thread_pool: Arc<rayon::ThreadPool>,
    ) -> NeighborReady {
        let target_status = step.target_status;

        if self.is_status_disallowed(target_status) {
            return Box::pin(async { None });
        }

        if !self.acquire_status_bump(target_status) {
            let self_clone = self.clone();
            return Box::pin(
                async move { self_clone.await_chunk(target_status).await.map(|_| ()) },
            );
        }

        let sender = self.sender.clone();
        let cache = cache.clone();
        let context = chunk_map.world_gen_context.clone();
        // This is one of the `crate::chunk::chunk_status_tasks` functions.
        let task = step.task;
        let self_clone = self.clone();

        let future =
            chunk_map.task_tracker.spawn(async move {
                if target_status == ChunkStatus::Empty {
                    match rayon_spawn(&thread_pool, move || {
                        task(context, &step, &cache, self_clone)
                    })
                    .await
                    {
                        Ok(()) => {
                            sender.send_modify(|chunk| {
                                if let ChunkResult::Ok(s) = chunk {
                                    //log::info!("Task completed for {:?}", target_status);

                                    if *s < target_status {
                                        *s = target_status;
                                    }
                                }
                            });
                            Some(())
                        }
                        Err(e) => {
                            log::error!("Chunk generation task failed: {e}");
                            sender.send_replace(ChunkResult::Failed);
                            None
                        }
                    }
                } else {
                    let parent_status = target_status
                        .parent()
                        .expect("Target status must have parent if not Empty");

                    //log::info!(
                    //    "Parent status: {:?}, target status: {:?}",
                    //    parent_status,
                    //    target_status
                    //);

                    let has_parent = self_clone.try_chunk(parent_status).is_some();

                    assert!(has_parent, "Parent chunk missing");

                    match rayon_spawn(&thread_pool, move || {
                        task(context, &step, &cache, self_clone)
                    })
                    .await
                    {
                        Ok(()) => {
                            sender.send_modify(|chunk| if let ChunkResult::Ok(s) = chunk {
                            if *s < target_status {
                                *s = target_status;
                            } else if *s != ChunkStatus::Full {
                                panic!(
                                    "Task completed for {:?}, but status is already at {:?}",
                                    target_status, *s
                                );
                            }
                        });
                            //log::info!("Task completed for {:?}", target_status);
                            Some(())
                        }
                        Err(e) => {
                            log::error!("Chunk generation task failed: {e}");
                            sender.send_replace(ChunkResult::Failed);
                            None
                        }
                    }
                }
            });

        Box::pin(async move {
            match future.await {
                Ok(result) => result,
                Err(e) => {
                    log::error!("Chunk generation task failed: {e}");
                    None
                }
            }
        })
    }

    fn acquire_status_bump(&self, status: ChunkStatus) -> bool {
        let status_index = status.get_index();
        let parent_index = status
            .parent()
            .map_or(usize::MAX, super::chunk_access::ChunkStatus::get_index);

        //log::info!(
        //    "Parent index: {:?}, Status index: {:?}",
        //    parent_index,
        //    status_index
        //);

        let previous_started = self.started_work.compare_exchange(
            parent_index,
            status_index,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );

        match previous_started {
            Ok(_) => true,
            Err(current) => {
                if current != usize::MAX && current >= status_index {
                    false
                } else {
                    panic!(
                        "Unexpected started work status: {current:?} (index {current}) while trying to start: {status:?} (index {status_index})"
                    );
                }
            }
        }
    }

    /// Upgrades the chunk to a full chunk.
    ///
    /// # Panics
    /// Panics if the chunk is not at `ProtoChunk` stage or completed.
    pub fn upgrade_to_full(&self) {
        {
            let mut data = self.data.write();
            replace_with_or_abort(&mut *data, |chunk| match chunk {
                Some(ChunkAccess::Proto(proto_chunk)) => {
                    Some(ChunkAccess::Full(LevelChunk::from_proto(proto_chunk)))
                }
                _ => unreachable!(),
            });
        }

        self.sender.send_modify(|chunk| {
            *chunk = ChunkResult::Ok(ChunkStatus::Full);
        });
    }

    /// Inserts a chunk into the holder with a specific status.
    pub fn insert_chunk(&self, chunk: ChunkAccess, status: ChunkStatus) {
        self.data.write().replace(chunk);
        self.sender.send_modify(|c| {
            *c = ChunkResult::Ok(status);
        });
    }

    /// Cancels the current generation task.
    pub fn cancel_generation_task(&self) {
        let mut task_guard = self.generation_task.lock();
        if let Some(task) = task_guard.take() {
            task.mark_for_cancel();
        }
    }

    /// Forces the chunk to fail.
    pub fn force_fail(&self) {
        let mut task_guard = self.generation_task.lock();
        if let Some(task) = task_guard.take() {
            task.mark_for_cancel();
            log::info!("Force failing chunk at {:?}", self.pos);

            //self.sender.send_replace(ChunkResult::Failed);
            self.sender.send_replace(ChunkResult::Unloaded);
        }
    }
}

fn rayon_spawn<F, R>(thread_pool: &rayon::ThreadPool, func: F) -> impl Future<Output = R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static + Debug,
{
    let (sender, receiver) = oneshot::channel();
    thread_pool.spawn(move || {
        sender.send(func()).expect("Failed to send result");
    });
    async move { receiver.await.unwrap() }
}
