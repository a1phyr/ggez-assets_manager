use std::{io, sync::Arc};

use assets_manager::{
    hot_reloading::{EventSender, FsWatcherBuilder},
    source::{self, DirEntry, FileContent, Source},
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
    local: Option<source::FileSystem>,
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
            let local = source::FileSystem::new(fs.user_data_dir()).ok();
            let config = source::FileSystem::new(fs.user_config_dir()).ok();

            GgezFileSystem {
                resources,
                zip,
                local,
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

        let (local, config) = match directories::ProjectDirs::from("", author, game_id) {
            Some(project_dir) => (
                source::FileSystem::new(project_dir.data_local_dir()).ok(),
                source::FileSystem::new(project_dir.config_dir()).ok(),
            ),
            None => (None, None),
        };

        Self {
            resources,
            zip,
            local,
            config,
        }
    }
}

impl Source for GgezFileSystem {
    fn read(&self, id: &str, ext: &str) -> io::Result<FileContent> {
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
        if let Some(source) = &self.local {
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
        if let Some(source) = &self.local {
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

        exists(&self.resources, entry)
            || exists(&self.zip, entry)
            || exists(&self.local, entry)
            || exists(&self.config, entry)
    }

    fn configure_hot_reloading(
        &self,
        events: EventSender,
    ) -> Result<(), assets_manager::BoxedError> {
        let mut watcher = FsWatcherBuilder::new()?;
        if let Some(res) = &self.resources {
            watcher.watch(res.root().to_owned())?;
        }
        if let Some(res) = &self.local {
            watcher.watch(res.root().to_owned())?;
        }
        if let Some(res) = &self.config {
            watcher.watch(res.root().to_owned())?;
        }
        watcher.build(events);
        Ok(())
    }
}
