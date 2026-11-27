//! Host timings for library lookups and slot.state atomic writes.
//! The write here is to a regular disk, not a class-10 SD card: on the SP the
//! same two fsyncs (file + directory) are often tens to hundreds of milliseconds.
use std::time::{Duration, Instant};

use slot_store::{atomic_write, write_slot_state, Cart, Library, Platform, SlotState};

const GAMES: usize = 6_189;
const LOOPS: u32 = 20;

fn fake_carts(n: usize) -> Vec<Cart> {
    let platforms = Platform::ALL;
    (0..n)
        .map(|i| {
            let platform = platforms[i % platforms.len()];
            let stem = format!("Game {i:04}");
            Cart {
                platform,
                stem: stem.clone(),
                rom: format!("/tmp/Games/{}/{stem}.rom", platform.dir_name()).into(),
                label: Some(format!("/tmp/Labels/{}/{stem}.png", platform.dir_name()).into()),
                famicom_label: None,
                title: stem,
                code: String::new(),
                shell: None,
            }
        })
        .collect()
}

fn fake_library(carts: &[Cart]) -> Library {
    let mut library = Library::default();
    for (i, cart) in carts.iter().enumerate() {
        if i % 20 == 0 {
            let record = library.get_mut(cart.platform, &cart.stem);
            record.favorite = true;
            record.favorited_at = 1_700_000_000 + i as i64;
        }
        if i % 7 == 0 {
            let record = library.get_mut(cart.platform, &cart.stem);
            record.played_secs = i as u64 * 10;
            record.last_played = 1_700_000_000 + i as i64;
        }
    }
    library
}

fn mean(times: &[Duration]) -> Duration {
    times.iter().sum::<Duration>() / times.len() as u32
}

fn report(label: &str, times: &[Duration]) {
    let mut sorted = times.to_vec();
    sorted.sort();
    eprintln!(
        "perf_nav {label}: n={} mean={:?} median={:?} min={:?} max={:?}",
        times.len(),
        mean(times),
        sorted[sorted.len() / 2],
        sorted[0],
        sorted.last().unwrap()
    );
}

fn time_loops(loops: u32, mut body: impl FnMut()) -> Vec<Duration> {
    body();
    (0..loops)
        .map(|_| {
            let start = Instant::now();
            body();
            start.elapsed()
        })
        .collect()
}

#[test]
fn library_lookups_and_state_writes() {
    let carts = fake_carts(GAMES);
    let library = fake_library(&carts);

    let lookup = time_loops(LOOPS, || {
        assert!(library.any_favorite(&carts));
        for cart in &carts {
            let _ = library.get(cart.platform, &cart.stem);
        }
    });
    report("Library::get all 6189 + any_favorite", &lookup);

    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("Config")).unwrap();
    let state = SlotState {
        show_favorites: true,
        favorites_only: false,
        clock_set: true,
        ..SlotState::default()
    };
    write_slot_state(root, &state).unwrap();

    let writes = time_loops(LOOPS, || {
        write_slot_state(root, &state).unwrap();
    });
    report(
        "write_slot_state (atomic_write + 2 fsyncs) on this disk",
        &writes,
    );

    let bytes = b"favorites_only=1\nshow_favorites=1\n";
    let path = root.join("Config/tiny.txt");
    let tiny = time_loops(LOOPS, || {
        atomic_write(&path, bytes).unwrap();
    });
    report("atomic_write 32 bytes on this disk", &tiny);

    assert!(mean(&lookup) < Duration::from_millis(2_000));
    assert!(mean(&writes) < Duration::from_millis(2_000));
}
