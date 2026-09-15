pragma ComponentBehavior: Bound

import QtQuick

// Steam-like game card: 2:3 cover, name, category chip, version line,
// update badge. Null-safe for games without github/sgdb.
Item {
  id: root

  property Theme theme: null
  property var game: null
  property bool menuOpen: false

  signal play()
  signal removeGame()
  signal copyExec()
  signal toggleMenu()
  signal closeMenu()

  readonly property var gh: game && game.github ? game.github : null
  readonly property var sg: game && game.sgdb ? game.sgdb : null
  readonly property bool hasUpdate: gh && gh.has_update === true
  readonly property string coverPath: sg && sg.cover ? String(sg.cover) : ""
  readonly property string versionLine: {
    var inst = game && game.installed_version ? String(game.installed_version) : ""
    var latest = gh && gh.latest_tag ? String(gh.latest_tag) : ""
    if (inst && latest) return "installed: " + inst + " · latest: " + latest
    if (latest) return "latest: " + latest
    if (inst) return "installed: " + inst
    return ""
  }

  scale: cardMouse.containsMouse && !root.menuOpen ? 1.03 : 1.0
  Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }

  Column {
    anchors.fill: parent
    spacing: root.theme ? root.theme.u(8) : 8

    Item {
      id: coverBox
      width: parent.width
      height: Math.round(width * 1.5)

      // Placeholder under the image: gradient + first letter, also shown
      // permanently for games without a cover.
      Rectangle {
        anchors.fill: parent
        radius: 6
        gradient: Gradient {
          GradientStop { position: 0.0; color: root.theme ? root.theme.card : "#2a475e" }
          GradientStop { position: 1.0; color: root.theme ? root.theme.bg : "#1b2838" }
        }
        Text {
          anchors.centerIn: parent
          text: root.game && root.game.name ? String(root.game.name).charAt(0).toUpperCase() : "?"
          color: root.theme ? root.theme.muted : "#8f98a0"
          font.pixelSize: root.theme ? root.theme.fontHero : 64
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

      Rectangle {
        id: badge
        visible: root.hasUpdate
        anchors { top: parent.top; right: parent.right; margins: root.theme ? root.theme.u(8) : 8 }
        width: badgeLabel.implicitWidth + 14
        height: badgeLabel.implicitHeight + 8
        radius: height / 2
        color: root.theme ? root.theme.update : "#f0a232"
        Text {
          id: badgeLabel
          anchors.centerIn: parent
          text: "↑ v" + (root.gh && root.gh.latest_tag ? root.gh.latest_tag : "")
          color: "#0d1117"
          font.pixelSize: root.theme ? root.theme.fontS : 11
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
        radius: 4
        color: "#0d1117"
        border.width: 1
        border.color: root.theme ? root.theme.update : "#f0a232"
        z: 10
        Text {
          id: tipLabel
          anchors.centerIn: parent
          text: "Update available: " + (root.gh && root.gh.latest_tag ? root.gh.latest_tag : "")
            + " (installed " + (root.game && root.game.installed_version ? root.game.installed_version : "?") + ")"
          color: root.theme ? root.theme.text : "#c7d5e0"
          font.pixelSize: root.theme ? root.theme.fontS : 11
        }
      }
    }

    Text {
      width: parent.width
      text: root.game && root.game.name ? root.game.name : ""
      color: root.theme ? root.theme.text : "#c7d5e0"
      font.pixelSize: root.theme ? root.theme.fontM : 13
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
      border.width: 1
      border.color: root.theme ? root.theme.muted : "#8f98a0"
      Text {
        id: catLabel
        anchors.centerIn: parent
        text: root.game && root.game.category ? root.game.category : ""
        color: root.theme ? root.theme.muted : "#8f98a0"
        font.pixelSize: root.theme ? root.theme.fontS : 11
      }
    }

    Text {
      visible: root.versionLine !== ""
      width: parent.width
      text: root.versionLine
      color: root.theme ? root.theme.muted : "#8f98a0"
      font.pixelSize: root.theme ? root.theme.fontS : 11
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
    anchors { right: parent.right; top: parent.top; margins: root.theme ? root.theme.u(6) : 6 }
    width: root.theme ? root.theme.u(150) : 150
    height: menuCol.height + 8
    radius: 6
    color: "#10161f"
    border.width: 1
    border.color: root.theme ? root.theme.cardEdge : "#3a5f7e"
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
          height: root.theme ? root.theme.u(28) : 28
          radius: 4
          color: itemMouse.containsMouse ? (root.theme ? root.theme.card : "#2a475e") : "transparent"
          Text {
            anchors { verticalCenter: parent.verticalCenter; left: parent.left; leftMargin: 8 }
            text: menuItem.modelData.label
            color: root.theme ? root.theme.text : "#c7d5e0"
            font.pixelSize: root.theme ? root.theme.fontS : 11
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
