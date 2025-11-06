use std::sync::Arc;

use tokio::sync::watch;

use crate::chunk::{
    level_chunk::LevelChunk,
    proto_chunk::{ChunkAccses, ChunkStatus, ProtoChunk},
};

// Holds a ChunkAccsess
pub struct ChunkHolder {
    // Will hold None if the chunk is cancelled.
    chunk_access: watch::Receiver<Option<(ChunkStatus, ChunkAccses)>>,
    sender: watch::Sender<Option<(ChunkStatus, ChunkAccses)>>,
}

impl ChunkHolder {
    pub fn new(proto_chunk: ProtoChunk) -> Self {
        let (sender, receiver) = watch::channel(Some((
            ChunkStatus::Empty,
            ChunkAccses::Proto(Arc::new(proto_chunk)),
        )));
        Self {
            chunk_access: receiver,
            sender,
        }
    }

    // Will return None if the chunk is not full or is cancelled.
    pub fn try_get_full(&self) -> Option<Arc<LevelChunk>> {
        match &*self.chunk_access.borrow() {
            Some((ChunkStatus::Full, ChunkAccses::Full(full_chunk))) => Some(full_chunk.clone()),
            _ => None,
        }
    }

    // Will wait until this chunk has reached the Full status. Will return None if the chunk generation iss cancelled.
    pub async fn as_full(&self) -> Option<Arc<LevelChunk>> {
        let mut subscriber = self.sender.subscribe();
        loop {
            let chunk_access = subscriber.borrow();
            match &*chunk_access {
                Some((ChunkStatus::Full, ChunkAccses::Full(full_chunk))) => {
                    return Some(full_chunk.clone());
                }
                Some((_, _)) => {}
                None => {
                    return None;
                }
            }
            drop(chunk_access);
            if let Err(e) = subscriber.changed().await {
                log::error!("Failed to wait for chunk access: {e}");
                return None;
            }
        }
    }
}
