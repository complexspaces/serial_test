#![allow(clippy::await_holding_lock)]

use crate::code_lock::{check_new_key, LOCKS};
use futures::FutureExt;
use std::{ops::Deref, panic};

#[doc(hidden)]
pub fn local_parallel_core_with_return<E>(
    name: &str,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    let res = panic::catch_unwind(function);
    unlock.deref()[name].end_parallel();
    match res {
        Ok(ret) => ret,
        Err(err) => {
            panic::resume_unwind(err);
        }
    }
}

#[doc(hidden)]
pub fn local_parallel_core(name: &str, function: fn()) {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    let res = panic::catch_unwind(|| {
        function();
    });
    unlock.deref()[name].end_parallel();
    if let Err(err) = res {
        panic::resume_unwind(err);
    }
}

#[doc(hidden)]
pub async fn local_async_parallel_core_with_return<E>(
    name: &str,
    fut: impl std::future::Future<Output = Result<(), E>> + panic::UnwindSafe,
) -> Result<(), E> {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    let res = fut.catch_unwind().await;
    unlock.deref()[name].end_parallel();
    match res {
        Ok(ret) => ret,
        Err(err) => {
            panic::resume_unwind(err);
        }
    }
}

#[doc(hidden)]
pub async fn local_async_parallel_core(
    name: &str,
    fut: impl std::future::Future<Output = ()> + panic::UnwindSafe,
) {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    let res = fut.catch_unwind().await;
    unlock.deref()[name].end_parallel();
    if let Err(err) = res {
        panic::resume_unwind(err);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        code_lock::LOCKS, local_async_parallel_core, local_async_parallel_core_with_return,
        local_parallel_core, local_parallel_core_with_return,
    };
    use std::{io::Error, ops::Deref, panic};

    #[test]
    fn unlock_on_assert_sync_without_return() {
        let _ = panic::catch_unwind(|| {
            local_parallel_core("unlock_on_assert_sync_without_return", || {
                assert!(false);
            })
        });
        let unlock = LOCKS.read_recursive();
        assert_eq!(
            unlock.deref()["unlock_on_assert_sync_without_return"].parallel_count(),
            0
        );
    }

    #[test]
    fn unlock_on_assert_sync_with_return() {
        let _ = panic::catch_unwind(|| {
            local_parallel_core_with_return(
                "unlock_on_assert_sync_with_return",
                || -> Result<(), Error> {
                    assert!(false);
                    Ok(())
                },
            )
        });
        let unlock = LOCKS.read_recursive();
        assert_eq!(
            unlock.deref()["unlock_on_assert_sync_with_return"].parallel_count(),
            0
        );
    }

    #[tokio::test]
    async fn unlock_on_assert_async_without_return() {
        async fn demo_assert() {
            assert!(false);
        }
        async fn call_serial_test_fn() {
            local_async_parallel_core("unlock_on_assert_async_without_return", demo_assert()).await
        }
        // as per https://stackoverflow.com/a/66529014/320546
        let _ = panic::catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            let _enter_guard = handle.enter();
            futures::executor::block_on(call_serial_test_fn());
        });
        let unlock = LOCKS.read_recursive();
        assert_eq!(
            unlock.deref()["unlock_on_assert_async_without_return"].parallel_count(),
            0
        );
    }

    #[tokio::test]
    async fn unlock_on_assert_async_with_return() {
        async fn demo_assert() -> Result<(), Error> {
            assert!(false);
            Ok(())
        }

        #[allow(unused_must_use)]
        async fn call_serial_test_fn() {
            local_async_parallel_core_with_return(
                "unlock_on_assert_async_with_return",
                demo_assert(),
            )
            .await;
        }

        // as per https://stackoverflow.com/a/66529014/320546
        let _ = panic::catch_unwind(|| {
            let handle = tokio::runtime::Handle::current();
            let _enter_guard = handle.enter();
            futures::executor::block_on(call_serial_test_fn());
        });
        let unlock = LOCKS.read_recursive();
        assert_eq!(
            unlock.deref()["unlock_on_assert_async_with_return"].parallel_count(),
            0
        );
    }
}
