use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

struct State {
  completed: bool,
  waker: Option<Waker>,
}

#[derive(Clone)]
pub struct SignalFutureController {
  shared_state: Arc<Mutex<State>>,
}

impl SignalFutureController {
  pub fn signal(&self) {
    let mut shared_state = self.shared_state.lock().unwrap();
    shared_state.completed = true;
    if let Some(waker) = shared_state.waker.take() {
      waker.wake();
    };
  }
}

/// A simple future that can be programmatically resolved externally using the controller that is provided in tandem when creating a `SignalFuture`. This makes it useful as a way to signal to some consumer of the future that something has completed, using standard async syntax and semantics.
///
/// # Examples
///
/// ```
/// struct DelayedWriter { fd: File, pending: Mutex<Vec<(u64, Vec<u8>, SignalFutureController)>> }
/// impl DelayedWriter {
///   pub async fn write(&self, offset: u64, data: Vec<u8>) {
///     let (fut, fut_ctl) = SignalFuture::new();
///     self.pending.lock().await.push((offset, data, fut_ctl));
///     fut.await
///   }
///   pub async fn background_loop(&self) {
///     loop {
///       sleep(Duration::from_millis(500));
///       for (offset, data, fut_ctl) in self.pending.lock().await.drain(..) {
///         self.fd.write_at(offset, data).await;
///         fut_ctl.signal();
///       };
///     };
///   }
/// }
/// ```
pub struct SignalFuture {
  shared_state: Arc<Mutex<State>>,
}

impl SignalFuture {
  pub fn new() -> (SignalFuture, SignalFutureController) {
    let shared_state = Arc::new(Mutex::new(State {
      completed: false,
      waker: None,
    }));

    (
      SignalFuture {
        shared_state: shared_state.clone(),
      },
      SignalFutureController {
        shared_state: shared_state.clone(),
      },
    )
  }
}

impl Future for SignalFuture {
  type Output = ();

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut shared_state = self.shared_state.lock().unwrap();
    if shared_state.completed {
      Poll::Ready(())
    } else {
      shared_state.waker = Some(cx.waker().clone());
      Poll::Pending
    }
  }
}
