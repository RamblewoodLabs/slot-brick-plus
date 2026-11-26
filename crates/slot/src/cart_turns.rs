//! Builds the frames of a cart's turn into the slot (`slot_ui::turn`) off the UI thread, for
//! the selected cart ahead of time, and hands them to the app as textures.
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use slot_gfx::{Compositor, TexId};
use slot_store::{Cart, Platform};

type Key = (Platform, String);
use slot_ui::turn::{faces, gw_faces, TurnFaces, GW_OPEN_DIR};

use crate::app::{App, TurnTex};
use crate::label_cache::label_art;

/// How long a press of A waits for frames that are not ready yet before going in unturned.
const WAIT: Duration = Duration::from_millis(1500);

pub struct CartTurns {
    jobs: Option<Sender<Cart>>,
    built: Receiver<(Key, Option<TurnFaces>)>,
    asked: Option<Key>,
    /// The last cart that came back with no frames (a Game & Watch with no strip), so it
    /// is not asked for again every frame while it stays selected.
    none: Option<Key>,
    free: Vec<TexId>,
}

impl CartTurns {
    pub fn spawn(root: PathBuf) -> Self {
        let (jobs, inbox) = mpsc::channel::<Cart>();
        let (outbox, built) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("slot-turns".into())
            .spawn(move || work(&inbox, &outbox, &root));
        let jobs = match spawned {
            Ok(_) => Some(jobs),
            Err(e) => {
                eprintln!("slot: cart turns: worker thread failed to start: {e}");
                None
            }
        };
        CartTurns {
            jobs,
            built,
            asked: None,
            none: None,
            free: Vec::new(),
        }
    }

    pub fn sync(&mut self, app: &mut App, compositor: &mut Compositor) {
        while let Ok((stem, built)) = self.built.try_recv() {
            self.place(app, compositor, stem, built);
        }
        let Some(cart) = app.turn_want() else {
            return;
        };
        if app.turn_for() == Some((cart.platform, cart.stem.as_str())) {
            return;
        }
        let key = (cart.platform, cart.stem.clone());
        if self.none.as_ref() == Some(&key) {
            return;
        }
        if self.asked.as_ref() != Some(&key) {
            let stem = key;
            if let Some(jobs) = &self.jobs {
                if jobs.send(cart).is_ok() {
                    self.asked = Some(stem);
                }
            }
        }
        if !app.turn_needed() {
            return;
        }
        // On its way in now: wait for its frames (the core is loading meanwhile).
        while let Ok((stem, built)) = self.built.recv_timeout(WAIT) {
            let done = self.asked.as_ref() == Some(&stem);
            self.place(app, compositor, stem, built);
            if done {
                return;
            }
        }
        self.asked = None;
    }

    fn place(
        &mut self,
        app: &mut App,
        compositor: &mut Compositor,
        stem: Key,
        built: Option<TurnFaces>,
    ) {
        if self.asked.as_ref() == Some(&stem) {
            self.asked = None;
        }
        let Some(built) = built else {
            self.none = Some(stem);
            return;
        };
        if self.none.as_ref() == Some(&stem) {
            self.none = None;
        }
        let mut up = |face: &slot_ui::CartFace| match self.free.pop() {
            Some(tex) => {
                compositor.update_texture(tex, face.w, face.h, &face.rgba);
                tex
            }
            None => compositor.create_texture(face.w, face.h, &face.rgba),
        };
        let frames = built
            .frames
            .iter()
            .map(|f| {
                (
                    up(&f.face),
                    [f.x as f32, f.y as f32, f.face.w as f32, f.face.h as f32],
                )
            })
            .collect();
        let seated = built.seated.as_ref().map(&mut up);
        if let Some(old) = app.set_turn(TurnTex {
            platform: stem.0,
            stem: stem.1,
            frames,
            seated,
        }) {
            self.free.extend(old.textures());
        }
    }
}

fn work(inbox: &Receiver<Cart>, outbox: &Sender<(Key, Option<TurnFaces>)>, root: &Path) {
    while let Ok(mut cart) = inbox.recv() {
        // Only the newest ask matters: the selection may have moved on meanwhile.
        while let Ok(newer) = inbox.try_recv() {
            cart = newer;
        }
        cart.read_header();
        let built = if cart.platform == Platform::Gw {
            gw_strip(root, &cart)
        } else {
            faces(&cart, label_art(root, &cart))
        };
        if outbox
            .send(((cart.platform, cart.stem.clone()), built))
            .is_err()
        {
            return;
        }
    }
}

/// A Game & Watch's own frames, `Labels/GW/open/<stem>.png`; none when it has no strip.
fn gw_strip(root: &Path, cart: &Cart) -> Option<TurnFaces> {
    let path = root
        .join("Labels")
        .join(cart.platform.dir_name())
        .join(GW_OPEN_DIR)
        .join(format!("{}.png", cart.stem));
    let bytes = std::fs::read(&path).ok()?;
    let built = gw_faces(&bytes);
    if built.is_none() {
        eprintln!(
            "slot: {}: not {} frames of the shelf face's size",
            path.display(),
            slot_ui::turn::GW_FRAMES
        );
    }
    built
}
