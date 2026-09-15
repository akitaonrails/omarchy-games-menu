pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

// Top bar: search, category filter chips, sort controls, refresh spinner,
// zoom hint, add-game button. Purely reflects `prefs`; all changes go up as
// signals so shell.qml owns the prefs writes.
ColumnLayout {
  id: root

  property Theme theme: null
  property var prefs: ({})
  property var categories: []
  property bool spinning: false

  signal queryEdited(string q)
  signal categoryToggled(string c)
  signal sortRequested(string key)
  signal addRequested()

  function focusSearch() { searchField.forceActiveFocus() }

  spacing: root.theme ? root.theme.u(10) : 10

  RowLayout {
    Layout.fillWidth: true
    spacing: root.theme ? root.theme.u(12) : 12

    TextField {
      id: searchField
      Layout.preferredWidth: root.theme ? root.theme.u(280) : 280
      text: root.prefs.query || ""
      placeholderText: "Search games…"
      color: root.theme ? root.theme.text : "#c7d5e0"
      placeholderTextColor: root.theme ? root.theme.muted : "#8f98a0"
      font.pixelSize: root.theme ? root.theme.fontM : 13
      onTextEdited: root.queryEdited(text)
      background: Rectangle {
        radius: 6
        color: root.theme ? root.theme.card : "#2a475e"
        border.width: 1
        border.color: searchField.activeFocus
          ? (root.theme ? root.theme.accent : "#66c0f4") : "transparent"
      }
    }

    // Sort: clicking the active key toggles asc/desc.
    Row {
      spacing: 6
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
          width: sortLabel.implicitWidth + (root.theme ? root.theme.u(20) : 20)
          height: root.theme ? root.theme.u(30) : 30
          radius: 6
          color: active ? (root.theme ? root.theme.card : "#2a475e") : "transparent"
          border.width: 1
          border.color: active
            ? (root.theme ? root.theme.accent : "#66c0f4")
            : (root.theme ? root.theme.cardEdge : "#3a5f7e")
          Text {
            id: sortLabel
            anchors.centerIn: parent
            text: sortBtn.modelData.label + (sortBtn.active
              ? (root.prefs.sort_dir === "asc" ? " ▲" : " ▼") : "")
            color: sortBtn.active
              ? (root.theme ? root.theme.accent : "#66c0f4")
              : (root.theme ? root.theme.muted : "#8f98a0")
            font.pixelSize: root.theme ? root.theme.fontS : 11
          }
          MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: root.sortRequested(sortBtn.modelData.key)
          }
        }
      }
    }

    Text {
      visible: root.spinning
      text: "⟳"
      color: root.theme ? root.theme.accent : "#66c0f4"
      font.pixelSize: root.theme ? root.theme.fontL : 16
      RotationAnimator on rotation {
        running: root.spinning
        from: 0; to: 360; duration: 1000
        loops: Animation.Infinite
      }
    }

    Item { Layout.fillWidth: true }

    Text {
      text: "Ctrl+Scroll zoom · type to search · F5 reload · Esc quit"
      color: root.theme ? root.theme.muted : "#8f98a0"
      font.pixelSize: root.theme ? root.theme.fontS : 11
    }

    Rectangle {
      id: addBtn
      implicitWidth: addLabel.implicitWidth + (root.theme ? root.theme.u(24) : 24)
      implicitHeight: root.theme ? root.theme.u(30) : 30
      radius: 6
      color: addMa.containsMouse
        ? (root.theme ? root.theme.accent : "#66c0f4")
        : (root.theme ? root.theme.card : "#2a475e")
      Text {
        id: addLabel
        anchors.centerIn: parent
        text: "+ Add game"
        color: addMa.containsMouse ? "#0d1117" : (root.theme ? root.theme.text : "#c7d5e0")
        font.pixelSize: root.theme ? root.theme.fontS : 11
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
    spacing: 6
    Repeater {
      model: root.categories
      delegate: Rectangle {
        id: chip
        required property var modelData
        readonly property bool checked: (root.prefs.categories || []).indexOf(modelData) !== -1
        width: chipLabel.implicitWidth + (root.theme ? root.theme.u(16) : 16)
        height: root.theme ? root.theme.u(24) : 24
        radius: 12
        color: checked
          ? (root.theme ? root.theme.accent : "#66c0f4")
          : (root.theme ? root.theme.card : "#2a475e")
        Text {
          id: chipLabel
          anchors.centerIn: parent
          text: chip.modelData
          color: chip.checked ? "#0d1117" : (root.theme ? root.theme.muted : "#8f98a0")
          font.pixelSize: root.theme ? root.theme.fontS : 11
        }
        MouseArea {
          anchors.fill: parent
          cursorShape: Qt.PointingHandCursor
          onClicked: root.categoryToggled(chip.modelData)
        }
      }
    }
  }
}
