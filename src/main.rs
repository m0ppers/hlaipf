extern crate git2;
extern crate docopt;
extern crate rustc_serialize;
extern crate regex;
extern crate time;

use git2::{Repository, Error, Oid, Tree, Commit, Diff, DiffOptions, Time};
use docopt::Docopt;
use std::fs;
use std::path::{PathBuf};
use time::{Timespec, at_utc};

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

// mop: a mess..it SHOULD be an abstract collection of repositories like "this is my project directory" "this is my github account" etc.
// hardcoded to directory right now because of incompetence
struct RepositoryCollection {
    read_dir_iter: fs::ReadDir
}

impl RepositoryCollection {
    fn create(name: &str) -> Option<RepositoryCollection> {
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

struct PhpCommitLocator<'a> {
    author: &'a str,
    repo: Repository,
}

impl<'a> PhpCommitLocator<'a> {
    fn new(author: &str, repo: Repository) -> PhpCommitLocator {
        PhpCommitLocator {
            author: author,
            repo: repo,
        }
    }

    fn fetch_earliest_php_commit(&mut self) -> Result<Option<HlaipfResult>, Error> {
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

struct HlaipfResult {
    repository_path: PathBuf,
    commit_message: Option<String>,
    commit_time: Time,
    commit_oid: Oid,
}

#[derive(RustcDecodable)]
struct Args {
    arg_author: String,
    arg_repositorieslocation: Vec<String>,
}

fn to_repository_collections(repositorylocation: &String) -> Option<RepositoryCollection> {
    RepositoryCollection::create(repositorylocation)
}

fn run(args: &Args) -> Result<(), Error> {
    let repository_collections = args.arg_repositorieslocation.iter()
        .map(|repositorylocation| to_repository_collections(repositorylocation))
        .filter(|repository_collection_opt| repository_collection_opt.is_some())
        .map(|repository_collection| repository_collection.unwrap())
    ;

    let mut results:Vec<HlaipfResult> = repository_collections
    .flat_map(|repository_collection| {
        repository_collection
    })
    .map(|repository| {
        PhpCommitLocator::new(&args.arg_author, repository)
    })
    .map(|mut php_commit_locator: PhpCommitLocator| {
        php_commit_locator.fetch_earliest_php_commit()
    })
    .filter(|result| {
        result.is_ok()
    })
    .map(|result| {
        result.unwrap()
    })
    .filter(|result: &Option<HlaipfResult>| {
        result.is_some()
    })
    .map(|result: Option<HlaipfResult>| {
        result.unwrap()
    })
    .collect();

    results.sort_by(|a, b| {
        b.commit_time.seconds().cmp(&a.commit_time.seconds())
    });
    
    if results.is_empty() {
        println!("Well done. You seem to be PHP free");
    } else {
        let ref result = results[0];
        let commit_time_spec = Timespec::new(result.commit_time.seconds(), 0);
        println!("Your last PHP commit was at {} in repository {}. Commit Id: {}", at_utc(commit_time_spec).rfc3339(), result.repository_path.as_path().to_string_lossy(), result.commit_oid);

        if let Some(ref commit_message) = result.commit_message {
            println!("===============================");
            println!("{}", commit_message);
        }
    }
    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: hlaipf <author> <repositorieslocation>...
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
