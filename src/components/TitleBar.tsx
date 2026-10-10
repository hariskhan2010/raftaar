import { getCurrentWindow } from '@tauri-apps/api/window'
import { Minus, Square, X, Zap } from 'lucide-react'
import { useT } from '../i18n'

/** Custom title bar (the window has no native decorations). */
export function TitleBar() {
  const { t } = useT()
  const win = () => getCurrentWindow()
  return (
    <div className="titlebar" data-tauri-drag-region onDoubleClick={() => win().toggleMaximize()}>
      <div className="titlebar-brand">
        <Zap size={13} color="var(--accent)" />
        <span>Raftaar</span>
      </div>
      <div className="titlebar-controls" onDoubleClick={(e) => e.stopPropagation()}>
        <button aria-label={t.minimize} onClick={() => win().minimize()}>
          <Minus size={15} />
        </button>
        <button aria-label={t.maximize} onClick={() => win().toggleMaximize()}>
          <Square size={12} />
        </button>
        <button className="close" aria-label={t.close} onClick={() => win().close()}>
          <X size={16} />
        </button>
      </div>
    </div>
  )
}
