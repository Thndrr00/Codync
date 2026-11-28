//! Pure decision for which files a downloaded ACP build must mark executable (#15).

use std::path::PathBuf;

/// Every program in a downloaded build needs the executable bit on Unix, not just the main `cmd`:
/// a build with several programs otherwise starts the main one and fails the rest with
/// "Permission denied". Order is kept and duplicates are removed.
pub(crate) fn binaries_to_mark(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut marked: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for path in paths {
        if path.as_os_str().is_empty() || path.file_name().is_none() {
            continue;
        }
        if !marked.contains(path) {
            marked.push(path.clone());
        }
    }
    marked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_programs_marks_nothing() {
        assert!(binaries_to_mark(&[]).is_empty());
    }

    #[test]
    fn one_program_is_marked() {
        assert_eq!(binaries_to_mark(&[PathBuf::from("bin/agent")]), [PathBuf::from("bin/agent")]);
    }

    #[test]
    fn every_program_is_marked_and_duplicates_are_dropped() {
        let paths = [
            PathBuf::from("bin/agent"),
            PathBuf::from("bin/helper"),
            PathBuf::from("bin/agent"),
            PathBuf::from("lib/node/bin/tool"),
        ];
        assert_eq!(
            binaries_to_mark(&paths),
            [PathBuf::from("bin/agent"), PathBuf::from("bin/helper"), PathBuf::from("lib/node/bin/tool")]
        );
    }

    #[test]
    fn paths_without_a_file_name_are_skipped() {
        assert_eq!(binaries_to_mark(&[PathBuf::new(), PathBuf::from(".")]), Vec::<PathBuf>::new());
    }
}
