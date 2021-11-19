use std::{borrow::Cow, fs, io, sync::Arc};

#[cfg(feature = "hot-reloading")]
use assets_manager::hot_reloading;
use assets_manager::source::{self, DirEntry, Source};

/// A [`Source`] using `ggez`' paths to read from the filesystem.
///
/// See [`ggez::filesystem`] for more details.
///
/// When hot-reloading is activated, changes to `"resources.zip"` are ignored.
#[derive(Debug, Clone)]
pub struct FileSystem {
    resources: source::FileSystem,
    zip: Arc<source::Zip<fs::File>>,
    config: source::FileSystem,
}

impl FileSystem {
    /// Creates a new `FileSystem`.
    ///
    /// `game_id` and `author` parameters should be the same as thoses given to
    /// [`ggez::ContextBuilder::new`].
    pub fn new(game_id: &str, author: &str) -> io::Result<Self> {
        let resources = source::FileSystem::new("resources")?;
        let zip = Arc::new(source::Zip::open("resources.zip")?);

        let project_dir = directories::ProjectDirs::from("", author, game_id).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "could not find home directory")
        })?;
        let config = source::FileSystem::new(project_dir.data_local_dir())?;

        Ok(Self {
            resources,
            zip,
            config,
        })
    }
}

impl Source for FileSystem {
    fn read(&self, id: &str, ext: &str) -> io::Result<Cow<[u8]>> {
        match self.resources.read(id, ext) {
            Err(_) => (),
            content => return content,
        };
        match self.zip.read(id, ext) {
            Err(_) => (),
            content => return content,
        };
        self.config.read(id, ext)
    }

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        let mut had_err = true;

        if self.resources.read_dir(id, f).is_ok() {
            had_err = false;
        }

        if self.zip.read_dir(id, f).is_ok() {
            had_err = false;
        }

        let err = self.resources.read_dir(id, f);
        if had_err {
            Ok(())
        } else {
            err
        }
    }

    fn exists(&self, entry: DirEntry) -> bool {
        self.resources.exists(entry) || self.zip.exists(entry) || self.config.exists(entry)
    }

    #[cfg(feature = "hot-reloading")]
    fn configure_hot_reloading(
        &self,
    ) -> Result<Option<hot_reloading::HotReloader>, assets_manager::BoxedError> {
        let mut watcher = hot_reloading::FsWatcherBuilder::new()?;
        watcher.watch(self.resources.root().to_owned())?;
        watcher.watch(self.config.root().to_owned())?;
        let config = watcher.build();
        Ok(Some(hot_reloading::HotReloader::start(
            config,
            self.clone(),
        )))
    }
}
