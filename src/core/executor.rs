use Result;
use scoped_pool::Pool;
use crossbeam::channel;

/// Search executor whether search request are single thread or multithread.
///
/// We don't expose Rayon thread pool directly here for several reasons.
///
/// First dependency hell. It is not a good idea to expose the
/// API of a dependency, knowing it might conflict with a different version
/// used by the client. Second, we may stop using rayon in the future.
pub enum Executor {
    SingleThread,
    ThreadPool(Pool),
}

impl Executor {
    /// Creates an Executor that performs all task in the caller thread.
    pub fn single_thread() -> Executor {
        Executor::SingleThread
    }

    // Creates an Executor that dispatches the tasks in a thread pool.
    pub fn multi_thread(num_threads: usize) -> Executor {
        let pool = Pool::new(num_threads);
        Executor::ThreadPool(pool)
    }

    // Perform a map in the thread pool.
    //
    // Regardless of the executor (`SingleThread` or `ThreadPool`), panics in the task
    // will propagate to the caller.
    pub fn map<A: Send, R: Send, AIterator: Iterator<Item=A>, F: Sized + Sync + Fn(A) -> Result<R>>(&self, f: F, args: AIterator) -> Result<Vec<R>> {
        match self {
            Executor::SingleThread => {
                args.map(f).collect::<Result<_>>()
            }
            Executor::ThreadPool(pool) => {
                let (fruit_sender, fruit_receiver) = channel::unbounded();
                pool.scoped(|scope| {
                    for arg in args {
                        scope.execute(|| {
                            let fruit = f(arg);
                            if let Err(err) = fruit_sender.send(fruit) {
                                error!("Failed to send search task. It probably means all search threads have panicked. {:?}", err);
                            }
                        });
                    }
                });
                fruit_receiver.into_iter().collect::<Result<_>>()
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Executor;


    #[test]
    #[should_panic]
    fn test_panic_propagates_single_thread() {
        let _result: Vec<usize> = Executor::single_thread().map(|_| {panic!("panic should propagate"); }, vec![0].into_iter()).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_panic_propagates_multi_thread() {
        let _result: Vec<usize> = Executor::multi_thread(2).map(|_| {panic!("panic should propagate"); }, vec![0].into_iter()).unwrap();
    }

}