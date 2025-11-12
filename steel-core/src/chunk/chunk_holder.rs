use replace_with::replace_with_or_abort;
use tokio::sync::watch;

use crate::chunk::{
    chunk_access::{ChunkAccses, ChunkStatus},
    level_chunk::LevelChunk,
    proto_chunk::ProtoChunk,
};

pub type ChunkStageHolder = (ChunkStatus, ChunkAccses);

// Holds a ChunkAccess
pub struct ChunkHolder {
    // Will hold None if the chunk is cancelled.
    chunk_access: watch::Receiver<Option<ChunkStageHolder>>,
    sender: watch::Sender<Option<ChunkStageHolder>>,
}

impl ChunkHolder {
    pub fn new(proto_chunk: ProtoChunk) -> Self {
        let (sender, receiver) =
            watch::channel(Some((ChunkStatus::Empty, ChunkAccses::Proto(proto_chunk))));
        Self {
            chunk_access: receiver,
            sender,
        }
    }

    pub fn with_chunk_mut<F, R>(&self, status: ChunkStatus, f: F) -> Option<R>
    where
        F: FnOnce(&mut ChunkAccses) -> R,
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

    pub fn with_chunk<F, R>(&self, status: ChunkStatus, f: F) -> Option<R>
    where
        F: FnOnce(&ChunkAccses) -> R,
    {
        match &*self.chunk_access.borrow() {
            Some((s, chunk)) if status <= *s => Some(f(chunk)),
            _ => None,
        }
    }

    // Will return None if the chunk is not full or is cancelled.
    pub fn with_full_chunk<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&LevelChunk) -> R,
    {
        match &*self.chunk_access.borrow() {
            Some((ChunkStatus::Full, ChunkAccses::Full(chunk))) => Some(f(chunk)),
            _ => None,
        }
    }

    pub async fn await_chunk_and_then<F, R>(&self, status: ChunkStatus, f: F) -> Option<R>
    where
        F: FnOnce(&ChunkAccses) -> R,
    {
        let mut subscriber = self.sender.subscribe();
        loop {
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
            drop(chunk_access);

            let changed = subscriber.changed().await;
            if let Err(e) = changed {
                log::error!("Failed to wait for chunk access: {e}");
                return None;
            }
        }
    }

    // Will wait until this chunk has reached the Full status. Will return None if the chunk generation iss cancelled.
    pub async fn await_full_and_then<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&ChunkAccses) -> R,
    {
        self.await_chunk_and_then(ChunkStatus::Full, f).await
    }

    pub async fn await_full(&self) -> Option<()> {
        self.await_full_and_then(|_| ()).await
    }

    /// # Panics
    /// The function expects that the chunk is completed and at ProtoChunk stage
    pub fn upgrade_to_full(&self) {
        self.sender.send_modify(|chunk| {
            replace_with_or_abort(chunk, |chunk| match chunk {
                Some((_, ChunkAccses::Proto(proto_chunk))) => Some((
                    ChunkStatus::Full,
                    ChunkAccses::Full(LevelChunk::from_proto(proto_chunk)),
                )),
                _ => {
                    panic!("Cannot upgrade a chunk that is not at full and at ProtoChunk status");
                }
            });
        });
    }
}
