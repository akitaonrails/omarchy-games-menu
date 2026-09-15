pragma ComponentBehavior: Bound

import QtQuick

// Sectioned grid of game cards: one labeled section per group (Games,
// Emulators, Tools) so they never mix. Cell size derives from
// prefs.cover_scale (via theme). Plain wheel scrolls (Flickable default);
// Ctrl+wheel zooms via signal so shell.qml can clamp + persist cover_scale.
Item {
  id: root

  property Theme theme: null
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

  // The one card whose context menu is open, across all sections; null = none.
  property Item menuCard: null

  // Base card 200x300 cover + label block, padded to a cell. Scaled by both
  // the user zoom (coverScale) and the output scale (uiScale).
  readonly property real s: theme ? theme.coverScale * theme.uiScale : 1
  readonly property int cellW: Math.max(90, Math.round(212 * s))
  readonly property int cellH: Math.max(170, Math.round(396 * s))

  Flickable {
    id: flick
    anchors.fill: parent
    clip: true
    contentWidth: width
    contentHeight: column.height
    boundsBehavior: Flickable.StopAtBounds

    WheelHandler {
      acceptedModifiers: Qt.ControlModifier
      onWheel: function(event) { root.zoomRequested(event.angleDelta.y > 0 ? 0.1 : -0.1) }
    }

    Column {
      id: column
      width: flick.width
      spacing: root.theme ? root.theme.u(28) : 28

      Repeater {
        model: root.sections

        delegate: Column {
          id: section
          required property var modelData
          width: column.width
          spacing: root.theme ? root.theme.u(12) : 12

          Row {
            spacing: 10
            Text {
              id: sectionTitle
              text: section.modelData.title
              color: root.theme ? root.theme.text : "#c7d5e0"
              font.pixelSize: root.theme ? root.theme.fontL : 16
              font.bold: true
            }
            Text {
              anchors.baseline: sectionTitle.baseline
              text: section.modelData.games.length
              color: root.theme ? root.theme.muted : "#8f98a0"
              font.pixelSize: root.theme ? root.theme.fontS : 11
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
            Behavior on cellWidth { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }
            Behavior on cellHeight { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }

            delegate: GameCard {
              id: card
              required property var modelData
              width: grid.cellWidth - 12
              height: grid.cellHeight - 12
              theme: root.theme
              game: modelData
              menuOpen: root.menuCard === card
              onToggleMenu: root.menuCard = (root.menuCard === card) ? null : card
              onCloseMenu: if (root.menuCard === card) root.menuCard = null
              onPlay: { root.menuCard = null; root.playRequested(card.game) }
              onRemoveGame: { root.menuCard = null; root.removeRequested(card.game) }
              onCopyExec: { root.menuCard = null; root.copyExecRequested(String(card.game.exec || "")) }
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
    color: root.theme ? root.theme.muted : "#8f98a0"
    font.pixelSize: root.theme ? root.theme.fontL : 16
  }
}
