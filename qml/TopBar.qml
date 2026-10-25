pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

// Top bar: search, category filter chips, sort controls, refresh spinner,
// zoom hint, add-game button. Purely reflects `prefs`; all changes go up as
// signals so shell.qml owns the prefs writes.
ColumnLayout {
  id: root

  required property Theme theme
  property var prefs: ({})
  property var categories: []
  property bool spinning: false

  signal queryEdited(string q)
  signal categoryToggled(string c)
  signal sortRequested(string key)
  signal zoomPresetRequested(real scale)
  signal addRequested()

  function focusSearch() { searchField.forceActiveFocus() }

  spacing: root.theme.space2

  RowLayout {
    Layout.fillWidth: true
    spacing: root.theme.space3

    TextField {
      id: searchField
      Layout.preferredWidth: root.theme.u(280)
      text: root.prefs.query || ""
      placeholderText: "Search games…"
      Accessible.name: "Search games"
      color: root.theme.text
      placeholderTextColor: root.theme.muted
      font.pixelSize: root.theme.fontM
      onTextEdited: root.queryEdited(text)
      background: Rectangle {
        radius: root.theme.radius
        color: root.theme.card
        border.width: searchField.activeFocus
          ? root.theme.focusWidth : root.theme.borderWidth
        border.color: searchField.activeFocus
          ? root.theme.accent : root.theme.cardEdge
      }
    }

    // Sort: clicking the active key toggles asc/desc.
    Row {
      spacing: root.theme.space1
      Repeater {
        model: [
          { key: "most_played", label: "Most played" },
          { key: "name", label: "Name" },
          { key: "date_added", label: "Date added" },
          { key: "release_date", label: "Release date" }
        ]
        delegate: Rectangle {
          id: sortBtn
          required property var modelData
          readonly property bool active: root.prefs.sort_key === modelData.key
          function activate() { root.sortRequested(modelData.key) }
          width: sortLabel.implicitWidth + root.theme.u(20)
          height: root.theme.controlHeight
          radius: root.theme.radius
          activeFocusOnTab: true
          Accessible.role: Accessible.Button
          Accessible.name: "Sort by " + modelData.label
          Accessible.selected: active
          Accessible.onPressAction: sortBtn.activate()
          Keys.onReturnPressed: sortBtn.activate()
          Keys.onEnterPressed: sortBtn.activate()
          Keys.onSpacePressed: sortBtn.activate()
          color: active ? root.theme.card : "transparent"
          border.width: activeFocus
            ? root.theme.focusWidth : root.theme.borderWidth
          border.color: active || activeFocus
            ? root.theme.accent : root.theme.cardEdge
          Text {
            id: sortLabel
            anchors.centerIn: parent
            text: sortBtn.modelData.label + (sortBtn.active
              ? (root.prefs.sort_dir === "asc" ? " ▲" : " ▼") : "")
            color: sortBtn.active
              ? root.theme.accent : root.theme.muted
            font.pixelSize: root.theme.fontS
          }
          MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: sortBtn.activate()
          }
        }
      }
    }

    Text {
      visible: root.spinning
      text: "⟳"
      color: root.theme.accent
      font.pixelSize: root.theme.fontL
      RotationAnimator on rotation {
        running: root.spinning && !root.theme.reducedMotion
        from: 0; to: 360; duration: root.theme.motionCycle
        loops: Animation.Infinite
      }
    }

    Item { Layout.fillWidth: true }

    // Cover-size presets: a guaranteed zoom path even where Ctrl+wheel is
    // unavailable. Active when cover_scale matches (within a step).
    Row {
      spacing: root.theme.space1
      Repeater {
        model: [
          { label: "S", scale: 0.7 },
          { label: "M", scale: 1.0 },
          { label: "L", scale: 1.5 }
        ]
        delegate: Rectangle {
          id: zoomBtn
          required property var modelData
          readonly property bool active:
            Math.abs((root.prefs.cover_scale || 1) - modelData.scale) < 0.05
          function activate() { root.zoomPresetRequested(modelData.scale) }
          width: root.theme.u(26)
          height: root.theme.controlHeight
          radius: root.theme.radius
          activeFocusOnTab: true
          Accessible.role: Accessible.Button
          Accessible.name: "Cover size " + modelData.label
          Accessible.selected: active
          Accessible.onPressAction: zoomBtn.activate()
          Keys.onReturnPressed: zoomBtn.activate()
          Keys.onEnterPressed: zoomBtn.activate()
          Keys.onSpacePressed: zoomBtn.activate()
          color: active ? root.theme.card : "transparent"
          border.width: activeFocus
            ? root.theme.focusWidth : root.theme.borderWidth
          border.color: active || activeFocus
            ? root.theme.accent : root.theme.cardEdge
          Text {
            anchors.centerIn: parent
            text: zoomBtn.modelData.label
            color: zoomBtn.active
              ? root.theme.accent : root.theme.muted
            font.pixelSize: root.theme.fontS
            font.bold: zoomBtn.active
          }
          MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: zoomBtn.activate()
          }
        }
      }
    }

    Text {
      text: "Arrows/stick move · Enter/A play · type to search · Ctrl+N add · Esc/B quit"
      color: root.theme.muted
      font.pixelSize: root.theme.fontS
    }

    Rectangle {
      id: addBtn
      implicitWidth: addLabel.implicitWidth + root.theme.u(24)
      implicitHeight: root.theme.controlHeight
      radius: root.theme.radius
      activeFocusOnTab: true
      Accessible.role: Accessible.Button
      Accessible.name: "Add game"
      Accessible.onPressAction: root.addRequested()
      Keys.onReturnPressed: root.addRequested()
      Keys.onEnterPressed: root.addRequested()
      Keys.onSpacePressed: root.addRequested()
      color: root.theme.card
      border.width: activeFocus
        ? root.theme.focusWidth : root.theme.borderWidth
      border.color: addMa.containsMouse || activeFocus
        ? root.theme.accent : root.theme.cardEdge
      Text {
        id: addLabel
        anchors.centerIn: parent
        text: "+ Add game"
        color: addMa.containsMouse || addBtn.activeFocus
          ? root.theme.accent : root.theme.text
        font.pixelSize: root.theme.fontS
        font.bold: true
      }
      MouseArea {
        id: addMa
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.addRequested()
      }
    }
  }

  // Category filter chips: multi-select, empty selection = all.
  Flow {
    Layout.fillWidth: true
    spacing: root.theme.space1
    Repeater {
      model: root.categories
      delegate: Rectangle {
        id: chip
        required property var modelData
        readonly property bool checked: (root.prefs.categories || []).indexOf(modelData) !== -1
        function activate() { root.categoryToggled(modelData) }
        width: chipLabel.implicitWidth + root.theme.u(16)
        height: root.theme.controlHeight
        radius: root.theme.radius
        activeFocusOnTab: true
        Accessible.role: Accessible.CheckBox
        Accessible.name: "Filter " + modelData
        Accessible.checked: checked
        Accessible.onToggleAction: chip.activate()
        Keys.onReturnPressed: chip.activate()
        Keys.onEnterPressed: chip.activate()
        Keys.onSpacePressed: chip.activate()
        color: root.theme.card
        border.width: activeFocus
          ? root.theme.focusWidth : root.theme.borderWidth
        border.color: checked || activeFocus
          ? root.theme.accent : root.theme.cardEdge
        Text {
          id: chipLabel
          anchors.centerIn: parent
          text: (chip.checked ? "✓ " : "") + chip.modelData
          color: chip.checked
            ? root.theme.accent : root.theme.muted
          font.pixelSize: root.theme.fontS
        }
        MouseArea {
          anchors.fill: parent
          cursorShape: Qt.PointingHandCursor
          onClicked: chip.activate()
        }
      }
    }
  }
}
