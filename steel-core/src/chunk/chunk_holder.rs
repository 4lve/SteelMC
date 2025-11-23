//! `ChunkHolder` manages chunk state and asynchronous generation tasks.
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use futures::Future;
use parking_lot::Mutex;
use replace_with::replace_with_or_abort;
use steel_utils::ChunkPos;
use tokio::sync::watch;
use tokio::task::spawn_blocking;

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

/// A tuple containing the chunk status and the chunk access.
pub type ChunkStageHolder = (ChunkStatus, ChunkAccess);

/// The result of a chunk operation.
pub enum ChunkResult {
    /// The chunk is not loaded.
    Unloaded,
    /// The chunk operation failed.
    Failed,
    /// The chunk operation succeeded.
    Ok(ChunkStageHolder),
}

/// Holds a chunk in a watch channel, allowing for concurrent access and state tracking.
pub struct ChunkHolder {
    chunk_access: watch::Receiver<ChunkResult>,
    sender: watch::Sender<ChunkResult>,
    generation_task: Mutex<Option<Arc<ChunkGenerationTask>>>,
    pos: ChunkPos,
    /// The current ticket level of the chunk.
    pub ticket_level: AtomicU8,
    /// The highest status that has started work.
    started_work: AtomicUsize,
    /// The highest status that generation is allowed to reach.
    highest_allowed_status: AtomicU8,
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
        let highest_allowed_status = ChunkLevel::generation_status(ticket_level)
            .map(|s| s.get_index() as u8)
            .unwrap_or(STATUS_NONE);

