pragma ComponentBehavior: Bound

import QtQuick

// Steam-like game card: 2:3 cover, name, category chip, version line,
// update badge. Null-safe for games without github/sgdb.
Item {
  id: root

  required property Theme theme
  property var game: null
  property bool menuOpen: false
  // Keyboard/controller cursor. Independent of hover so a resting mouse
  // pointer never looks like the selection.
  property bool selected: false

  signal play()
  signal removeGame()
  signal copyExec()
  signal toggleMenu()
  signal closeMenu()

  readonly property var gh: game && game.github ? game.github : null
  readonly property var wb: game && game.web ? game.web : null
  readonly property var sg: game && game.sgdb ? game.sgdb : null
  readonly property bool hasUpdate:
    (gh && gh.has_update === true) || (wb && wb.has_update === true)
  readonly property string badgeText: {
    if (root.gh && root.gh.has_update === true)
      return "↑ v" + (root.gh.latest_tag ? root.gh.latest_tag : "")
    if (root.wb && root.wb.latest_is_version === true && root.wb.latest)
      return "↑ " + String(root.wb.latest)
    return "↑ site"
  }
  readonly property string badgeTipText: {
    if (root.gh && root.gh.has_update === true)
      return "Update available: " + (root.gh.latest_tag ? root.gh.latest_tag : "")
        + " (installed " + (root.game && root.game.installed_version ? root.game.installed_version : "?") + ")"
    var host = ""
    if (root.wb && root.wb.update_url)
      host = String(root.wb.update_url).replace(/^https?:\/\//, "").split("/")[0]
    if (root.wb && root.wb.latest_is_version === true && root.wb.latest)
      return "Update available: " + String(root.wb.latest) + (host ? " (" + host + ")" : "")
    return "Download page changed" + (host ? " (" + host + ")" : "")
  }
  readonly property string coverPath: sg && sg.cover ? String(sg.cover) : ""
  readonly property string versionLine: {
    var inst = game && game.installed_version ? String(game.installed_version) : ""
    var latest = gh && gh.latest_tag ? String(gh.latest_tag) : ""
    if (inst && latest) return "installed: " + inst + " · latest: " + latest
    if (latest) return "latest: " + latest
    if (inst) return "installed: " + inst
    return ""
  }
  Accessible.role: Accessible.ListItem
  Accessible.name: game && game.name ? game.name : "Game"
  Accessible.description: versionLine
  Accessible.selected: selected
  Accessible.onPressAction: root.play()

  Column {
    anchors.fill: parent
    spacing: root.theme.u(8)

    Item {
      id: coverBox
      width: parent.width
      height: Math.round(width * 1.5)

      // Placeholder under the image: gradient + first letter, also shown
      // permanently for games without a cover.
      Rectangle {
        anchors.fill: parent
        radius: root.theme.radius
        gradient: Gradient {
          GradientStop { position: 0.0; color: root.theme.card }
          GradientStop { position: 1.0; color: root.theme.bg }
        }
        Text {
          anchors.centerIn: parent
          text: root.game && root.game.name ? String(root.game.name).charAt(0).toUpperCase() : "?"
          color: root.theme.muted
          font.pixelSize: root.theme.fontHero
          font.bold: true
        }
      }

      Image {
        anchors.fill: parent
        source: root.coverPath ? "file://" + root.coverPath : ""
        asynchronous: true
        cache: false
        smooth: true
        fillMode: Image.PreserveAspectCrop
        visible: status === Image.Ready
      }

      // Selection ring, drawn over the cover so it reads on art of any
      // brightness. Above the image, below the update badge.
      Rectangle {
        anchors.fill: parent
        radius: root.theme.radius
        visible: root.selected || cardMouse.containsMouse
        color: "transparent"
        border.width: root.selected
          ? root.theme.focusWidth : root.theme.borderWidth
        border.color: root.selected
          ? root.theme.accent : root.theme.cardEdge
        z: 5
      }

      Rectangle {
        id: badge
        visible: root.hasUpdate
        anchors { top: parent.top; right: parent.right; margins: root.theme.u(8) }
        width: badgeLabel.implicitWidth + 14
        height: badgeLabel.implicitHeight + 8
        radius: height / 2
        color: root.theme.update
        Text {
          id: badgeLabel
          anchors.centerIn: parent
          text: root.badgeText
          color: root.theme.bg
          font.pixelSize: root.theme.fontS
          font.bold: true
        }
        // Hover-only; clicks fall through to the card MouseArea.
        MouseArea {
          id: badgeMouse
          anchors.fill: parent
          hoverEnabled: true
          acceptedButtons: Qt.NoButton
        }
      }

      Rectangle {
        id: badgeTip
        visible: root.hasUpdate && badgeMouse.containsMouse
        anchors { top: badge.bottom; right: badge.right; topMargin: 4 }
        width: tipLabel.implicitWidth + 14
        height: tipLabel.implicitHeight + 8
        radius: root.theme.radius
        color: root.theme.bg
        border.width: root.theme.borderWidth
        border.color: root.theme.update
        z: 10
        Text {
          id: tipLabel
          anchors.centerIn: parent
          text: root.badgeTipText
          color: root.theme.text
          font.pixelSize: root.theme.fontS
        }
      }
    }

    Text {
      width: parent.width
      text: root.game && root.game.name ? root.game.name : ""
      color: root.theme.text
      font.pixelSize: root.theme.fontM
      elide: Text.ElideRight
      wrapMode: Text.WordWrap
      maximumLineCount: 2
    }

    Rectangle {
      visible: root.game && !!root.game.category
      width: catLabel.implicitWidth + 12
      height: catLabel.implicitHeight + 6
      radius: height / 2
      color: "transparent"
      border.width: root.theme.borderWidth
      border.color: root.theme.muted
      Text {
        id: catLabel
        anchors.centerIn: parent
        text: root.game && root.game.category ? root.game.category : ""
        color: root.theme.muted
        font.pixelSize: root.theme.fontS
      }
    }

    Text {
      visible: root.versionLine !== ""
      width: parent.width
      text: root.versionLine
      color: root.theme.muted
      font.pixelSize: root.theme.fontS
      elide: Text.ElideRight
    }
  }

  MouseArea {
    id: cardMouse
    anchors.fill: parent
    hoverEnabled: true
    acceptedButtons: Qt.LeftButton | Qt.RightButton
    cursorShape: Qt.PointingHandCursor
    onClicked: function(mouse) {
      if (mouse.button === Qt.RightButton) {
        root.toggleMenu()
      } else if (root.menuOpen) {
        root.closeMenu()
      } else {
        root.play()
      }
    }
  }

  // Inline context menu (deliberately not QtQuick.Controls Menu).
  Rectangle {
    id: menu
    visible: root.menuOpen
    z: 20
    anchors { right: parent.right; top: parent.top; margins: root.theme.u(6) }
    width: root.theme.u(150)
    height: menuCol.height + 8
    radius: root.theme.radius
    color: root.theme.bg
    border.width: root.theme.borderWidth
    border.color: root.theme.cardEdge
    Column {
      id: menuCol
      anchors { left: parent.left; right: parent.right; top: parent.top; margins: 4 }
      Repeater {
        model: [
          { label: "Play", action: "play" },
          { label: root.game && root.game.custom ? "Remove game" : "Hide game", action: "remove" },
          { label: "Copy exec", action: "copy" }
        ]
        delegate: Rectangle {
          id: menuItem
          required property var modelData
          width: menuCol.width
          height: root.theme.u(28)
          radius: root.theme.radius
          Accessible.role: Accessible.MenuItem
          Accessible.name: modelData.label
          color: itemMouse.containsMouse ? root.theme.card : "transparent"
          Text {
            anchors { verticalCenter: parent.verticalCenter; left: parent.left; leftMargin: 8 }
            text: menuItem.modelData.label
            color: root.theme.text
            font.pixelSize: root.theme.fontS
          }
          MouseArea {
            id: itemMouse
            anchors.fill: parent
            hoverEnabled: true
            onClicked: {
              if (menuItem.modelData.action === "play") root.play()
              else if (menuItem.modelData.action === "remove") root.removeGame()
              else root.copyExec()
            }
          }
        }
      }
    }
  }
}
