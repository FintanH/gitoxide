use std::fs;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use crate::file::{cow_str, from_paths::escape_backslashes};
use git_config::file::from_paths;
use git_config::File;
use tempfile::{tempdir, tempdir_in};

pub fn create_symlink(from: &Path, to: &Path) {
    create_dir_all(from.parent().unwrap()).unwrap();
    #[cfg(not(target_os = "windows"))]
    std::os::unix::fs::symlink(to, &from).unwrap();
    #[cfg(target_os = "windows")]
    std::os::windows::fs::symlink_file(to, &from).unwrap();
}

fn canonicalized_tempdir() -> crate::Result<tempfile::TempDir> {
    #[cfg(windows)]
    let canonicalized_tempdir = std::env::temp_dir();
    #[cfg(not(windows))]
    let canonicalized_tempdir = std::env::temp_dir().canonicalize()?;

    Ok(tempdir_in(canonicalized_tempdir)?)
}

mod gitdir;
mod onbranch;

#[test]
fn include_and_includeif_correct_inclusion_order() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("p");
    let first_include_path = dir.path().join("first-incl");
    let second_include_path = dir.path().join("second-incl");
    let include_if_path = dir.path().join("incl-if");
    fs::write(
        first_include_path.as_path(),
        "
[core]
  b = first-incl-path",
    )
    .unwrap();

    fs::write(
        second_include_path.as_path(),
        "
[core]
  b = second-incl-path",
    )
    .unwrap();

    fs::write(
        include_if_path.as_path(),
        "
[core]
  b = incl-if-path",
    )
    .unwrap();

    fs::write(
        config_path.as_path(),
        format!(
            r#"
[core]
[include]
  path = {}
[includeIf "gitdir:p/"]
  path = {}
[include]
  path = {}"#,
            escape_backslashes(&first_include_path),
            escape_backslashes(&include_if_path),
            escape_backslashes(&second_include_path),
        ),
    )
    .unwrap();

    let dir = config_path.join(".git");
    let config = File::from_paths(Some(&config_path), options_with_git_dir(&dir)).unwrap();

    assert_eq!(
        config.strings("core", None, "b"),
        Some(vec![
            cow_str("first-incl-path"),
            cow_str("incl-if-path"),
            cow_str("second-incl-path")
        ]),
        "first include is matched correctly",
    );
    assert_eq!(
        config.string("core", None, "b"),
        Some(cow_str("second-incl-path")),
        "second include is matched after incl-if",
    );
}

#[test]
fn pattern_is_current_dir() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("a");
    let relative_dot_slash_path = dir.path().join("g");
    fs::write(
        relative_dot_slash_path.as_path(),
        "
[core]
  b = relative-dot-slash-path",
    )
    .unwrap();

    fs::write(
        config_path.as_path(),
        format!(
            r#"
[core]
  x = 1
[includeIf "gitdir:./"]
  path = {}"#,
            escape_backslashes(&relative_dot_slash_path),
        ),
    )
    .unwrap();

    {
        let dir = config_path.parent().unwrap().join("p").join("q").join(".git");
        let config = File::from_paths(Some(&config_path), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.string("core", None, "b"),
            Some(cow_str("relative-dot-slash-path")),
            "relative path pattern is matched correctly"
        );
    }
}

