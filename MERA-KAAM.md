# Raftaar: mera kaam (owner checklist)

Code ka kaam mukammal hai. Ye woh kaam hain jo sirf aap kar sakte hain.
Har kaam ke saath likha hai ke kyun zaroori hai aur kaise karna hai.

---

## 1. Keys ka backup (sab se pehle, aaj hi)

**Kyun:** ye do files kho gayi to (a) purane users ko kabhi update nahi bhej sakenge,
(b) bechi hui Premium keys ke saath nayi keys nahi bana sakenge.

- [ ] `C:\Users\DCS\.tauri\raftaar-updater.key` (updates sign karne ki key)
- [ ] `C:\Users\DCS\.tauri\raftaar-license.key` (Premium keys banane ki key)

**Kaise:** dono files USB, Google Drive (private folder) ya password manager mein copy karein.
Kabhi kisi ko na bhejein, kabhi GitHub pe na daalein.

---

## 2. Pakistan payments (JazzCash / Easypaisa)

**Kyun:** jab tak ye khaali hai, app mein "Buying opens soon" dikhta hai aur koi khareed nahi sakta.

- [ ] File `D:\game_booster\src\config.ts` kholein aur bharein:
  - `jazzcash: '0300 1234567 · Aap ka naam'`
  - `easypaisa: '0345 1234567 · Aap ka naam'`
  - `whatsapp: '923001234567'` (sirf digits, 92 se shuru)
- [ ] Qeemat check karein (abhi Rs 399 mahina, Rs 2,999 saal). Badalni ho to `pricePkr` mein badlein.
- [ ] Naya version release karein (neeche "Naya version release karna" dekhein).

Ya ye details mujhe bata dein, main bhar ke release kar dunga.

---

## 3. Card payments, duniya bhar ke liye (Lemon Squeezy)

**Kyun:** Pakistan se bahar ke log JazzCash use nahi karte. Lemon Squeezy card se paise leta hai,
tax sambhalta hai, aur customer ko key khud email karta hai.

- [ ] https://lemonsqueezy.com pe account banayein aur store banayein.
- [ ] Payout setup karein (Pakistan ke liye PayPal ya bank; unki payout list check karein).
- [ ] Product "Raftaar Premium" banayein: monthly $2.99 aur yearly $19.99 (ya jo aap chahein),
      **License keys ON** karein, activation limit 1 ya 2 rakhein.
- [ ] Ye do cheezein mujhe dein ya khud likhein:
  - Store ID (Settings → Stores) → `src-tauri\src\lib.rs` mein `LEMON_SQUEEZY_STORE_ID: Option<u64> = Some(12345);`
  - Checkout link → `src\config.ts` mein `checkoutUrl`
- [ ] Naya version release karein.

---

## 4. Code signing (SmartScreen ka laal warning hatana)

**Kyun:** abhi installer chalane pe Windows "Windows protected your PC" dikhata hai. Bohat log
yahin ruk jaate hain aur install nahi karte.

- [ ] **Free option:** https://signpath.org pe open-source project ke liye apply karein
      (repo public hai, is liye eligible hai). Approval mein kuch hafte lag sakte hain.
- [ ] **Paid option:** kisi CA se code signing certificate (~$200-400 saal).
- [ ] Certificate milne ke baad mujhe batayein, main build mein signing jod dunga.

---

## 5. Asli testing (release ko bharosemand banana)

**Kyun:** ye cheezein is PC pe test nahi ho sakti thin (admin, game ya Sandbox chahiye).

- [ ] **FPS Test:** kisi gaming PC pe Raftaar admin ke taur pe chalayein, game shuru karein,
      FPS Test chalayein. Average FPS aur 1% low aana chahiye.
- [ ] **Game Boost:** Boost on karein, game kholein, Discord jaisi app band honi chahiye,
      game band karne pe wapas khulni chahiye.
- [ ] **Game detection:** Steam/Epic wale PC pe "Games on this PC" mein games dikhne chahiye.
- [ ] **Sandbox test (sab se ahem):**
  1. Admin PowerShell: `Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM -All`, phir restart.
  2. `D:\game_booster\scripts\sandbox.wsb` kholein.
  3. Sandbox mein: `state-diff.ps1 -Save before.json` → Raftaar install → Optimize (Max) → Revert All →
     `state-diff.ps1 -Compare before.json`. Jawab **"0 differences"** hona chahiye.

Koi bhi masla aaye to screenshot ya error mujhe bhej dein.

---

## 6. Customer ko Premium key dena (jab payment aaye)

1. JazzCash/Easypaisa pe payment confirm karein.
2. `D:\game_booster` mein ye command chalayein:
   ```
   cargo run -p engine --bin raftaar-cli -- license-new "Customer ka naam ya number" 30
   ```
   - `30` = ek mahina, `365` = ek saal, `lifetime` = hamesha ke liye
3. Jo `RFTP-...` key aaye woh customer ko WhatsApp kar dein.
4. Customer: Raftaar → Premium → key paste → Activate.

Key check karni ho: `cargo run -p engine --bin raftaar-cli -- license-check RFTP-...`

---

## 7. Naya version release karna (jab bhi kuch badlein)

1. `package.json` aur `src-tauri\tauri.conf.json` mein `version` barhayein (jaise 0.4.0 → 0.4.1).
2. `D:\game_booster` mein: `.\scripts\release.ps1 -Notes "Kya badla"`
3. https://github.com/hariskhan2010/raftaar/releases/new pe naya release `v0.4.1` banayein aur
   script ki batayi hui **teeno** files upload karein (`Raftaar_..._x64-setup.exe`, `Raftaar-Setup.exe` aur `latest.json`). `Raftaar-Setup.exe` zaroori hai: website ka Download button isi ko seedha download karta hai.
4. Website ka download naya karein: `cd docs` phir `vercel deploy --prod` (script installer ko `docs` mein copy kar deta hai).
5. Sab users ko agli baar app kholne pe "Update now" dikhega.

---

## 8. Logon tak pohanchna (marketing)

- [ ] Website share karein: https://hariskhan2010.github.io/raftaar/
- [ ] Gaming groups (Facebook, Discord, Reddit r/pcgaming, r/lowendgaming) mein post karein.
- [ ] YouTube pe ek video: pehle FPS Test → Optimize → baad ka FPS Test (asli farq dikhayein).
- [ ] Pakistan ke gaming cafes se baat karein (cafe mode agla phase hai).
- [ ] Apna domain lena ho (jaise raftaar.app) to batayein, website us pe laga dunga.

---

## Links

| Kya | Kahan |
|---|---|
| Website | https://raftaar-eight.vercel.app (Vercel) · https://hariskhan2010.github.io/raftaar/ (GitHub Pages) |
| Download | https://github.com/hariskhan2010/raftaar/releases/latest |
| Code | https://github.com/hariskhan2010/raftaar |
| Project folder | `D:\game_booster` |
| Keys | `C:\Users\DCS\.tauri\` (backup!) |
