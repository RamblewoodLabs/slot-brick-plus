use crate::Platform;

/// A row of carts: the combined Favorites shelf, or one system's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShelfKind {
    Favorites,
    System(Platform),
}

impl ShelfKind {
    pub fn name(self) -> &'static str {
        match self {
            ShelfKind::Favorites => "Favorites",
            ShelfKind::System(platform) => platform.name(),
        }
    }

    /// The system this shelf is for; none on the combined Favorites shelf.
    pub fn system(self) -> Option<Platform> {
        match self {
            ShelfKind::Favorites => None,
            ShelfKind::System(platform) => Some(platform),
        }
    }
}
