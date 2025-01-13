use std::sync::Arc;
use arc_swap::ArcSwap;

#[derive(Clone, Default)]
pub struct WriteOneReadAll<T> where T: Clone {
    pub inner: Arc<ArcSwap<T>>
}

impl<T> WriteOneReadAll<T> where T: Clone  {
    pub fn write(&mut self, t: T) -> () {
        self.inner.store(Arc::new(t));
    }
    pub fn read(&self) -> Arc<T> {
        self.inner.load_full()
    }

    pub fn clone_read(&self) -> T {
        (*self.read()).clone()
    }

}


