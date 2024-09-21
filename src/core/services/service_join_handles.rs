use tokio::task::JoinHandle;
use redgold_schema::{ErrorInfoContext, RgResult};

pub struct ServiceJoinHandles {
    pub handles: Vec<NamedHandle>
}

impl ServiceJoinHandles {
    pub fn add(&mut self, name: impl Into<String>, handle: JoinHandle<RgResult<()>>) -> &mut Self {
        self.handles.push(NamedHandle::new(name, handle));
        self
    }
}

impl Default for ServiceJoinHandles {
    fn default() -> Self {
        Self {
            handles: Vec::new()
        }
    }
}

pub struct NamedHandle {
    pub name: String,
    pub handle: JoinHandle<RgResult<()>>
}

impl NamedHandle {
    pub fn new(name: impl Into<String>, handle: JoinHandle<RgResult<()>>) -> Self {
        Self {
            name: name.into(),
            handle,
        }
    }

    pub async fn result(self) -> RgResult<String> {
        self.handle.await.error_info("Join error")??;
        Ok(self.name)
    }
}