use std::sync::atomic::{AtomicUsize, Ordering};

/// Map `f` over `items` on at most `workers` threads, with a progress line.
/// Results come back in input order.
pub fn par_map<T: Sync, R: Send>(
    items: &[T],
    workers: usize,
    label: impl Fn(&T) -> String + Sync,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let total = items.len();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel::<(usize, R)>();

    std::thread::scope(|s| {
        for _ in 0..workers.min(total) {
            let tx = tx.clone();
            let (next, done, label, f) = (&next, &done, &label, &f);
            s.spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= total {
                    break;
                }
                let r = f(&items[i]);
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                super::shell::progress_line(n, total, &label(&items[i]));
                let _ = tx.send((i, r));
            });
        }
    });
    drop(tx);
    super::shell::clear_progress_line();

    let mut out: Vec<Option<R>> = (0..total).map(|_| None).collect();
    for (i, r) in rx {
        out[i] = Some(r);
    }
    out.into_iter().flatten().collect()
}
