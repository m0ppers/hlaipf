extern crate git2;
extern crate docopt;
extern crate rustc_serialize;
extern crate regex;

use git2::{Repository, Error, Oid, Tree, ObjectType, Commit, Diff, DiffOptions};
use docopt::Docopt;
use std::fs;
use std::path::{PathBuf, Path};

fn diff_contains_php(repo: Repository, from: Tree, to: Tree) -> bool {
    let mut opts = DiffOptions::new();
    
    let fail_repo = Repository::open(repo.path());
    if fail_repo.is_ok() {
        let diff = Diff::tree_to_tree(&fail_repo.unwrap(), Some(&from), Some(&to), Some(&mut opts));
        return diff.unwrap().deltas().any(|delta| delta.new_file().path().unwrap().to_str().unwrap().ends_with(".php"))
    }
    false
}

fn is_php(repo: Repository, commit: Commit) -> bool {
    println!("Parents: {}", commit.parents().count());
    match commit.parents().count() {
        // mop: todo..first commit. need to find out how to handle that
        0 => false,
        1 => diff_contains_php(repo, commit.parent(0).unwrap().tree().unwrap(), commit.tree().unwrap()),
        _ => false,
    }
}

fn find_php<'a>(repo: &'a Repository) -> Result<Option<Commit<'a>>, Error> {
    return Ok(None);
    /*let headrev = try!(repo.revparse_single("HEAD"));
    let headoid = headrev.id();
    
    let mut revwalk = try!(repo.revwalk());
    revwalk.set_sorting(git2::SORT_TIME);
    revwalk.push(headoid);
*/
    /*for id in revwalk {
        let commit = try!(repo.find_commit(id));
        if commit.author().email().unwrap().contains("andreas.streichardt@gmail.com") {
            println!("ID => {}, author => {}, tree_id => {}", id, commit.author(), commit.tree_id());
            
            //if is_php(repo, commit) {
                return Ok(Some(commit))
            //}
        }
    }
    Ok(None)
    */
}

// mop: a mess..it SHOULD be an abstract collection of repositories like "this is my project directory" "this is my github account" etc.
// hardcoded to directory right now because of incompetence
struct RepositoryCollection {
    name: String,
    read_dir_iter: fs::ReadDir
}

impl RepositoryCollection {
    fn create(name: &str) -> Option<RepositoryCollection> {
        let mut path_buf = PathBuf::from(name);
        let metadata = fs::metadata(path_buf.as_path().to_str().unwrap());
        
        // mop: later: github, bitbucket etc
        if metadata.unwrap().is_dir() {
            let path_ref = &path_buf.as_path();
            Some(RepositoryCollection {
                name: name.to_string(),
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

struct PhpCommitLocator {
    repo: Repository,
}

impl PhpCommitLocator {
    fn new(repo: Repository) -> PhpCommitLocator {
        PhpCommitLocator {
            repo: repo,
        }
    }

    fn fetch_earliest_php_commit(&mut self) -> Option<HlaipfResult> {
        None
    }
}

struct HlaipfResult<'repo> {
    repo: Repository,
    commit: Commit<'repo>,
}

impl<'repo> HlaipfResult<'repo> {
    fn new(repo: Repository, commit: Commit) -> HlaipfResult {
        HlaipfResult {
            repo: repo,
            commit: commit,
        }
    }

    fn is_php(&self) -> bool {
        true
    }
}

#[derive(RustcDecodable)]
struct Args {
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
     
    let results:Vec<HlaipfResult> = repository_collections.fold(Vec::new(), |mut results, repository_collection| {
        repository_collection
        .map(|repository| {
            PhpCommitLocator::new(repository)
        })
        .map(|mut php_commit_locator: PhpCommitLocator| {
            php_commit_locator.fetch_earliest_php_commit()
        })
        .filter(|result: &Option<HlaipfResult>| {
            result.is_some()
        })
        .map(|result: Option<HlaipfResult>| {
            result.unwrap()
        })
        .fold(results, |mut results, result: HlaipfResult| {
            results.push(result);
            results
        })
    });

    /*for repository_collection in repository_collections {
        for repository in repository_collection {
            let find_result = find_php(&repository);
            if find_result.is_ok() {
                // mop: XXX silent error :S
                let php_commit = find_result.unwrap();
                if php_commit.is_some() {
                    let result = HlaipfResult { repo: &repository, commit: &php_commit.unwrap() };
                    results.push(result);
                }
            }
        }
    }
    */

    for result in results.iter() {
        println!("RESULT {}", result.commit.author())
    }
    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: hlaipf <repositorieslocation>...
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
