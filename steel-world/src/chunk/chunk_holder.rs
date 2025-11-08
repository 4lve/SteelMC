use std::sync::Arc;

use tokio::sync::watch;

use crate::chunk::{
    chunk_access::{ChunkAccess, ChunkStatus},
    proto_chunk::ProtoChunk,
};

pub type ChunkStageHolder = (ChunkStatus, Arc<ChunkAccess>);

// Holds a ChunkAccess
pub struct ChunkHolder {
    
    // Will hold None if the chunk is cancelled.
    chunk_access: watch::Receiver<Option<ChunkStageHolder>>,
    sender: watch::Sender<Option<ChunkStageHolder>>,
}

impl ChunkHolder {
    pub fn new(proto_chunk: ProtoChunk) -> Self {
        let (sender, receiver) = watch::channel(Some((
            ChunkStatus::Empty,
            Arc::new(ChunkAccess::Proto(proto_chunk)),
        )));
        Self {
            chunk_access: receiver,
            sender,
        }
    }

    // Will return None if the chunk is not full or is cancelled.
    pub fn try_get_full(&self) -> Option<Arc<ChunkAccess>> {
        match &*self.chunk_access.borrow() {
            Some((ChunkStatus::Full, chunk)) => Some(chunk.clone()),
            _ => None,
        }
    }

    // Will wait until this chunk has reached the Full status. Will return None if the chunk generation iss cancelled.
    pub async fn as_full(&self) -> Option<Arc<ChunkAccess>> {
        let mut subscriber = self.sender.subscribe();
        loop {
            let chunk_access = subscriber.borrow();
            match &*chunk_access {
                Some((ChunkStatus::Full, chunk)) => {
                    return Some(chunk.clone());
                }
                Some((_, _)) => {}
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
}
