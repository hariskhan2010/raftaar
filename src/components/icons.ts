import { BatteryCharging, Crosshair, EyeOff, HardDrive, Layers, Monitor, Wifi } from 'lucide-react'
import type { Category } from '../api'

export const CATEGORY_ICON: Record<Category, typeof Crosshair> = {
  Fps: Crosshair,
  Power: BatteryCharging,
  Background: Layers,
  Privacy: EyeOff,
  Visuals: Monitor,
  Network: Wifi,
  Storage: HardDrive,
}
