import { useState } from 'react'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Check, Crown, CreditCard, KeyRound, MessageCircle, Smartphone } from 'lucide-react'
import { api, type LicenseView } from '../api'
import { SALES } from '../config'
import { errText, formatDate, useT } from '../i18n'
import { useApp } from '../store'
import { Banner, PageHead } from '../components/ui'

export function Premium() {
  const { t, lang } = useT()
  const { license, setLicense, toast } = useApp()
  const [key, setKey] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const activate = async () => {
    setBusy(true)
    setError(null)
    try {
      const v: LicenseView = await api.licenseActivate(key)
      setLicense(v)
      setKey('')
      toast(t.activated)
    } catch (e) {
      setError(errText(e))
    } finally {
      setBusy(false)
    }
  }

  const remove = async () => {
    try {
      setLicense(await api.licenseRemove())
    } catch (e) {
      toast(errText(e))
    }
  }

  const open = (url: string) => openUrl(url).catch((e) => toast(errText(e)))
  const s = license?.status
  const hasPk = SALES.jazzcash || SALES.easypaisa
  const pkr = (n: number) => `Rs ${n.toLocaleString()}`
  const usd = (n: number) => `$${n.toFixed(2)}`

  return (
    <>
      <PageHead title={t.premiumTitle} sub={t.premiumSub} />

      {s && (
        <section className="card row between wrap" style={{ borderColor: s.kind === 'Free' ? undefined : 'rgba(255,106,26,.45)' }}>
          <div className="row" style={{ gap: 14 }}>
            <div className="logo-mark" style={{ width: 44, height: 44 }}>
              <Crown size={22} />
            </div>
            <div>
              <strong style={{ fontSize: 16 }}>
                {s.kind === 'Trial'
                  ? t.statusTrial(s.days_left)
                  : s.kind === 'Premium'
                    ? t.statusPremium
                    : s.expired
                      ? t.statusExpired
                      : t.statusFree}
              </strong>
              {s.kind === 'Premium' && (
                <p className="muted small">
                  {s.license.name && `${t.licensedTo(s.license.name)} · `}
                  {s.license.expires ? t.validUntil(formatDate(s.license.expires, lang)) : t.lifetime}
                </p>
              )}
            </div>
          </div>
          {s.kind === 'Premium' && (
            <button className="btn ghost sm" onClick={remove}>
              {t.removeKey}
            </button>
          )}
        </section>
      )}

      <div className="grid cols-2">
        <section className="card stack">
          <h2 style={{ fontSize: 15 }}>{t.freeFeatures}</h2>
          <ul className="stack" style={{ gap: 8, margin: 0, padding: 0, listStyle: 'none' }}>
            {t.freeList.map((f) => (
              <li key={f} className="row muted" style={{ alignItems: 'flex-start' }}>
                <Check size={16} color="var(--ok)" style={{ flexShrink: 0, marginTop: 3 }} />
                {f}
              </li>
            ))}
          </ul>
        </section>
        <section className="card stack" style={{ borderColor: 'rgba(255,106,26,.45)' }}>
          <div className="row between">
            <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
              <Crown size={16} color="var(--accent)" />
              {t.premiumFeatures}
            </h2>
            <span className="ltr" style={{ fontWeight: 700 }}>
              {pkr(SALES.pricePkr.month)} <span className="muted small">{t.perMonth}</span>
            </span>
          </div>
          <ul className="stack" style={{ gap: 8, margin: 0, padding: 0, listStyle: 'none' }}>
            {t.premiumList.map((f) => (
              <li key={f} className="row" style={{ alignItems: 'flex-start' }}>
                <Check size={16} color="var(--accent)" style={{ flexShrink: 0, marginTop: 3 }} />
                {f}
              </li>
            ))}
          </ul>
        </section>
      </div>

      {s?.kind !== 'Premium' && (
        <div className="grid cols-2">
          <section className="card stack">
            <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
              <Smartphone size={16} color="var(--accent)" />
              {t.buyPakistan}
            </h2>
            <p className="ltr" style={{ fontWeight: 650 }}>
              {pkr(SALES.pricePkr.month)} {t.perMonth} · {pkr(SALES.pricePkr.year)} {t.perYear}
            </p>
            {hasPk ? (
              <>
                {SALES.jazzcash && (
                  <p>
                    <strong>JazzCash:</strong> <span className="ltr selectable">{SALES.jazzcash}</span>
                  </p>
                )}
                {SALES.easypaisa && (
                  <p>
                    <strong>Easypaisa:</strong> <span className="ltr selectable">{SALES.easypaisa}</span>
                  </p>
                )}
                <p className="muted small">{t.payInstructions}</p>
                {SALES.whatsapp && (
                  <button className="btn" onClick={() => open(`https://wa.me/${SALES.whatsapp}`)}>
                    <MessageCircle size={15} />
                    {t.openWhatsApp}
                  </button>
                )}
              </>
            ) : (
              <p className="muted">{t.buySoon}</p>
            )}
          </section>
          <section className="card stack">
            <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
              <CreditCard size={16} color="var(--accent)" />
              {t.buyCard}
            </h2>
            <p className="ltr" style={{ fontWeight: 650 }}>
              {usd(SALES.priceUsd.month)} {t.perMonth} · {usd(SALES.priceUsd.year)} {t.perYear}
            </p>
            {SALES.checkoutUrl && license?.card_keys ? (
              <button className="btn primary" onClick={() => open(SALES.checkoutUrl)}>
                <CreditCard size={15} />
                {t.buyCardBtn}
              </button>
            ) : (
              <p className="muted">{t.buySoon}</p>
            )}
          </section>
        </div>
      )}

      <section className="card stack">
        <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
          <KeyRound size={16} color="var(--accent)" />
          {t.haveKey}
        </h2>
        {error && <Banner kind="error">{error}</Banner>}
        <form
          className="row"
          onSubmit={(e) => {
            e.preventDefault()
            activate()
          }}
        >
          <input
            className="input ltr mono"
            style={{ flex: 1 }}
            placeholder={t.keyPlaceholder}
            value={key}
            onChange={(e) => setKey(e.target.value)}
            aria-label={t.keyPlaceholder}
          />
          <button className="btn primary" type="submit" disabled={!key.trim() || busy}>
            {busy ? t.activating : t.activate}
          </button>
        </form>
      </section>
    </>
  )
}
