extern crate git2;
extern crate docopt;
extern crate rustc_serialize;
extern crate regex;

use git2::{Repository, Error, Oid, Tree, ObjectType, Commit, Diff, DiffOptions};
use docopt::Docopt;
use std::fs;
use std::path::{PathBuf, Path};

fn diff_contains_php(repo: &Repository, from: Tree, to: Tree) -> bool {
    let mut opts = DiffOptions::new();
    let diff = Diff::tree_to_tree(repo, Some(&from), Some(&to), Some(&mut opts));
    
    diff.unwrap().deltas().any(|delta| delta.new_file().path().unwrap().to_str().unwrap().ends_with(".php"))
}

fn is_php(repo: &Repository, commit: Commit) -> bool {
    println!("Parents: {}", commit.parents().count());
    match commit.parents().count() {
        0 => false,
        1 => diff_contains_php(repo, commit.parent(0).unwrap().tree().unwrap(), commit.tree().unwrap()),
        _ => false,
    }
}

fn find_php(repo: Repository) -> Result<(), Error> {
    let headrev = try!(repo.revparse_single("HEAD"));
    let headoid = headrev.id();
    
    let mut revwalk = try!(repo.revwalk());
    revwalk.set_sorting(git2::SORT_TIME);
    revwalk.push(headoid);
    
    for id in revwalk {
        let commit = try!(repo.find_commit(id));
        if commit.author().email().unwrap().contains("andreas.streichardt@gmail.com") {
            println!("ID => {}, author => {}, tree_id => {}", id, commit.author(), commit.tree_id());
            
            if is_php(&repo, commit) {
                println!("JA")
            }
        }
    }
    Ok(())
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

    for repository_collection in repository_collections {
        println!("COLLECTION NAME => {}", repository_collection.name);

        for repository in repository_collection {
            println!("HEHE {:?}", repository.path());
            find_php(repository);
        }
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
