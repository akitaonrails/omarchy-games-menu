import QtQuick

// Shared semantic appearance. One instance lives in shell.qml and is passed
// down. Omarchy colors, text size and reduced motion arrive through `ogm
// appearance`; these defaults keep the overlay usable elsewhere.
QtObject {
  property real coverScale: 1.0
  property real uiScale: 1.0
  property var appearance: ({})

  // Scale a fixed UI dimension by the output scale.
  function u(v) { return Math.round(v * uiScale) }

  readonly property color overlayBase: appearance.overlay_background || "#0d1117"
  readonly property real overlayAlpha: typeof appearance.overlay_alpha === "number"
    ? Math.min(1, Math.max(0, appearance.overlay_alpha)) : 0.90
  readonly property color overlayBg: Qt.rgba(
    overlayBase.r, overlayBase.g, overlayBase.b, overlayAlpha)
  readonly property color bg: appearance.background || "#1b2838"
  readonly property color card: appearance.card || "#2a475e"
  readonly property color cardEdge: appearance.card_edge || "#3a5f7e"
  readonly property color accent: appearance.accent || "#66c0f4"
  readonly property color text: appearance.text || "#c7d5e0"
  readonly property color muted: appearance.muted || "#8f98a0"
  readonly property color update: appearance.update || "#f0a232"
  readonly property color ok: appearance.ok || "#5c7e10"
  readonly property bool reducedMotion: appearance.reduced_motion === true

  // ODK-aligned foundation tokens: a 4px grid, subtle rectangular corners,
  // stable borders/focus, and shared motion durations.
  readonly property int space1: u(4)
  readonly property int space2: u(8)
  readonly property int space3: u(12)
  readonly property int space4: u(16)
  readonly property int radius: u(2)
  readonly property int borderWidth: Math.max(1, u(1))
  readonly property int focusWidth: Math.max(2, u(2))
  readonly property int controlHeight: u(34 * textScale)
  readonly property int motionFast: reducedMotion ? 0 : 140
  readonly property int motionNormal: reducedMotion ? 0 : 180
  readonly property int motionEnter: reducedMotion ? 0 : 240
  readonly property int motionExit: reducedMotion ? 0 : 160
  readonly property int motionCycle: 1000

  // Text size is independent of artwork zoom and follows Omarchy's 9–20px
  // base-size range. Display scaling remains a separate physical-size concern.
  readonly property real fontBase: typeof appearance.font_base === "number"
    ? Math.min(20, Math.max(9, appearance.font_base)) : 12
  readonly property real textScale: fontBase / 12
  readonly property real _fs: textScale * uiScale
  readonly property int fontS: Math.round(11 * _fs)
  readonly property int fontM: Math.round(13 * _fs)
  readonly property int fontL: Math.round(16 * _fs)
  readonly property int fontHero: Math.round(72 * Math.min(2.0, coverScale) * uiScale)
}
