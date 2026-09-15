pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

// Modal "add game" dialog. Emits saveRequested(argv-after-"ogm") with empty
// optional args omitted; shell.qml runs the process and closes the dialog.
Item {
  id: root

  property Theme theme: null
  property var categories: []
  property bool isOpen: false
  property string selectedCategory: "port"

  signal saveRequested(var args)

  visible: root.isOpen

  function open() {
    nameField.text = ""
    execField.text = ""
    githubField.text = ""
    sgdbField.text = ""
    root.selectedCategory = "port"
    root.isOpen = true
    nameField.forceActiveFocus()
  }

  function close() { root.isOpen = false }

  readonly property bool valid: nameField.text.trim() !== "" && execField.text.trim() !== ""

  component Field: TextField {
    Layout.fillWidth: true
    color: root.theme ? root.theme.text : "#c7d5e0"
    placeholderTextColor: root.theme ? root.theme.muted : "#8f98a0"
    font.pixelSize: root.theme ? root.theme.fontM : 13
    background: Rectangle {
      radius: 6
      color: root.theme ? root.theme.card : "#2a475e"
      border.width: 1
      border.color: parent.activeFocus
        ? (root.theme ? root.theme.accent : "#66c0f4") : "transparent"
    }
  }

  component FieldLabel: Text {
    color: root.theme ? root.theme.muted : "#8f98a0"
    font.pixelSize: root.theme ? root.theme.fontS : 11
  }

  // Dim layer, also swallows clicks (click outside the panel closes).
  Rectangle {
    anchors.fill: parent
    color: "#B30d1117"
    MouseArea { anchors.fill: parent; onClicked: root.close() }
  }

  Rectangle {
    id: panel
    anchors.centerIn: parent
    width: root.theme ? root.theme.u(480) : 480
    height: panelCol.implicitHeight + (root.theme ? root.theme.u(48) : 48)
    radius: 10
    color: root.theme ? root.theme.bg : "#1b2838"
    border.width: 1
    border.color: root.theme ? root.theme.cardEdge : "#3a5f7e"
    MouseArea { anchors.fill: parent } // swallow so clicks don't reach the dim layer

    ColumnLayout {
      id: panelCol
      anchors { left: parent.left; right: parent.right; top: parent.top; margins: root.theme ? root.theme.u(24) : 24 }
      spacing: root.theme ? root.theme.u(10) : 10

      Text {
        text: "Add game"
        color: root.theme ? root.theme.text : "#c7d5e0"
        font.pixelSize: root.theme ? root.theme.fontL : 16
        font.bold: true
      }

      FieldLabel { text: "Name *" }
      Field { id: nameField; placeholderText: "My game" }

      FieldLabel { text: "Exec command *" }
      Field { id: execField; placeholderText: "/usr/bin/distrobox-enter -n gaming -- …" }

      FieldLabel { text: "Category" }
      Flow {
        Layout.fillWidth: true
        spacing: 6
        Repeater {
          model: root.categories
          delegate: Rectangle {
            id: catChip
            required property var modelData
            readonly property bool checked: root.selectedCategory === modelData
            width: catChipLabel.implicitWidth + (root.theme ? root.theme.u(16) : 16)
            height: root.theme ? root.theme.u(24) : 24
            radius: 12
            color: checked
              ? (root.theme ? root.theme.accent : "#66c0f4")
              : (root.theme ? root.theme.card : "#2a475e")
            Text {
              id: catChipLabel
              anchors.centerIn: parent
              text: catChip.modelData
              color: catChip.checked ? "#0d1117" : (root.theme ? root.theme.muted : "#8f98a0")
              font.pixelSize: root.theme ? root.theme.fontS : 11
            }
            MouseArea {
              anchors.fill: parent
              cursorShape: Qt.PointingHandCursor
              onClicked: root.selectedCategory = catChip.modelData
            }
          }
        }
      }

      FieldLabel { text: "GitHub repo (owner/repo, optional)" }
      Field { id: githubField; placeholderText: "HarbourMasters/Shipwright" }

      FieldLabel { text: "SteamGridDB search name (optional)" }
      Field { id: sgdbField; placeholderText: "Defaults to the game name" }

      RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: 8
        spacing: 10

        Item { Layout.fillWidth: true }

        Rectangle {
          implicitWidth: cancelLabel.implicitWidth + (root.theme ? root.theme.u(24) : 24)
          implicitHeight: root.theme ? root.theme.u(32) : 32
          radius: 6
          color: cancelMa.containsMouse
            ? (root.theme ? root.theme.card : "#2a475e") : "transparent"
          border.width: 1
          border.color: root.theme ? root.theme.cardEdge : "#3a5f7e"
          Text {
            id: cancelLabel
            anchors.centerIn: parent
            text: "Cancel"
            color: root.theme ? root.theme.text : "#c7d5e0"
            font.pixelSize: root.theme ? root.theme.fontS : 11
          }
          MouseArea {
            id: cancelMa
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: root.close()
          }
        }

        Rectangle {
          implicitWidth: saveLabel.implicitWidth + (root.theme ? root.theme.u(24) : 24)
          implicitHeight: root.theme ? root.theme.u(32) : 32
          radius: 6
          opacity: root.valid ? 1 : 0.45
          color: saveMa.containsMouse && root.valid
            ? (root.theme ? root.theme.accent : "#66c0f4")
            : (root.theme ? root.theme.card : "#2a475e")
          Text {
            id: saveLabel
            anchors.centerIn: parent
            text: "Save"
            color: saveMa.containsMouse && root.valid
              ? "#0d1117" : (root.theme ? root.theme.text : "#c7d5e0")
            font.pixelSize: root.theme ? root.theme.fontS : 11
            font.bold: true
          }
          MouseArea {
            id: saveMa
            anchors.fill: parent
            hoverEnabled: true
            enabled: root.valid
            cursorShape: root.valid ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: {
              var args = ["add",
                "--name", nameField.text.trim(),
                "--exec", execField.text.trim(),
                "--category", root.selectedCategory]
              if (githubField.text.trim() !== "")
                args = args.concat(["--github", githubField.text.trim()])
              if (sgdbField.text.trim() !== "")
                args = args.concat(["--sgdb-query", sgdbField.text.trim()])
              root.saveRequested(args)
              root.close()
            }
          }
        }
      }
    }
  }
}
