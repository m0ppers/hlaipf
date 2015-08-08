extern crate docopt;
extern crate rustc_serialize;
extern crate time;
extern crate git2;

use hlaipf::{RepositoryCollection, HlaipfResult, PhpCommitLocator};
use docopt::Docopt;
use self::time::{Timespec, at_utc};
use self::git2::{Error};

pub mod hlaipf;

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
