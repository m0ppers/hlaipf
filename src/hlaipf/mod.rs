extern crate git2;
extern crate regex;

use std::fs;
use self::git2::{Repository, Error, Oid, Tree, Commit, Diff, DiffOptions, Time};
use std::path::{PathBuf};


fn diff_contains_php(repo: &Repository, from: Option<&Tree>, to: Option<&Tree>) -> bool {
    let mut opts = DiffOptions::new();
    
    let diff = Diff::tree_to_tree(repo, from, to, Some(&mut opts));
    return diff.unwrap().deltas().any(|delta| delta.new_file().path().unwrap().to_str().unwrap().ends_with(".php"))
}

fn is_php(repo: &Repository, commit: Commit) -> bool {
    match commit.parents().count() {
        // mop: todo..first commit. need to find out how to handle that
        0 => diff_contains_php(repo, None, Some(&commit.tree().unwrap())),
        1 => diff_contains_php(repo, Some(&commit.parent(0).unwrap().tree().unwrap()), Some(&commit.tree().unwrap())),
        _ => false,
    }
}


pub struct PhpCommitLocator<'a> {
    author: &'a str,
    repo: Repository,
}

impl<'a> PhpCommitLocator<'a> {
    pub fn new(author: &str, repo: Repository) -> PhpCommitLocator {
        PhpCommitLocator {
            author: author,
            repo: repo,
        }
    }

    pub fn fetch_earliest_php_commit(&mut self) -> Result<Option<HlaipfResult>, Error> {
        let headrev = try!(self.repo.revparse_single("HEAD"));
        let headoid = headrev.id();

        let mut revwalk = try!(self.repo.revwalk());
        revwalk.set_sorting(git2::SORT_TIME);
        revwalk.push(headoid);
        for id in revwalk {
            let commit = try!(self.repo.find_commit(id));
            if commit.author().email().unwrap().contains(self.author) {
                // moved value workaround :S must investigate :S probably easy to fix :S
                let commit_id = commit.id();
                let commit_time = commit.time();
                let message;
                if commit.message().is_some() {
                    message = Some(commit.message().unwrap().to_string());
                } else {
                    message = None;
                }
                if is_php(&self.repo, commit) {
                    return Ok(Some(HlaipfResult {
                        repository_path: self.repo.path().to_path_buf(),
                        commit_oid: commit_id,
                        commit_time: commit_time,
                        commit_message: message
                    }))
                }
            }
        }
        Ok(None)
    }
}

pub struct HlaipfResult {
    pub repository_path: PathBuf,
    pub commit_message: Option<String>,
    pub commit_time: Time,
    pub commit_oid: Oid,
}

// mop: a mess..it SHOULD be an abstract collection of repositories like "this is my project directory" "this is my github account" etc.
// hardcoded to directory right now because of incompetence
pub struct RepositoryCollection {
    read_dir_iter: fs::ReadDir
}

impl RepositoryCollection {
    pub fn create(name: &str) -> Option<RepositoryCollection> {
        let path_buf = PathBuf::from(name);
        let metadata = fs::metadata(path_buf.as_path().to_str().unwrap());
        
        // mop: later: github, bitbucket etc
        if metadata.unwrap().is_dir() {
            Some(RepositoryCollection {
                read_dir_iter: fs::read_dir(path_buf.as_path()).unwrap(),
            })
        } else {
            None
        }
    }
}

impl Iterator for RepositoryCollection {
    type Item = Repository;

    fn next(&mut self) -> Option<Repository> {
        while let Some(entry) = self.read_dir_iter.next() {
            let repo = Repository::open(entry.unwrap().path());
            if repo.is_ok() {
                return Some(repo.unwrap());
            }
        }
        None
    }
}
