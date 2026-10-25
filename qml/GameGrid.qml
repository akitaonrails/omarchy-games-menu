pragma ComponentBehavior: Bound

import QtQuick

// Sectioned grid of game cards: one labeled section per group (Games,
// Emulators, Tools) so they never mix. Cell size derives from
// prefs.cover_scale (via theme). Plain wheel scrolls (Flickable default);
// Ctrl+wheel zooms via signal so shell.qml can clamp + persist cover_scale.
Item {
  id: root

  required property Theme theme
  property var sections: []
  property int visibleCount: 0
  property int totalGames: 0

  signal zoomRequested(real delta)
  signal playRequested(var game)
  signal removeRequested(var game)
  signal copyExecRequested(string exec)

  // Keyboard navigation for the outer Flickable (the section GridViews are
  // non-interactive; this is the only scroller).
  function scrollTo(y) {
    var maxY = Math.max(0, flick.contentHeight - flick.height)
    flick.contentY = Math.max(0, Math.min(maxY, y))
  }
  function pageUp() { scrollTo(flick.contentY - flick.height) }
  function pageDown() { scrollTo(flick.contentY + flick.height) }
  function scrollHome() { scrollTo(0) }
  function scrollEnd() { scrollTo(flick.contentHeight) }

  // ---- selection cursor (arrow keys / controller) ----
  // Addressed as (section, index) rather than one flat index because the
  // sections have independent lengths and each wraps at the same column
  // count. -1 means nothing is selected yet.
  property int selSection: -1
  property int selIndex: -1

  // Derived from the *minimum* cell width, never from cellW — cellW is
  // computed from this and the binding would be circular.
  readonly property int columns: Math.max(1, Math.floor(width / minCellW))

  function sectionLen(s) {
    return (s >= 0 && s < sections.length && sections[s].games) ? sections[s].games.length : 0
  }
  function selectedGame() {
    if (selSection < 0 || selSection >= sections.length) return null
    var g = sections[selSection].games
    return (g && selIndex >= 0 && selIndex < g.length) ? g[selIndex] : null
  }
  function nextNonEmpty(from, step) {
    for (var s = from + step; s >= 0 && s < sections.length; s += step)
      if (sectionLen(s) > 0) return s
    return -1
  }

  // Filtering can empty or shorten the section the cursor sits in.
  function clampSelection() {
    if (selSection < 0) return
    if (selSection >= sections.length || sectionLen(selSection) === 0) {
      var f = nextNonEmpty(-1, 1)
      selSection = f
      selIndex = f >= 0 ? 0 : -1
      return
    }
    selIndex = Math.max(0, Math.min(selIndex, sectionLen(selSection) - 1))
  }
  onSectionsChanged: clampSelection()

  // Returns false when there is nothing to select at all.
  function ensureSelection() {
    if (selectedGame()) return true
    var f = nextNonEmpty(-1, 1)
    if (f < 0) return false
    selSection = f
    selIndex = 0
    return true
  }

  function moveSelection(dir) {
    // The first press only places the cursor, it does not also move it.
    if (!selectedGame()) { ensureSelection(); return }

    var s = selSection
    var i = selIndex
    var cols = columns
    var col = i % cols

    if (dir === "right") {
      if (i + 1 < sectionLen(s)) i += 1
      else { var ns = nextNonEmpty(s, 1); if (ns >= 0) { s = ns; i = 0 } }
    } else if (dir === "left") {
      if (i > 0) i -= 1
      else { var ps = nextNonEmpty(s, -1); if (ps >= 0) { s = ps; i = sectionLen(ps) - 1 } }
    } else if (dir === "down") {
      if (i + cols < sectionLen(s)) {
        i += cols
      } else if (i < sectionLen(s) - 1) {
        // Partial last row: fall to the final card before leaving.
        i = sectionLen(s) - 1
      } else {
        var nd = nextNonEmpty(s, 1)
        if (nd >= 0) { s = nd; i = Math.min(col, sectionLen(nd) - 1) }
      }
    } else if (dir === "up") {
      if (i - cols >= 0) {
        i -= cols
      } else if (i > 0) {
        i = 0
      } else {
        var pu = nextNonEmpty(s, -1)
        if (pu >= 0) {
          var lastRow = Math.floor((sectionLen(pu) - 1) / cols) * cols
          s = pu
          i = Math.min(lastRow + col, sectionLen(pu) - 1)
        }
      }
    }

    selSection = s
    selIndex = i
  }

  // Scroll just enough to bring the cursor fully into the viewport. Driven
  // by the card itself (GameCard.onSelectedChanged) so the real laid-out
  // geometry is used rather than a recomputed row position.
  function scrollCardIntoView(card) {
    if (!card) return
    var pad = theme.u(12)
    var top = card.mapToItem(column, 0, 0).y
    var bottom = top + card.height
    if (top - pad < flick.contentY) scrollTo(top - pad)
    else if (bottom + pad > flick.contentY + flick.height)
      scrollTo(bottom + pad - flick.height)
  }

  // The one card whose context menu is open, across all sections; null = none.
  property Item menuCard: null

  // Base card 200x300 cover + label block. Scaled by both the user zoom
  // (coverScale) and the output scale (uiScale).
  readonly property real s: theme.coverScale * theme.uiScale
  readonly property int cardW: Math.max(78, Math.round(200 * s))
  readonly property int cardH: Math.max(158, Math.round(384 * s))
  // Cards keep their chosen size; the cell stretches so `columns` of them
  // tile the full width exactly. The slack lands in the gutters instead of
  // as dead space on the right edge, which is what a fixed cell leaves over.
  readonly property int minCellW: cardW + 12
  readonly property int cellW: Math.max(minCellW, Math.floor(width / columns))
  readonly property int cellH: cardH + 12

  Flickable {
    id: flick
    anchors.fill: parent
    clip: true
    contentWidth: width
    contentHeight: column.height
    boundsBehavior: Flickable.StopAtBounds

    WheelHandler {
      // Manual modifier check instead of acceptedModifiers: that property
      // requires an exact modifier match and fails when e.g. a lock key is
      // active, silently degrading Ctrl+wheel to plain scrolling.
      onWheel: function(event) {
        if ((event.modifiers & Qt.ControlModifier) !== 0) {
          root.zoomRequested(event.angleDelta.y > 0 ? 0.1 : -0.1)
          event.accepted = true
        }
      }
    }

    Column {
      id: column
      width: flick.width
      spacing: root.theme.u(28)

      Repeater {
        model: root.sections

        delegate: Column {
          id: section
          required property var modelData
          required property int index
          width: column.width
          spacing: root.theme.u(12)

          Row {
            spacing: 10
            Text {
              id: sectionTitle
              text: section.modelData.title
              color: root.theme.text
              font.pixelSize: root.theme.fontL
              font.bold: true
            }
            Text {
              anchors.baseline: sectionTitle.baseline
              text: section.modelData.games.length
              color: root.theme.muted
              font.pixelSize: root.theme.fontS
            }
          }

          GridView {
            id: grid
            width: parent.width
            height: contentHeight
            interactive: false
            model: section.modelData.games

            cellWidth: root.cellW
            cellHeight: root.cellH
            Behavior on cellWidth { NumberAnimation { duration: root.theme.motionFast; easing.type: Easing.OutCubic } }
            Behavior on cellHeight { NumberAnimation { duration: root.theme.motionFast; easing.type: Easing.OutCubic } }

            // The cell is as wide as the width allows; the card keeps its
            // own size and sits centered, so the slack shows up as even
            // gutters on both sides of every column.
            delegate: Item {
              id: cell
              required property var modelData
              required property int index
              width: grid.cellWidth
              height: grid.cellHeight

              GameCard {
                id: card
                anchors.centerIn: parent
                width: root.cardW
                height: root.cardH
                theme: root.theme
                game: cell.modelData
                selected: root.selSection === section.index && root.selIndex === cell.index
                onSelectedChanged: if (card.selected) root.scrollCardIntoView(card)
                menuOpen: root.menuCard === card
                onToggleMenu: root.menuCard = (root.menuCard === card) ? null : card
                onCloseMenu: if (root.menuCard === card) root.menuCard = null
                onPlay: { root.menuCard = null; root.playRequested(card.game) }
                onRemoveGame: { root.menuCard = null; root.removeRequested(card.game) }
                onCopyExec: { root.menuCard = null; root.copyExecRequested(String(card.game.exec || "")) }
                Behavior on width { NumberAnimation { duration: root.theme.motionFast; easing.type: Easing.OutCubic } }
                Behavior on height { NumberAnimation { duration: root.theme.motionFast; easing.type: Easing.OutCubic } }
              }
            }
          }
        }
      }
    }
  }

  Text {
    anchors.centerIn: parent
    visible: root.visibleCount === 0
    text: root.totalGames === 0
      ? "No games found — run ogm scan"
      : "No games match your filters"
    color: root.theme.muted
    font.pixelSize: root.theme.fontL
  }
}
