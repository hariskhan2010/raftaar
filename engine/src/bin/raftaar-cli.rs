//! Debug CLI for testing the engine on a real machine (use Windows Sandbox
//! for anything that writes). Usage: raftaar-cli <scan|score|list|apply ID|revert ID|revert-all|restore-point>

use engine::catalog::catalog;
use engine::snapshot::SnapshotStore;
use engine::sys::{RealShell, WinRegistry};
use engine::tweak::Engine;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let snap = std::env::temp_dir().join("raftaar-cli-snapshots.json");
    let (reg, shell) = (WinRegistry, RealShell);
    let engine = || Engine::new(&reg, &shell, SnapshotStore::load(snap.clone()).expect("snapshot file"));

    match args.first().map(String::as_str) {
        Some("scan") => println!("{:#?}", engine::scan::scan()),
        Some("score") => {
            let s = engine::score::measure();
            println!("{s:#?}\nscore = {}", engine::score::score(&s));
        }
        Some("list") => catalog().iter().for_each(|t| println!("{:<22} {:?}  {}", t.id, t.risk, t.name_en)),
        Some("apply") => {
            let id = args.get(1).expect("tweak id");
            let t = catalog().into_iter().find(|t| &t.id == id).expect("unknown tweak");
            println!("{:?}", engine().apply(&t));
        }
        Some("revert") => println!("{:?}", engine().revert(args.get(1).expect("tweak id"))),
        Some("revert-all") => println!("{:?}", engine().revert_all()),
        Some("restore-point") => println!("{:?}", engine::restore::create_restore_point(&shell, &reg, "Raftaar test")),
        _ => eprintln!("usage: raftaar-cli <scan|score|list|apply ID|revert ID|revert-all|restore-point>"),
    }
}
