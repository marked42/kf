use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::args::GrepArgs;

pub struct FilesFinder<'a> {
    files: &'a [PathBuf],
    recursive: bool,
}

impl<'a> FilesFinder<'a> {
    pub fn from_args(args: &'a GrepArgs) -> Self {
        Self {
            files: &args.files,
            recursive: args.recursive,
        }
    }

    // TODO: use iterator to avoid collecting all files at once
    pub fn find_files(&self) -> Vec<std::io::Result<PathBuf>> {
        self.files.iter().flat_map(|path| self.find_files_at_path(path)).collect()
    }

    pub fn find_files_at_path(&self, path: &Path) -> Vec<std::io::Result<PathBuf>> {
        let mut result = vec![];
        let metadata = fs::metadata(path);

        match metadata {
            Ok(f) => {
                if f.is_file() {
                    result.push(Ok(path.to_path_buf()));
                } else if f.is_dir() {
                    if self.recursive {
                        match self.find_files_in_dir(&path) {
                            Err(e) => result.push(Err(e)),
                            Ok(sub_files) => {
                                result.extend(sub_files.into_iter().map(Ok));
                            }
                        }
                    } else {
                        result.push(Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!(
                                "{} is a directory, use -r to search recursively",
                                path.display()
                            ),
                        )));
                    }
                }
            }
            Err(e) => {
                result.push(Err(e));
            }
        }

        result
    }

    fn find_files_in_dir<P: AsRef<Path>>(&self, dir_path: &P) -> io::Result<Vec<PathBuf>> {
        let mut files = vec![];

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() && self.recursive {
                let mut nested_files = self.find_files_in_dir(&path)?;
                files.append(&mut nested_files);
            }
        }

        Ok(files)
    }
}
