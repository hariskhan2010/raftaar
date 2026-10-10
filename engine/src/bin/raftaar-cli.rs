//! Debug and owner CLI.
//!
//! Testing the engine on a real machine (use Windows Sandbox for anything that writes):
//!   raftaar-cli <scan|score|list|apply ID|revert ID|revert-all|restore-point>
//!
//! Selling Premium (owner only):
//!   raftaar-cli license-keygen                  one time: creates the signing key pair
//!   raftaar-cli license-new <name> <days|lifetime>   prints a key to send to a customer
//!   raftaar-cli license-check <key>             shows what a key contains

use engine::catalog::catalog;
use engine::license::{Claims, Plan};
use engine::snapshot::SnapshotStore;
use engine::sys::{RealShell, WinRegistry};
use engine::tweak::Engine;

const USAGE: &str = "usage: raftaar-cli <scan|score|list|apply ID|revert ID|revert-all|restore-point|\
                     license-keygen|license-new NAME DAYS|lifetime|license-check KEY>";

fn license_key_path() -> std::path::PathBuf {
    let home = std::env::var_os("USERPROFILE").expect("USERPROFILE");
    std::path::Path::new(&home).join(".tauri").join("raftaar-license.key")
}

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
        Some("license-keygen") => {
            let path = license_key_path();
            if path.exists() {
                eprintln!("{} already exists. Refusing to overwrite: every key sold so far depends on it.", path.display());
                std::process::exit(1);
            }
            let (secret, public) = engine::license::generate_keypair();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &secret).unwrap();
            println!("Private key saved to {} (back it up; never share it).", path.display());
            println!("Public key (put in engine/src/license.rs PUBLIC_KEY_HEX):\n{public}");
        }
        Some("license-new") => {
            let name = args.get(1).expect("customer name or phone");
            let len = args.get(2).map(String::as_str).unwrap_or("30");
            let now = engine::snapshot::now_unix();
            let (plan, expires) = match len {
                "lifetime" => (Plan::Lifetime, 0),
                d => {
                    let days: u64 = d.parse().expect("days must be a number or 'lifetime'");
                    (if days >= 365 { Plan::Yearly } else { Plan::Monthly }, now + days * 86_400)
                }
            };
            let secret = std::fs::read_to_string(license_key_path()).expect("run license-keygen first");
            let key = engine::license::issue(&secret, &Claims { n: name.clone(), p: plan, e: expires, t: now }).unwrap();
            println!("{key}");
        }
        Some("license-check") => {
            let key = args.get(1).expect("key");
            let pk = ed25519_public();
            match engine::license::verify_raftaar(key, &pk) {
                Ok(c) => println!("valid: {c:?}"),
                Err(e) => println!("INVALID: {}", e.0),
            }
        }
        _ => eprintln!("{USAGE}"),
    }
}

fn ed25519_public() -> ed25519_dalek::VerifyingKey {
    let bytes = engine::license::hex32(engine::license::PUBLIC_KEY_HEX).expect("embedded key");
    ed25519_dalek::VerifyingKey::from_bytes(&bytes).expect("embedded key")
}