#[test]
fn gitdir() {
    let dir = tempdir().unwrap();

    let config_path = dir.path().join("a");
    let absolute_path = dir.path().join("b");
    let home_dot_git_path = dir.path().join("c");
    let foo_trailing_slash_path = dir.path().join("foo_slash");
    let wildcards_path = dir.path().join("d");
    let relative_path = dir.path().join("e");
    let casei_path = dir.path().join("i");
    let relative_dot_git_path = dir.path().join("w");
    let relative_with_backslash_path = dir.path().join("x");
    let tmp_path = dir.path().join("tmp");
    let dot_dot_path = dir.path().join("dot_dot");
    let dir = canonicalized_tempdir().unwrap();
    let tmp_dir_m_n_with_slash = format!(
        "{}/",
        dir.path().join("m").join("n").to_str().unwrap().replace('\\', "/")
    );

    fs::write(
        config_path.as_path(),
        format!(
            r#"
[core]
  a = 1
  b = 1
  c = 1
  d = 1
  i = 1
  t = 1
  x = 1
[includeIf "gitdir/i:a/B/c/D/"]
  path = {}
[includeIf "gitdir:c\\d/"]
  path = {}
[includeIf "gitdir:foo/bar"]
  path = {}
[includeIf "gitdir:.."]
  path = {}
[includeIf "gitdir:w/.git"]
  path = {}
[includeIf "gitdir:~/"]
  path = {}
[includeIf "gitdir:foo/"]
  path = {}
[includeIf "gitdir:stan?ard/glo*ng/[xwz]ildcards/.git"]
  path = {}
[includeIf "gitdir:{}"]
  path = {}
[includeIf "gitdir:/e/x/"]
  path = {}"#,
            escape_backslashes(&casei_path),
            escape_backslashes(&relative_with_backslash_path),
            escape_backslashes(&relative_path),
            escape_backslashes(&dot_dot_path),
            escape_backslashes(&relative_dot_git_path),
            escape_backslashes(&home_dot_git_path),
            escape_backslashes(&foo_trailing_slash_path),
            escape_backslashes(&wildcards_path),
            &tmp_dir_m_n_with_slash,
            escape_backslashes(&tmp_path),
            escape_backslashes(&absolute_path),
        ),
    )
    .unwrap();

    fs::write(
        casei_path.as_path(),
        "
[core]
  i = case-i-match",
    )
    .unwrap();

    fs::write(
        relative_with_backslash_path.as_path(),
        "
[core]
  c = relative with backslash do not match",
    )
    .unwrap();

    fs::write(
        absolute_path.as_path(),
        "
[core]
  b = absolute-path",
    )
    .unwrap();

    fs::write(
        home_dot_git_path.as_path(),
        "
[core]
  b = home-dot-git",
    )
    .unwrap();

    fs::write(
        wildcards_path.as_path(),
        "
[core]
  b = standard-globbing-wildcards",
    )
    .unwrap();

    fs::write(
        relative_path.as_path(),
        "
[core]
  a = relative-path",
    )
    .unwrap();

    fs::write(
        dot_dot_path.as_path(),
        "
[core]
  d = dot-dot-path",
    )
    .unwrap();

    fs::write(
        relative_dot_git_path.as_path(),
        "
[core]
  a = relative-dot-git",
    )
    .unwrap();

    fs::write(
        foo_trailing_slash_path.as_path(),
        "
[core]
  b = foo-trailing-slash",
    )
    .unwrap();

    fs::write(
        tmp_path.as_path(),
        "
[core]
  t = absolute-path-with-symlink",
    )
    .unwrap();

    {
        let dir = Path::new("/A/b/C/d/.git");
        let config = File::from_paths(Some(&config_path), options_with_git_dir(dir)).unwrap();
        assert_eq!(
            config.string("core", None, "i"),
            Some(cow_str("case-i-match")),
            "case insensitive patterns match"
        );
    }

    {
        let dir = Path::new("/c//d/.git");
        let config = File::from_paths(Some(&config_path), options_with_git_dir(dir)).unwrap();
        assert_eq!(
            config.integer("core", None, "c"),
            Some(Ok(1)),
            "relative with backslash do not match"
        );
    }

    {
        let dir = config_path.join("foo").join("bar");
        let config = File::from_paths(Some(&config_path), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.string("core", None, "a"),
            Some(cow_str("relative-path")),
            "the pattern is prefixed and suffixed with ** to match GIT_DIR containing it in the middle"
        );
    }

    {
        let dir = dot_dot_path.parent().unwrap();
        let config = File::from_paths(Some(&config_path), options_with_git_dir(dir)).unwrap();
        assert_eq!(
            config.string("core", None, "d"),
            Some(cow_str("dot-dot-path")),
            ".. path is included"
        );
    }

    {
        let dir = PathBuf::from("C:\\w\\.git".to_string());
        let config = File::from_paths(Some(&config_path), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.string("core", None, "a"),
            Some(cow_str("relative-dot-git")),
            "backslashes in GIT_DIR are converted to forward slashes"
        );
    }

    {
        let dir = dirs::home_dir().unwrap().join(".git");
        let config = File::from_paths(Some(&config_path), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.strings("core", None, "b"),
            Some(vec![cow_str("1"), cow_str("home-dot-git")]),
            "tilde ~ path is resolved to home directory"
        );
    }

    {
        let dir = config_path.join("foo").join(".git");
        let config = File::from_paths(Some(&config_path), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.string("core", None, "b"),
            Some(cow_str("foo-trailing-slash")),
            "path with trailing slash is matched"
        );
    }

    {
        let dir = dir
            .path()
            .join("standard")
            .join("globbing")
            .join("wildcards")
            .join(".git");
        let config = File::from_paths(Some(&config_path), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.string("core", None, "b"),
            Some(cow_str("standard-globbing-wildcards")),
            "standard globbing wildcards works"
        );
    }

    {
        let dir = dirs::home_dir().unwrap().join(".git");
        let config = File::from_paths(Some(config_path.as_path()), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.string("core", None, "b"),
            Some(cow_str("home-dot-git")),
            "absolute path pattern is matched with sub path from GIT_DIR"
        );
    }

    {
        let dir = canonicalized_tempdir().unwrap();
        let symlink_outside_tempdir_m_n = dir.path().join("m").join("symlink");
        create_symlink(
            &symlink_outside_tempdir_m_n,
            &PathBuf::from(&format!("{}.git", tmp_dir_m_n_with_slash)),
        );
        let dir = PathBuf::from(&symlink_outside_tempdir_m_n);
        let config = File::from_paths(Some(config_path), options_with_git_dir(&dir)).unwrap();
        assert_eq!(
            config.string("core", None, "t"),
            Some(cow_str("absolute-path-with-symlink")),
            "absolute path pattern is matched with path from GIT_DIR when it contains symlink"
        );
        fs::remove_file(symlink_outside_tempdir_m_n.as_path()).unwrap();
    }
}

fn options_with_git_dir(git_dir: &Path) -> from_paths::Options<'_> {
    from_paths::Options {
        git_dir: Some(git_dir),
        ..Default::default()
    }
}