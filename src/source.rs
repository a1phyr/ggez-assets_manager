use std::{borrow::Cow, io, sync::Arc};

use assets_manager::{
    hot_reloading::{DynUpdateSender, EventSender, FsWatcherBuilder},
    source::{self, DirEntry, Source},
};

/// A [`Source`] using `ggez`' paths to read from the filesystem.
///
/// See [`ggez::filesystem`] for more details.
///
/// When hot-reloading is activated, changes to `"resources.zip"` are ignored.
#[derive(Debug, Clone)]
pub struct GgezFileSystem {
    resources: Option<source::FileSystem>,
    zip: Option<Arc<source::Zip>>,
    config: Option<source::FileSystem>,
}

fn no_valid_source_error() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Cannot find a valid source")
}

impl GgezFileSystem {
    /// Creates a new `FileSystem` from `ggez` context.
    ///
    /// Note that additionnal
    #[inline]
    pub fn from_context(fs: &impl ggez::context::Has<ggez::filesystem::Filesystem>) -> Self {
        fn inner(fs: &ggez::filesystem::Filesystem) -> GgezFileSystem {
            let resources = source::FileSystem::new(fs.resources_dir()).ok();
            let zip = source::Zip::open(fs.zip_dir()).ok().map(Arc::new);
            let config = source::FileSystem::new(fs.user_data_dir()).ok();

            GgezFileSystem {
                resources,
                zip,
                config,
            }
        }

        inner(fs.retrieve())
    }

    /// Creates a new `FileSystem`.
    ///
    /// `game_id` and `author` parameters should be the same as thoses given to
    /// [`ggez::ContextBuilder::new`].
    #[deprecated = "use `GgezFileSystem::from_context` instead"]
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
        if let Some(source) = &self.zip {
            match source.read(id, ext) {
                Err(e) => err = Some(e),
                content => return content,
            }
        }
        if let Some(source) = &self.config {
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
        if let Some(source) = &self.zip {
            match source.read_dir(id, f) {
                Err(e) => err = Some(e),
                content => return content,
            }
        }
        if let Some(source) = &self.config {
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

    fn make_source(&self) -> Option<Box<dyn Source + Send>> {
        // Disable hot-reloading if there is no asset directory
        if self.resources.is_none() && self.config.is_none() {
            None
        } else {
            Some(Box::new(self.clone()))
        }
    }

    fn configure_hot_reloading(
        &self,
        events: EventSender,
    ) -> Result<DynUpdateSender, assets_manager::BoxedError> {
        let mut watcher = FsWatcherBuilder::new()?;
        if let Some(res) = &self.resources {
            watcher.watch(res.root().to_owned())?;
        }
        if let Some(res) = &self.config {
            watcher.watch(res.root().to_owned())?;
        }
        Ok(watcher.build(events))
    }
}
