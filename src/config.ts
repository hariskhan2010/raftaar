// How customers buy Premium. Empty values hide that option in the app.
// After changing anything here, build and publish a new release (scripts/release.ps1).
export const SALES = {
  // Pakistan: the customer sends money, then messages you on WhatsApp with their
  // name or phone; you reply with a key from `raftaar-cli license-new "<name>" 30`.
  jazzcash: '', // e.g. '0300 1234567 · Haris Khan'
  easypaisa: '', // e.g. '0345 1234567 · Haris Khan'
  whatsapp: '', // international format, digits only, e.g. '923001234567'

  // Everywhere else: the Lemon Squeezy checkout link of the Premium product. Lemon
  // Squeezy emails the license key itself. Also set LEMON_SQUEEZY_STORE_ID in
  // src-tauri/src/lib.rs, or those keys will be refused.
  checkoutUrl: '', // e.g. 'https://raftaar.lemonsqueezy.com/buy/...'

  pricePkr: { month: 399, year: 2999 },
  priceUsd: { month: 2.99, year: 19.99 },
}

export const SITE_URL = 'https://hariskhan2010.github.io/raftaar/'
