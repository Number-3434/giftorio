use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, Mutex},
};

pub type ChunkQueue = Arc<Mutex<VecDeque<Vec<u8>>>>;

pub struct StreamingWriter {
    chunks: ChunkQueue,
}

impl StreamingWriter {
    pub fn new(chunks: ChunkQueue) -> Self {
        Self { chunks }
    }
}

impl Write for StreamingWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.chunks.lock().unwrap().push_back(data.to_vec());
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
