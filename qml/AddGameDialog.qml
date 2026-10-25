pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

// Modal "add game" dialog. Emits saveRequested(argv-after-"ogm") with empty
// optional args omitted; shell.qml runs the process and closes the dialog.
Item {
  id: root

  required property Theme theme
  property var categories: []
  property bool isOpen: false
  property string selectedCategory: "port"

  signal saveRequested(var args)

  visible: root.isOpen
  Accessible.role: Accessible.Dialog
  Accessible.name: "Add game"

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

  function save() {
    if (!root.valid) return
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

  readonly property bool valid: nameField.text.trim() !== "" && execField.text.trim() !== ""

  component Field: TextField {
    Layout.fillWidth: true
    color: root.theme.text
    placeholderTextColor: root.theme.muted
    font.pixelSize: root.theme.fontM
    background: Rectangle {
      radius: root.theme.radius
      color: root.theme.card
      border.width: parent.activeFocus
        ? root.theme.focusWidth : root.theme.borderWidth
      border.color: parent.activeFocus
        ? root.theme.accent : root.theme.cardEdge
    }
  }

  component FieldLabel: Text {
    color: root.theme.muted
    font.pixelSize: root.theme.fontS
  }

  // Dim layer, also swallows clicks (click outside the panel closes).
  Rectangle {
    anchors.fill: parent
    color: Qt.rgba(
      root.theme.overlayBase.r,
      root.theme.overlayBase.g,
      root.theme.overlayBase.b,
      0.70)
    MouseArea { anchors.fill: parent; onClicked: root.close() }
  }

  Rectangle {
    id: panel
    anchors.centerIn: parent
    width: root.theme.u(480)
    height: panelCol.implicitHeight + root.theme.u(48)
    radius: root.theme.radius
    color: root.theme.bg
    border.width: root.theme.borderWidth
    border.color: root.theme.cardEdge
    MouseArea { anchors.fill: parent } // swallow so clicks don't reach the dim layer

    ColumnLayout {
      id: panelCol
      anchors { left: parent.left; right: parent.right; top: parent.top; margins: root.theme.u(24) }
      spacing: root.theme.u(10)

      Text {
        text: "Add game"
        color: root.theme.text
        font.pixelSize: root.theme.fontL
        font.bold: true
      }

      FieldLabel { text: "Name *" }
      Field { id: nameField; placeholderText: "My game"; Accessible.name: "Name" }

      FieldLabel { text: "Exec command *" }
      Field { id: execField; placeholderText: "/usr/bin/distrobox-enter -n gaming -- …"; Accessible.name: "Exec command" }

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
            function activate() { root.selectedCategory = modelData }
            width: catChipLabel.implicitWidth + root.theme.u(16)
            height: root.theme.controlHeight
            radius: root.theme.radius
            activeFocusOnTab: true
            Accessible.role: Accessible.RadioButton
            Accessible.name: modelData
            Accessible.checked: checked
            Accessible.onToggleAction: catChip.activate()
            Keys.onReturnPressed: catChip.activate()
            Keys.onEnterPressed: catChip.activate()
            Keys.onSpacePressed: catChip.activate()
            color: root.theme.card
            border.width: activeFocus
              ? root.theme.focusWidth : root.theme.borderWidth
            border.color: checked || activeFocus
              ? root.theme.accent : root.theme.cardEdge
            Text {
              id: catChipLabel
              anchors.centerIn: parent
              text: (catChip.checked ? "● " : "") + catChip.modelData
              color: catChip.checked
                ? root.theme.accent : root.theme.muted
              font.pixelSize: root.theme.fontS
            }
            MouseArea {
              anchors.fill: parent
              cursorShape: Qt.PointingHandCursor
              onClicked: catChip.activate()
            }
          }
        }
      }

      FieldLabel { text: "GitHub repo (owner/repo, optional)" }
      Field { id: githubField; placeholderText: "HarbourMasters/Shipwright"; Accessible.name: "GitHub repository" }

      FieldLabel { text: "SteamGridDB search name (optional)" }
      Field { id: sgdbField; placeholderText: "Defaults to the game name"; Accessible.name: "SteamGridDB search name" }

      RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: 8
        spacing: 10

        Item { Layout.fillWidth: true }

        Rectangle {
          implicitWidth: cancelLabel.implicitWidth + root.theme.u(24)
          implicitHeight: root.theme.controlHeight
          radius: root.theme.radius
          activeFocusOnTab: true
          Accessible.role: Accessible.Button
          Accessible.name: "Cancel"
          Accessible.onPressAction: root.close()
          Keys.onReturnPressed: root.close()
          Keys.onEnterPressed: root.close()
          Keys.onSpacePressed: root.close()
          color: cancelMa.containsMouse
            ? root.theme.card : "transparent"
          border.width: activeFocus
            ? root.theme.focusWidth : root.theme.borderWidth
          border.color: activeFocus
            ? root.theme.accent : root.theme.cardEdge
          Text {
            id: cancelLabel
            anchors.centerIn: parent
            text: "Cancel"
            color: root.theme.text
            font.pixelSize: root.theme.fontS
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
          implicitWidth: saveLabel.implicitWidth + root.theme.u(24)
          implicitHeight: root.theme.controlHeight
          radius: root.theme.radius
          activeFocusOnTab: root.valid
          Accessible.role: Accessible.Button
          Accessible.name: "Save game"
          Accessible.onPressAction: root.save()
          Keys.onReturnPressed: root.save()
          Keys.onEnterPressed: root.save()
          Keys.onSpacePressed: root.save()
          opacity: root.valid ? 1 : 0.45
          color: root.theme.card
          border.width: activeFocus
            ? root.theme.focusWidth : root.theme.borderWidth
          border.color: (saveMa.containsMouse && root.valid) || activeFocus
            ? root.theme.accent : root.theme.cardEdge
          Text {
            id: saveLabel
            anchors.centerIn: parent
            text: "Save"
            color: (saveMa.containsMouse && root.valid) || parent.activeFocus
              ? root.theme.accent : root.theme.text
            font.pixelSize: root.theme.fontS
            font.bold: true
          }
          MouseArea {
            id: saveMa
            anchors.fill: parent
            hoverEnabled: true
            enabled: root.valid
            cursorShape: root.valid ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: root.save()
          }
        }
      }
    }
  }
}
