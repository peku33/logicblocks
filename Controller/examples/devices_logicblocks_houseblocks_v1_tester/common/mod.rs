pub mod relay14_common_a;

use futures::Future;

pub fn execute_on_tokio<F: Future>(f: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(f)
}
