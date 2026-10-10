//! Premium licensing.
//!
//! Two kinds of keys unlock Premium:
//! - **Raftaar keys** (`RFTP-…`): signed with the owner's Ed25519 key and checked
//!   fully offline. The owner issues these by hand, e.g. after a JazzCash or
//!   Easypaisa payment (`raftaar-cli license-new`).
//! - **Lemon Squeezy keys**: emailed automatically after a card payment and checked
//!   online against Lemon Squeezy's public license API, with a 14-day offline grace.
//!
//! Revert, Safety and every free feature never depend on this module.

use std::path::PathBuf;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Public half of the owner's signing key (private half: %USERPROFILE%\.tauri\raftaar-license.key).
pub const PUBLIC_KEY_HEX: &str = "f4fce0c0ec036fbee9d7079b172f62fd749803d35871805170ec4cfacdc5b2c4";

pub const TRIAL_DAYS: u64 = 7;
const DAY: u64 = 86_400;
const OFFLINE_GRACE: u64 = 14 * DAY;
const LS_API: &str = "https://api.lemonsqueezy.com/v1/licenses";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plan {
    Monthly,
    Yearly,
    Lifetime,
}

/// What a Raftaar key says. Short field names keep keys short.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Customer name or phone, shown in the app.
    pub n: String,
    pub p: Plan,
    /// Expiry, unix seconds; 0 = never.
    pub e: u64,
    /// Issued at, unix seconds.
    pub t: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum KeySource {
    Raftaar,
    LemonSqueezy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct License {
    pub name: String,
    pub plan: Option<Plan>,
    pub expires: Option<u64>,
    pub source: KeySource,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Status {
    Trial { days_left: u64 },
    Premium { license: License },
    Free { expired: Option<License> },
}

impl Status {
    pub fn is_premium(&self) -> bool {
        matches!(self, Status::Trial { .. } | Status::Premium { .. })
    }
}

fn public_key() -> Result<VerifyingKey> {
    let bytes = hex32(PUBLIC_KEY_HEX).ok_or_else(|| Error::new("bad embedded public key"))?;
    VerifyingKey::from_bytes(&bytes).map_err(|e| Error(e.to_string()))
}

pub fn hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Creates a new owner key pair: (private hex, public hex).
pub fn generate_keypair() -> (String, String) {
    let sk = SigningKey::generate(&mut rand_core::OsRng);
    (to_hex(&sk.to_bytes()), to_hex(sk.verifying_key().as_bytes()))
}

pub fn issue(secret_hex: &str, claims: &Claims) -> Result<String> {
    let sk = SigningKey::from_bytes(&hex32(secret_hex).ok_or_else(|| Error::new("bad private key"))?);
    let payload = serde_json::to_vec(claims)?;
    let sig = sk.sign(&payload);
    Ok(format!("RFTP-{}.{}", B64.encode(&payload), B64.encode(sig.to_bytes())))
}

/// Checks a Raftaar key's signature. Expiry is judged by `status`, not here.
pub fn verify_raftaar(key: &str, pk: &VerifyingKey) -> Result<Claims> {
    let body = key.trim().strip_prefix("RFTP-").ok_or_else(|| Error::new("not a Raftaar key"))?;
    let (payload, sig) = body.split_once('.').ok_or_else(|| Error::new("malformed key"))?;
    let payload = B64.decode(payload).map_err(|_| Error::new("malformed key"))?;
    let sig: [u8; 64] = B64
        .decode(sig)
        .ok()
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error::new("malformed key"))?;
    pk.verify(&payload, &Signature::from_bytes(&sig)).map_err(|_| Error::new("invalid key"))?;
    Ok(serde_json::from_slice(&payload)?)
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parses the date part of an ISO 8601 timestamp ("2026-11-10T08:00:00.000000Z") to unix seconds.
pub fn parse_iso_date(s: &str) -> Option<u64> {
    let date = s.get(..10)?;
    let mut it = date.split('-');
    let (y, m, d) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    let days = days_from_civil(y, m, d);
    (days >= 0).then_some(days as u64 * DAY)
}

pub trait Http: Send + Sync {
    /// POSTs form fields, returns (status, body). Errors mean the request never completed.
    fn post_form(&self, url: &str, fields: &[(&str, &str)]) -> Result<(u16, String)>;
}

pub struct UreqHttp;

impl Http for UreqHttp {
    fn post_form(&self, url: &str, fields: &[(&str, &str)]) -> Result<(u16, String)> {
        let resp = ureq::post(url)
            .set("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(15))
            .send_form(fields);
        match resp {
            Ok(r) => {
                let code = r.status();
                Ok((code, r.into_string()?))
            }
            Err(ureq::Error::Status(code, r)) => Ok((code, r.into_string().unwrap_or_default())),
            Err(e) => Err(Error(e.to_string())),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    key: Option<String>,
    trial_started: u64,
    ls_instance: Option<String>,
    ls_validated_at: Option<u64>,
    ls_expires: Option<u64>,
    ls_name: Option<String>,
}

pub struct Licensing {
    path: PathBuf,
    stored: Stored,
    pk: VerifyingKey,
    /// Only keys from this Lemon Squeezy store are accepted; None = card sales not set up.
    ls_store: Option<u64>,
}

#[derive(Deserialize)]
struct LsKey {
    status: String,
    expires_at: Option<String>,
}

#[derive(Deserialize)]
struct LsMeta {
    store_id: u64,
    customer_name: Option<String>,
}

#[derive(Deserialize)]
struct LsInstance {
    id: String,
}

#[derive(Deserialize)]
struct LsResponse {
    activated: Option<bool>,
    valid: Option<bool>,
    error: Option<String>,
    license_key: Option<LsKey>,
    instance: Option<LsInstance>,
    meta: Option<LsMeta>,
}

impl Licensing {
    pub fn load(path: PathBuf, now: u64, ls_store: Option<u64>) -> Result<Self> {
        Self::load_with_key(path, now, ls_store, public_key()?)
    }

    fn load_with_key(path: PathBuf, now: u64, ls_store: Option<u64>, pk: VerifyingKey) -> Result<Self> {
        let mut stored: Stored =
            std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let fresh = stored.trial_started == 0;
        if fresh {
            stored.trial_started = now;
        }
        let me = Licensing { path, stored, pk, ls_store };
        if fresh {
            me.save()?;
        }
        Ok(me)
    }

    fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.path, serde_json::to_vec_pretty(&self.stored)?)?;
        Ok(())
    }

    pub fn status(&self, now: u64) -> Status {
        let expired = match self.license(now) {
            Some((lic, true)) => return Status::Premium { license: lic },
            Some((lic, false)) => Some(lic),
            None => None,
        };
        let trial_end = self.stored.trial_started + TRIAL_DAYS * DAY;
        if expired.is_none() && now < trial_end {
            return Status::Trial { days_left: (trial_end - now).div_ceil(DAY) };
        }
        Status::Free { expired }
    }

    /// The stored license and whether it is currently valid.
    fn license(&self, now: u64) -> Option<(License, bool)> {
        let key = self.stored.key.as_deref()?;
        if key.starts_with("RFTP-") {
            let c = verify_raftaar(key, &self.pk).ok()?;
            let lic = License {
                name: c.n,
                plan: Some(c.p),
                expires: (c.e != 0).then_some(c.e),
                source: KeySource::Raftaar,
            };
            let ok = c.e == 0 || now < c.e;
            Some((lic, ok))
        } else {
            let lic = License {
                name: self.stored.ls_name.clone().unwrap_or_default(),
                plan: None,
                expires: self.stored.ls_expires,
                source: KeySource::LemonSqueezy,
            };
            let fresh = self.stored.ls_validated_at.is_some_and(|t| now.saturating_sub(t) < OFFLINE_GRACE);
            let ok = fresh && self.stored.ls_expires.is_none_or(|e| now < e);
            Some((lic, ok))
        }
    }

    pub fn activate(&mut self, key: &str, http: &dyn Http, now: u64) -> Result<Status> {
        let key = key.trim();
        if key.starts_with("RFTP-") {
            let c = verify_raftaar(key, &self.pk)?;
            if c.e != 0 && now >= c.e {
                return Err(Error::new("This key has expired."));
            }
            self.stored.key = Some(key.to_string());
            self.stored.ls_instance = None;
        } else {
            let Some(store) = self.ls_store else {
                return Err(Error::new("invalid key"));
            };
            let (code, body) = http.post_form(&format!("{LS_API}/activate"), &[("license_key", key), ("instance_name", "Raftaar")])?;
            let r: LsResponse = serde_json::from_str(&body).map_err(|_| Error(format!("License server error ({code})")))?;
            if r.activated != Some(true) {
                return Err(Error(r.error.unwrap_or_else(|| "invalid key".into())));
            }
            let meta = r.meta.ok_or_else(|| Error::new("invalid key"))?;
            if meta.store_id != store {
                return Err(Error::new("invalid key"));
            }
            self.stored.key = Some(key.to_string());
            self.stored.ls_instance = r.instance.map(|i| i.id);
            self.stored.ls_validated_at = Some(now);
            self.stored.ls_expires = r.license_key.and_then(|k| k.expires_at).and_then(|e| parse_iso_date(&e));
            self.stored.ls_name = meta.customer_name;
        }
        self.save()?;
        Ok(self.status(now))
    }

    /// Re-checks a Lemon Squeezy key. Network failures keep the current state (grace period).
    pub fn revalidate(&mut self, http: &dyn Http, now: u64) -> Result<()> {
        let (Some(key), Some(inst)) = (self.stored.key.clone(), self.stored.ls_instance.clone()) else {
            return Ok(());
        };
        let Ok((_, body)) = http.post_form(&format!("{LS_API}/validate"), &[("license_key", &key), ("instance_id", &inst)])
        else {
            return Ok(());
        };
        let Ok(r) = serde_json::from_str::<LsResponse>(&body) else { return Ok(()) };
        let active = r.valid == Some(true)
            && r.license_key.as_ref().is_some_and(|k| k.status == "active")
            && r.meta.as_ref().is_some_and(|m| Some(m.store_id) == self.ls_store);
        if active {
            self.stored.ls_validated_at = Some(now);
            self.stored.ls_expires = r.license_key.and_then(|k| k.expires_at).and_then(|e| parse_iso_date(&e));
        } else {
            self.stored.ls_validated_at = None;
        }
        self.save()
    }

    pub fn remove(&mut self, http: &dyn Http) -> Result<()> {
        if let (Some(key), Some(inst)) = (&self.stored.key, &self.stored.ls_instance) {
            // Frees the activation slot so the key can move to another PC. Best effort.
            let _ = http.post_form(&format!("{LS_API}/deactivate"), &[("license_key", key), ("instance_id", inst)]);
        }
        self.stored.key = None;
        self.stored.ls_instance = None;
        self.stored.ls_validated_at = None;
        self.stored.ls_expires = None;
        self.stored.ls_name = None;
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const NOW: u64 = 1_800_000_000; // 2027-01-15

    fn keys() -> (String, VerifyingKey) {
        let (sk, pk) = generate_keypair();
        (sk, VerifyingKey::from_bytes(&hex32(&pk).unwrap()).unwrap())
    }

    fn licensing(pk: VerifyingKey, store: Option<u64>) -> (tempfile::TempDir, Licensing) {
        let d = tempfile::tempdir().unwrap();
        let l = Licensing::load_with_key(d.path().join("license.json"), NOW, store, pk).unwrap();
        (d, l)
    }

    struct FakeHttp {
        reply: Mutex<Result<(u16, String)>>,
        calls: Mutex<Vec<String>>,
    }

    impl FakeHttp {
        fn new(reply: Result<(u16, String)>) -> Self {
            FakeHttp { reply: Mutex::new(reply), calls: Mutex::new(vec![]) }
        }
    }

    impl Http for FakeHttp {
        fn post_form(&self, url: &str, _: &[(&str, &str)]) -> Result<(u16, String)> {
            self.calls.lock().unwrap().push(url.to_string());
            self.reply.lock().unwrap().clone()
        }
    }

    fn no_http() -> FakeHttp {
        FakeHttp::new(Err(Error::new("offline")))
    }

    #[test]
    fn embedded_public_key_is_valid() {
        public_key().unwrap();
    }

    #[test]
    fn issued_key_verifies_and_tampering_fails() {
        let (sk, pk) = keys();
        let c = Claims { n: "Ali 0300".into(), p: Plan::Monthly, e: NOW + 30 * DAY, t: NOW };
        let key = issue(&sk, &c).unwrap();
        assert_eq!(verify_raftaar(&key, &pk).unwrap(), c);
        let mut bad = key.clone();
        bad.insert(10, 'A');
        assert!(verify_raftaar(&bad, &pk).is_err());
        let (_, other_pk) = keys();
        assert!(verify_raftaar(&key, &other_pk).is_err(), "a key from another signer must fail");
    }

    #[test]
    fn new_install_gets_a_seven_day_trial_then_free() {
        let (_, pk) = keys();
        let (_d, l) = licensing(pk, None);
        assert_eq!(l.status(NOW), Status::Trial { days_left: 7 });
        assert_eq!(l.status(NOW + 6 * DAY + 1), Status::Trial { days_left: 1 });
        assert_eq!(l.status(NOW + 7 * DAY), Status::Free { expired: None });
        assert!(!l.status(NOW + 8 * DAY).is_premium());
    }

    #[test]
    fn trial_start_survives_restart() {
        let (_, pk) = keys();
        let d = tempfile::tempdir().unwrap();
        Licensing::load_with_key(d.path().join("l.json"), NOW, None, pk).unwrap();
        let later = Licensing::load_with_key(d.path().join("l.json"), NOW + 10 * DAY, None, pk).unwrap();
        assert!(!later.status(NOW + 10 * DAY).is_premium());
    }

    #[test]
    fn raftaar_key_unlocks_until_expiry() {
        let (sk, pk) = keys();
        let (_d, mut l) = licensing(pk, None);
        let key = issue(&sk, &Claims { n: "Sara".into(), p: Plan::Monthly, e: NOW + 30 * DAY, t: NOW }).unwrap();
        assert!(l.activate(&key, &no_http(), NOW).unwrap().is_premium());
        assert!(l.status(NOW + 29 * DAY).is_premium());
        match l.status(NOW + 31 * DAY) {
            Status::Free { expired: Some(lic) } => assert_eq!(lic.name, "Sara"),
            s => panic!("{s:?}"),
        }
    }

    #[test]
    fn lifetime_key_never_expires_and_expired_key_is_refused() {
        let (sk, pk) = keys();
        let (_d, mut l) = licensing(pk, None);
        let old = issue(&sk, &Claims { n: "x".into(), p: Plan::Monthly, e: NOW - 1, t: NOW - 40 * DAY }).unwrap();
        assert!(l.activate(&old, &no_http(), NOW).is_err());
        let life = issue(&sk, &Claims { n: "x".into(), p: Plan::Lifetime, e: 0, t: NOW }).unwrap();
        l.activate(&life, &no_http(), NOW).unwrap();
        assert!(l.status(NOW + 5000 * DAY).is_premium());
    }

    const LS_OK: &str = r#"{"activated":true,"error":null,"license_key":{"status":"active","expires_at":"2028-01-15T10:00:00.000000Z"},"instance":{"id":"inst-1"},"meta":{"store_id":42,"customer_name":"Jane"}}"#;

    #[test]
    fn lemon_squeezy_key_activates_only_for_our_store() {
        let (_, pk) = keys();
        let (_d, mut l) = licensing(pk, Some(42));
        let s = l.activate("ABCD-1234", &FakeHttp::new(Ok((200, LS_OK.into()))), NOW).unwrap();
        match s {
            Status::Premium { license } => {
                assert_eq!(license.name, "Jane");
                assert_eq!(license.expires, parse_iso_date("2028-01-15"));
            }
            s => panic!("{s:?}"),
        }
        let (_, pk) = keys();
        let (_d2, mut other) = licensing(pk, Some(7));
        assert!(other.activate("ABCD-1234", &FakeHttp::new(Ok((200, LS_OK.into()))), NOW).is_err());
    }

    #[test]
    fn lemon_squeezy_disabled_when_no_store_configured() {
        let (_, pk) = keys();
        let (_d, mut l) = licensing(pk, None);
        let http = FakeHttp::new(Ok((200, LS_OK.into())));
        assert!(l.activate("ABCD-1234", &http, NOW).is_err());
        assert!(http.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn lemon_squeezy_error_message_is_passed_on() {
        let (_, pk) = keys();
        let (_d, mut l) = licensing(pk, Some(42));
        let body = r#"{"activated":false,"error":"This license key has reached the activation limit."}"#;
        let err = l.activate("K", &FakeHttp::new(Ok((400, body.into()))), NOW).unwrap_err();
        assert!(err.0.contains("activation limit"));
    }

    #[test]
    fn offline_grace_then_revalidation() {
        let (_, pk) = keys();
        let (_d, mut l) = licensing(pk, Some(42));
        l.activate("K", &FakeHttp::new(Ok((200, LS_OK.into()))), NOW).unwrap();
        // Offline for 10 days: still premium, and a failed revalidation keeps it.
        l.revalidate(&no_http(), NOW + 10 * DAY).unwrap();
        assert!(l.status(NOW + 10 * DAY).is_premium());
        // Offline for 15 days: grace is over.
        assert!(!l.status(NOW + 15 * DAY).is_premium());
        // Back online and still valid: premium again.
        let valid = LS_OK.replace(r#""activated":true"#, r#""valid":true"#);
        l.revalidate(&FakeHttp::new(Ok((200, valid))), NOW + 15 * DAY).unwrap();
        assert!(l.status(NOW + 15 * DAY).is_premium());
        // Refunded: the server says disabled.
        let disabled = LS_OK.replace(r#""activated":true"#, r#""valid":false"#).replace("active", "disabled");
        l.revalidate(&FakeHttp::new(Ok((200, disabled))), NOW + 16 * DAY).unwrap();
        assert!(!l.status(NOW + 16 * DAY).is_premium());
    }

    #[test]
    fn remove_deactivates_and_returns_to_free() {
        let (_, pk) = keys();
        let (_d, mut l) = licensing(pk, Some(42));
        l.activate("K", &FakeHttp::new(Ok((200, LS_OK.into()))), NOW).unwrap();
        let http = FakeHttp::new(Ok((200, "{}".into())));
        l.remove(&http).unwrap();
        assert!(http.calls.lock().unwrap()[0].ends_with("/deactivate"));
        assert_eq!(l.status(NOW + 30 * DAY), Status::Free { expired: None });
    }

    #[test]
    fn iso_dates_parse() {
        assert_eq!(parse_iso_date("1970-01-02T00:00:00Z"), Some(DAY));
        assert_eq!(parse_iso_date("2000-03-01"), Some(951_868_800));
        assert_eq!(parse_iso_date("garbage"), None);
    }
}
