import type { RestoreResult } from './api'
import { useT } from './i18n'

export type RestoreChoice = 'enable' | 'continue' | 'cancel'

export function RestoreModal({ result, onChoose }: { result: RestoreResult; onChoose: (c: RestoreChoice) => void }) {
  const { t } = useT()
  return (
    <div className="modal-backdrop">
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby="restore-title">
        <h2 id="restore-title">{t.restoreTitle}</h2>
        {result.status === 'Failed' ? (
          <>
            <p>{t.restoreFailed}</p>
            <pre className="detail">{result.detail}</pre>
          </>
        ) : (
          <p>{t.restoreOff}</p>
        )}
        <p className="muted">{t.restoreExplain}</p>
        <div className="modal-actions">
          {result.status === 'ProtectionOff' && (
            <button className="btn primary" onClick={() => onChoose('enable')}>
              {t.enableProtection}
            </button>
          )}
          <button className="btn" onClick={() => onChoose('continue')}>
            {t.continueWithout}
          </button>
          <button className="btn ghost" onClick={() => onChoose('cancel')}>
            {t.cancel}
          </button>
        </div>
      </div>
    </div>
  )
}
