use std::path::PathBuf;

use janus::StringHash;

use crate::assets::{AssetError, AssetRegistry, HasMetadata, Import, Upload};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetMessageRequest {
    CreateNew { path: PathBuf },
    Delete,

    LoadToMemory,
    LoadToGpu,

    UnloadFromMemory,
    UnloadFromGpu,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AssetMessage {
    Request {
        id: StringHash,
        request: AssetMessageRequest,
    },

    Success {
        reference_id: StringHash,
        operation: AssetMessageRequest,
    },

    Failure {
        reference_id: StringHash,
        operation: AssetMessageRequest,
        error: AssetError,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug)]
pub enum AssetSyncMessage<M: Default + Clone + Copy> {
    Register { id: StringHash, data: M },
    Update { id: StringHash, data: M },
    Forget { id: StringHash },
}

impl<T, M> AssetRegistry<T, M>
where
    T: Import + Upload + HasMetadata<M>,
    <T as Upload>::AsGpu: HasMetadata<M>,
    M: Default + Clone + Copy,
{
    pub fn command_pipe(&self) -> crossbeam::channel::Sender<AssetMessage> {
        self.pipe_tx.clone()
    }

    pub fn pipe_messages(&mut self) {
        while let Ok(msg) = self.pipe_rx.recv() {
            match msg {
                AssetMessage::Request { id, request } => match &request {
                    AssetMessageRequest::CreateNew { path } => {
                        self.register(id, path);
                        self.pipe_tx
                            .send(AssetMessage::Success {
                                reference_id: id,
                                operation: request,
                            })
                            .unwrap();
                    }
                    AssetMessageRequest::Delete => {
                        if let Some(mut handle) = self.unregister(id) {
                            let _ = handle.free_from_gpu();
                            let _ = handle.free_from_memory();
                            self.pipe_tx
                                .send(AssetMessage::Success {
                                    reference_id: id,
                                    operation: request,
                                })
                                .unwrap();
                        } else {
                            self.pipe_tx
                                .send(AssetMessage::Failure {
                                    reference_id: id,
                                    operation: request,
                                    error: AssetError::AssetNotFound(id),
                                })
                                .unwrap();
                        }
                    }
                    AssetMessageRequest::LoadToMemory => {
                        if let Some(handle) = self.get_mut(id) {
                            if let Err(err) = handle.load_to_memory() {
                                let _ = self.pipe_tx.send(AssetMessage::Failure {
                                    reference_id: id,
                                    operation: request,
                                    error: err,
                                });
                            }
                        }
                    }
                    AssetMessageRequest::LoadToGpu => {
                        if let Some(handle) = self.get_mut(id) {
                            if let Err(err) = handle.upload_to_gpu() {
                                let _ = self.pipe_tx.send(AssetMessage::Failure {
                                    reference_id: id,
                                    operation: request,
                                    error: err,
                                });
                            }
                        }
                    }
                    AssetMessageRequest::UnloadFromMemory => {
                        if let Some(handle) = self.get_mut(id) {
                            if let Err(err) = handle.free_from_memory() {
                                let _ = self.pipe_tx.send(AssetMessage::Failure {
                                    reference_id: id,
                                    operation: request,
                                    error: err,
                                });
                            }
                        }
                    }
                    AssetMessageRequest::UnloadFromGpu => {
                        if let Some(handle) = self.get_mut(id) {
                            if let Err(err) = handle.free_from_gpu() {
                                let _ = self.pipe_tx.send(AssetMessage::Failure {
                                    reference_id: id,
                                    operation: request,
                                    error: err,
                                });
                            }
                        }
                    }
                },
                // synchronise completed operations with metadata registry
                AssetMessage::Success {
                    reference_id,
                    operation,
                } => {
                    if let Some(sync_pipe) = &self.sync_pipe_tx
                        && let Some(asset) = self.get(reference_id)
                    {
                        let metadata = asset.metadata();
                        match operation {
                            AssetMessageRequest::LoadToMemory
                            | AssetMessageRequest::LoadToGpu
                            | AssetMessageRequest::UnloadFromMemory
                            | AssetMessageRequest::UnloadFromGpu => sync_pipe
                                .send(AssetSyncMessage::Update {
                                    id: reference_id,
                                    data: metadata,
                                })
                                .unwrap(),

                            // create and delete handled explicitly
                            _ => {}
                        }
                    }
                }
                // ignore anything else
                _ => {}
            }
        }
    }
}
