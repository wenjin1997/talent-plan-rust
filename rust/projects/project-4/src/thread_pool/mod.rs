use crate::Result;
pub trait ThreadPool {
    /// Creates a new thread pool, immediately spawning the specified number 
    /// of threads.
    /// 
    /// Returns an error if any thread fails to spawn. All previously-spawned 
    /// threads are terminated.
    fn new(threads: u32) -> Result<Self> where Self: Sized;

    /// spawn a function into the threadpool.
    /// 
    /// Spawning always succeeds, but if the function panics the threadpool 
    /// continues to operate with the same number of threads — the thread 
    /// count is not reduced nor is the thread pool destroyed, corrupted or 
    /// invalidated.
    fn spawn<F>(&self, job: F) where F: FnOnce() + Send + 'static;
}

mod naive;
mod rayon;
mod shared_queue;

pub use self::naive::NaiveThreadPool;
pub use self::rayon::RayonThreadPool;
pub use self::shared_queue::SharedQueueThreadPool;