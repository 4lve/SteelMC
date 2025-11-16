//! This module contains the `ChunkHolder` struct, which is used to hold a chunk in a watch channel.
use std::pin::Pin;
use std::sync::Arc;

use futures::{Future, future};
use replace_with::replace_with_or_abort;
use steel_utils::ChunkPos;
use tokio::sync::{Mutex, watch};

use crate::{
    ChunkMap,
    chunk::{
        chunk_access::{ChunkAccess, ChunkStatus},
        chunk_generation_task::ChunkGenerationTask,
        level_chunk::LevelChunk,
        proto_chunk::ProtoChunk,
    },
};

/// A tuple containing the chunk status and the chunk access.
pub type ChunkStageHolder = (ChunkStatus, ChunkAccess);

/// Holds a chunk in a watch channel, allowing for multiple threads to access it.
pub struct ChunkHolder {
    // Will hold None if the chunk is cancelled.
    chunk_access: watch::Receiver<Option<ChunkStageHolder>>,
    sender: watch::Sender<Option<ChunkStageHolder>>,
    generation_task: Mutex<Option<Arc<ChunkGenerationTask>>>,
    pos: ChunkPos,
}

impl ChunkHolder {
    /// Creates a new chunk holder.
    #[must_use]
    pub fn new(proto_chunk: ProtoChunk, pos: ChunkPos) -> Self {
        let (sender, receiver) =
            watch::channel(Some((ChunkStatus::Empty, ChunkAccess::Proto(proto_chunk))));
        Self {
            chunk_access: receiver,
            sender,
            generation_task: Mutex::new(None),
            pos,
        }
    }

    /// Returns a future that completes when the chunk has reached the given status or None if cancelled.
    #[allow(clippy::missing_panics_doc)]
    pub async fn schedule_chunk_generation_task(
        &self,
        status: ChunkStatus,
        chunk_map: Arc<ChunkMap>,
    ) -> Pin<Box<dyn Future<Output = Option<()>> + Send + '_>> {
        // Check if the chunk is already at the given status.
        if self.with_chunk(status, |_| ()).is_some() {
            return Box::pin(future::ready(Some(())));
        }

        let task = self.generation_task.lock().await;

        #[allow(clippy::unwrap_used)]
        if task.is_none() || status > task.as_ref().unwrap().target_status {
            drop(task);
            self.reschedule_chunk_task(status, chunk_map).await;
        }

        Box::pin(ChunkHolder::await_chunk_and_then(
            self.sender.subscribe(),
            status,
            |_| (),
        ))
    }

    /// Reschedules the chunk task to the given status.
    pub async fn reschedule_chunk_task(&self, status: ChunkStatus, chunk_map: Arc<ChunkMap>) {
        let new_task = chunk_map.schedule_generation_task(status, self.pos);
        let mut old_task_guard = self.generation_task.lock().await;

        let old_task = old_task_guard.replace(new_task);
        drop(old_task_guard);

        if let Some(old_task) = old_task {
            old_task.mark_for_cancel();
        }
    }

    /// Gets mutable access to the chunk if the chunk has reached the given status.
    pub fn with_chunk_mut<F, R>(&self, status: ChunkStatus, f: F) -> Option<R>
    where
        F: FnOnce(&mut ChunkAccess) -> R,
    {
        let mut return_value: Option<R> = None;
        self.sender.send_modify(|chunk| match chunk {
            Some((s, chunk)) if status <= *s => {
                return_value = Some(f(chunk));
            }
            _ => {}
        });
        return_value
    }

    /// Gets access to the chunk if the chunk has reached the given status.
    pub fn with_chunk<F, R>(&self, status: ChunkStatus, f: F) -> Option<R>
    where
        F: FnOnce(&ChunkAccess) -> R,
    {
        match &*self.chunk_access.borrow() {
            Some((s, chunk)) if status <= *s => Some(f(chunk)),
            _ => None,
        }
    }

    /// Gets access to the chunk if the chunk is full.
    ///
    /// Will return None if the chunk is not full or is cancelled.
    pub fn with_full_chunk<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&LevelChunk) -> R,
    {
        match &*self.chunk_access.borrow() {
            Some((ChunkStatus::Full, ChunkAccess::Full(chunk))) => Some(f(chunk)),
            _ => None,
        }
    }

    /// Waits until the chunk has reached the given status, and then calls the given function.
    pub async fn await_chunk_and_then<F, R>(
        mut subscriber: watch::Receiver<Option<ChunkStageHolder>>,
        status: ChunkStatus,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&ChunkAccess) -> R,
    {
        loop {
            {
                let chunk_access = subscriber.borrow_and_update();
                match &*chunk_access {
                    Some((s, chunk)) if status <= *s => {
                        return Some(f(chunk));
                    }
                    // Don't return
                    Some(_) => {}
                    None => {
                        return None;
                    }
                }
            }

            if subscriber.changed().await.is_err() {
                log::error!("Failed to wait for chunk access");
                return None;
            }
        }
    }

    /// Waits until this chunk has reached the Full status.
    ///
    /// Will return None if the chunk generation is cancelled.
    pub async fn await_full_and_then<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&ChunkAccess) -> R,
    {
        ChunkHolder::await_chunk_and_then(self.sender.subscribe(), ChunkStatus::Full, f).await
    }

    /// Waits until this chunk has reached the Full status.
    pub async fn await_full(&self) -> Option<()> {
        self.await_full_and_then(|_| ()).await
    }

    /// Upgrades the chunk to a full chunk.
    ///
    /// # Panics
    /// The function expects that the chunk is completed and at `ProtoChunk` stage
    pub fn upgrade_to_full(&self) {
        self.sender.send_modify(|chunk| {
            replace_with_or_abort(chunk, |chunk| match chunk {
                Some((_, ChunkAccess::Proto(proto_chunk))) => Some((
                    ChunkStatus::Full,
                    ChunkAccess::Full(LevelChunk::from_proto(proto_chunk)),
                )),
                _ => {
                    panic!("Cannot upgrade a chunk that is not at full and at ProtoChunk status");
                }
            });
        });
    }
}