        Self {
            chunk_access: receiver,
            sender,
            generation_task: Mutex::new(None),
            pos,
            ticket_level: AtomicU8::new(ticket_level),
            started_work: AtomicUsize::new(usize::MAX),
            highest_allowed_status: AtomicU8::new(highest_allowed_status),
        }
    }

    /// Updates the highest allowed generation status based on the ticket level.
    pub fn update_highest_allowed_status(&self, ticket_level: u8) {
        let new_status = ChunkLevel::generation_status(ticket_level)
            .map(|s| s.get_index() as u8)
            .unwrap_or(STATUS_NONE);
        self.highest_allowed_status
            .store(new_status, Ordering::Relaxed);
    }

    /// Checks if the given status is disallowed.
    pub fn is_status_disallowed(&self, status: ChunkStatus) -> bool {
        let allowed = self.highest_allowed_status.load(Ordering::Relaxed);
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
        lookup_provider: impl FnMut(ChunkPos) -> Option<Arc<ChunkHolder>>,
    ) {
        if self.is_status_disallowed(status) {
            return;
        }

        if self.with_chunk(status, |_| ()).is_some() {
            return;
        }

        let task = self.generation_task.lock();

        #[allow(clippy::unwrap_used)]
        if task.is_none() || status > task.as_ref().unwrap().target_status {
            drop(task);
            self.reschedule_chunk_task_b(status, chunk_map, lookup_provider);
        }
    }

    /// Reschedules the chunk task to the given status.
    pub(crate) fn reschedule_chunk_task_b(
        &self,
        status: ChunkStatus,
        chunk_map: Arc<ChunkMap>,
        lookup_provider: impl FnMut(ChunkPos) -> Option<Arc<ChunkHolder>>,
    ) {
        let new_task = chunk_map.schedule_generation_task_b(status, self.pos, lookup_provider);
        let mut old_task_guard = self.generation_task.lock();

        let old_task = old_task_guard.replace(new_task);
        drop(old_task_guard);

        if let Some(old_task) = old_task {
            old_task.mark_for_cancel();
        }
    }

    /// Gets mutable access to the chunk if it has reached the given status.
    pub fn with_chunk_mut<F, R>(&self, status: ChunkStatus, f: F) -> Option<R>
    where
        F: FnOnce(&mut ChunkAccess) -> R,
    {
        let mut return_value: Option<R> = None;
        self.sender.send_modify(|chunk| match chunk {
            ChunkResult::Ok((s, chunk)) if status <= *s => {
                return_value = Some(f(chunk));
            }
            _ => {}
        });
        return_value
    }

    /// Gets access to the chunk if it has reached the given status.
    #[inline]
    pub fn with_chunk<F, R>(&self, status: ChunkStatus, f: F) -> Option<R>
    where
        F: FnOnce(&ChunkAccess) -> R,
    {
        match &*self.chunk_access.borrow() {
            ChunkResult::Ok((s, chunk)) if status <= *s => Some(f(chunk)),
            _ => None,
        }
    }

    /// Gets access to the chunk if it is full.
    pub fn with_full_chunk<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&LevelChunk) -> R,
    {
        match &*self.chunk_access.borrow() {
            ChunkResult::Ok((ChunkStatus::Full, ChunkAccess::Full(chunk))) => Some(f(chunk)),
            _ => None,
        }
    }

    /// Waits until the chunk has reached the given status, then calls the function.
    pub fn await_chunk_and_then<F, R>(
        &self,
        status: ChunkStatus,
        f: F,
    ) -> impl Future<Output = Option<R>>
    where
        F: FnOnce(&ChunkAccess) -> R,
    {
        let mut subscriber = self.sender.subscribe();
        async move {
            loop {
                {
                    let chunk_access = subscriber.borrow_and_update();
                    match &*chunk_access {
                        ChunkResult::Ok((s, chunk)) if status <= *s => {
                            return Some(f(chunk));
                        }
                        ChunkResult::Failed => return None,
                        _ => {}
                    }
                }

                if subscriber.changed().await.is_err() {
                    log::error!("Failed to wait for chunk access");
                    return None;
                }
            }
        }
    }

    /// Waits until this chunk has reached the Full status.
    pub async fn await_full_and_then<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&ChunkAccess) -> R,
    {
        self.await_chunk_and_then(ChunkStatus::Full, f).await
    }

    /// Waits until this chunk has reached the Full status.
    pub async fn await_full(&self) -> Option<()> {
        self.await_full_and_then(|_| ()).await
    }

    /// Gets the persisted status of the chunk.
    pub fn persisted_status(&self) -> Option<ChunkStatus> {
        match &*self.chunk_access.borrow() {
            ChunkResult::Ok((s, _)) => Some(*s),
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
    ) -> NeighborReady {
        let target_status = step.target_status;

        if self.is_status_disallowed(target_status) {
            return Box::pin(async { None });
        }

        if !self.acquire_status_bump(target_status) {
            let self_clone = self.clone();
            return Box::pin(async move {
                self_clone.await_chunk_and_then(target_status, |_| ()).await
            });
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
                    match spawn_blocking(move || task(context, &step, &cache, self_clone)).await {
                        Ok(_) => {
                            sender.send_modify(|chunk| {
                                if let ChunkResult::Ok((s, _)) = chunk {
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

                    let has_parent = self_clone.with_chunk(parent_status, |_| ()).is_some();

                    assert!(has_parent, "Parent chunk missing");

                    match spawn_blocking(move || task(context, &step, &cache, self_clone)).await {
                        Ok(_) => {
                            sender.send_modify(|chunk| if let ChunkResult::Ok((s, _)) = chunk {
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
        self.sender.send_modify(|chunk| {
            replace_with_or_abort(chunk, |chunk| match chunk {
                ChunkResult::Ok((_, ChunkAccess::Proto(proto_chunk))) => ChunkResult::Ok((
                    ChunkStatus::Full,
                    ChunkAccess::Full(LevelChunk::from_proto(proto_chunk)),
                )),
                _ => panic!("Cannot upgrade chunk: not at ProtoChunk status"),
            });
        });
    }

    /// Inserts a chunk into the holder with a specific status.
    pub fn insert_chunk(&self, chunk: ChunkAccess, status: ChunkStatus) {
        self.sender.send_modify(|c| {
            *c = ChunkResult::Ok((status, chunk));
        });
    }

    /// Cancels the current generation task.
    pub fn cancel_generation_task(&self) {
        let mut task_guard = self.generation_task.lock();
        if let Some(task) = task_guard.take() {
            task.mark_for_cancel();
        }
    }

    /// Cancels the current generation task asynchronously.
    pub async fn cancel_generation_task_async(&self) {
        let mut task_guard = self.generation_task.lock();
        if let Some(task) = task_guard.take() {
            task.mark_for_cancel();
        }
    }
}
