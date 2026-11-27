//! Headless timings for shelf rearrange and library lookups, sized like Jeff's
//! RG35XX SP card (~6,189 games). These numbers are this host's CPU, not the
//! handheld's, and they leave out the SD card's fsync (see slot-store).
use std::time::{Duration, Instant};

use slot_store::{Cart, Library, Platform, SlotState};

use crate::app::App;

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
    let min = sorted[0];
    let max = *sorted.last().unwrap();
    let mid = sorted[sorted.len() / 2];
    eprintln!(
        "perf_nav {label}: n={} mean={:?} median={:?} min={:?} max={:?}",
        times.len(),
        mean(times),
        mid,
        min,
        max
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
fn arrange_and_lookups_on_a_full_card() {
    let carts = fake_carts(GAMES);
    let library = fake_library(&carts);
    assert!(library.any_favorite(&carts));

    let lookup = time_loops(LOOPS, || {
        let mut hits = 0usize;
        for cart in &carts {
            if library.get(cart.platform, &cart.stem).favorite {
                hits += 1;
            }
        }
        assert!(hits > 0);
        assert!(library.any_favorite(&carts));
    });
    report("library.get+any_favorite over 6189 carts", &lookup);

    let mut app = App::new(carts);
    app.test_set_library(library);
    app.test_set_state(SlotState {
        show_favorites: true,
        favorites_only: false,
        clock_set: true,
        ..SlotState::default()
    });
    app.test_arrange();

    let arrange = time_loops(LOOPS, || app.test_arrange());
    report("App::arrange (all games + Favorites shelf)", &arrange);

    let toggle_only = time_loops(LOOPS, || app.test_toggle_favorites_only());
    report("toggle favorites_only + arrange", &toggle_only);

    let toggle_shelf = time_loops(LOOPS, || app.test_toggle_favorites_shelf());
    report("toggle show_favorites + arrange", &toggle_shelf);

    // Loose ceiling so CI fails on a real regression, not on a slow runner.
    assert!(
        mean(&arrange) < Duration::from_millis(2_000),
        "arrange mean {:?} is too slow for 6189 games",
        mean(&arrange)
    );
}
