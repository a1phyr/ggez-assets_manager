use std::io;

use assets_manager::{
    hot_reloading::{EventSender, FsWatcherBuilder},
    source::{DirEntry, FileContent, Source},
};

/// A [`Source`] using `ggez`' paths to read from the filesystem.
///
/// See [`ggez::filesystem`] for more details.
///
/// When hot-reloading is activated, changes to `"resources.zip"` are ignored.
#[derive(Debug)]
pub struct GgezFileSystem {
    fs: ggez::filesystem::Filesystem,
}

impl GgezFileSystem {
    /// Creates a new `FileSystem` from `ggez` context.
    ///
    /// Note that additionnal
    #[inline]
    pub fn from_context(fs: &impl ggez::context::Has<ggez::filesystem::Filesystem>) -> Self {
        Self {
            fs: fs.retrieve().clone(),
        }
    }
}

fn id_to_path(entry: DirEntry) -> String {
    let id = entry.id();

    if id.is_empty() {
        return String::from("/");
    }

    let mut path = String::with_capacity(id.len() + 10);

    for comp in id.split('.') {
        path.push('/');
        path.push_str(comp);
    }

    match entry {
        DirEntry::File(_, ext) => {
            path.push('.');
            path.push_str(ext);
        }
        DirEntry::Directory(_) => path.push('/'),
    }

    path
}

fn split_file_name(path: &std::path::Path) -> Option<(&str, &str)> {
    let name = path.file_name()?.to_str()?;
    match name.split_once('.') {
        Some(("", _)) => None,
        Some(res) => Some(res),
        None => Some((name, "")),
    }
}

impl Source for GgezFileSystem {
    fn read(&self, id: &str, ext: &str) -> io::Result<FileContent<'_>> {
        let data = self
            .fs
            .read(id_to_path(DirEntry::File(id, ext)))
            .map_err(io::Error::other)?;
        Ok(FileContent::Buffer(data))
    }

    fn read_dir(&self, id: &str, f: &mut dyn FnMut(DirEntry)) -> io::Result<()> {
        let contents = self
            .fs
            .read_dir(id_to_path(DirEntry::Directory(id)))
            .map_err(io::Error::other)?;

        let mut base_id = id.to_owned() + ".";

        contents.iter().for_each(|path| {
            let Some((name, ext)) = split_file_name(path) else {
                return;
            };

            base_id.truncate(id.len() + 1);
            let this_id: &str = if !id.is_empty() {
                base_id.truncate(id.len() + 1);
                base_id.push_str(name);
                &base_id
            } else {
                name
            };

            if self.fs.is_dir(path) {
                f(assets_manager::source::DirEntry::Directory(this_id))
            } else {
                f(assets_manager::source::DirEntry::File(this_id, ext))
            }
        });

        Ok(())
    }

    fn exists(&self, entry: DirEntry) -> bool {
        self.fs.exists(id_to_path(entry))
    }

    fn configure_hot_reloading(
        &self,
        events: EventSender,
    ) -> Result<(), assets_manager::BoxedError> {
        let mut watcher = FsWatcherBuilder::new()?;
        let _ = watcher.watch(self.fs.resources_dir().to_owned());
        let _ = watcher.watch(self.fs.user_data_dir().to_owned());
        let _ = watcher.watch(self.fs.user_config_dir().to_owned());
        watcher.build(events);
        Ok(())
    }
}
