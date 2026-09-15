import QtQuick

// Shared Steam-like dark palette + font scale. Stateless; one instance lives
// in shell.qml and is passed down. coverScale follows prefs.cover_scale;
// uiScale follows the Hyprland output scale (detected in shell.qml) so fixed
// pixel sizes track the system's fractional scaling.
QtObject {
  property real coverScale: 1.0
  property real uiScale: 1.0

  // Scale a fixed UI dimension by the output scale.
  function u(v) { return Math.round(v * uiScale) }

  readonly property color overlayBg: "#E60d1117"
  readonly property color bg: "#1b2838"
  readonly property color card: "#2a475e"
  readonly property color cardEdge: "#3a5f7e"
  readonly property color accent: "#66c0f4"
  readonly property color text: "#c7d5e0"
  readonly property color muted: "#8f98a0"
  readonly property color update: "#f0a232"
  readonly property color ok: "#5c7e10"

  // Fonts track cover_scale (damped so extreme zooms stay legible) and the
  // output scale (so they are physical-size correct on scaled monitors).
  readonly property real _fs: Math.min(1.6, Math.max(0.6, coverScale)) * uiScale
  readonly property int fontS: Math.round(11 * _fs)
  readonly property int fontM: Math.round(13 * _fs)
  readonly property int fontL: Math.round(16 * _fs)
  readonly property int fontHero: Math.round(72 * Math.min(2.0, coverScale) * uiScale)
}
