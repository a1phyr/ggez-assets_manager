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
pub struct GgezFileSystem {
    resources: Option<source::FileSystem>,
    zip: Option<Arc<source::Zip<fs::File>>>,
    config: Option<source::FileSystem>,
}

fn no_valid_source_error() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Cannot find a valid source")
}

impl GgezFileSystem {
    /// Creates a new `FileSystem`.
    ///
    /// `game_id` and `author` parameters should be the same as thoses given to
    /// [`ggez::ContextBuilder::new`].
    pub fn new(game_id: &str, author: &str) -> Self {
        let resources = source::FileSystem::new("resources").ok();
        let zip = source::Zip::open("resources.zip").ok().map(Arc::new);

        let config = directories::ProjectDirs::from("", author, game_id)
            .and_then(|project_dir| source::FileSystem::new(project_dir.data_local_dir()).ok());

        Self {
            resources,
            zip,
            config,
        }
    }
}

impl Source for GgezFileSystem {
    fn read(&self, id: &str, ext: &str) -> io::Result<Cow<[u8]>> {
        let mut err = None;

        if let Some(source) = &self.resources {
            match source.read(id, ext) {
                Err(e) => err = Some(e),
                content => return content,
            };
        }
        if let Some(source) = &self.resources {
            match source.read(id, ext) {
                Err(e) => err = Some(e),
                content => return content,
            }
        }
        if let Some(source) = &self.resources {
            match source.read(id, ext) {
                Err(e) => err = Some(e),
                content => return content,
            }
        }

        Err(err.unwrap_or_else(no_valid_source_error))
    }

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        let mut err = None;

        if let Some(source) = &self.resources {
            match source.read_dir(id, f) {
                Err(e) => err = Some(e),
                content => return content,
            };
        }
        if let Some(source) = &self.resources {
            match source.read_dir(id, f) {
                Err(e) => err = Some(e),
                content => return content,
            }
        }
        if let Some(source) = &self.resources {
            match source.read_dir(id, f) {
                Err(e) => err = Some(e),
                content => return content,
            }
        }

        Err(err.unwrap_or_else(no_valid_source_error))
    }

    fn exists(&self, entry: DirEntry) -> bool {
        fn exists<S: Source>(s: &Option<S>, entry: DirEntry) -> bool {
            s.as_ref().map_or(false, |s| s.exists(entry))
        }

        exists(&self.resources, entry) || exists(&self.zip, entry) || exists(&self.config, entry)
    }

    #[cfg(feature = "hot-reloading")]
    fn configure_hot_reloading(
        &self,
    ) -> Result<Option<hot_reloading::HotReloader>, assets_manager::BoxedError> {
        if self.resources.is_none() && self.config.is_none() {
            return Ok(None);
        }

        let mut watcher = hot_reloading::FsWatcherBuilder::new()?;
        if let Some(res) = &self.resources {
            watcher.watch(res.root().to_owned())?;
        }
        if let Some(res) = &self.config {
            watcher.watch(res.root().to_owned())?;
        }
        let config = watcher.build();
        Ok(Some(hot_reloading::HotReloader::start(
            config,
            self.clone(),
        )))
    }
}
