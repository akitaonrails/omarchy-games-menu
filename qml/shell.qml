// omarchy-games-menu — QuickShell frontend for the `ogm` game launcher.
//
// Run:   quickshell -p /mnt/data/Projects/omarchy-games-menu/qml
//
// Data flow:
//   ~/.local/share/ogm/state.json  (backend-owned, atomically rewritten,
//                                   watched live via FileView)
//   ~/.config/ogm/prefs.json       (owned by THIS UI; debounced atomic writes)
//   state + prefs → visibleSections (sorted/filtered, grouped) → GameGrid
//
// Preview against the bundled fixture without touching real state:
//   mkdir -p /tmp/ogm-dev/ogm /tmp/ogm-dev-cfg
//   cp dev/sample-state.json /tmp/ogm-dev/ogm/state.json
//   XDG_DATA_HOME=/tmp/ogm-dev XDG_CONFIG_HOME=/tmp/ogm-dev-cfg \
//     quickshell -p /mnt/data/Projects/omarchy-games-menu/qml

import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

ShellRoot {
  id: root

  property string home: Quickshell.env("HOME")
  readonly property string dataHome: {
    var x = Quickshell.env("XDG_DATA_HOME")
    return x && x.length ? x : home + "/.local/share"
  }
  readonly property string configHome: {
    var x = Quickshell.env("XDG_CONFIG_HOME")
    return x && x.length ? x : home + "/.config"
  }
  readonly property string statePath: dataHome + "/ogm/state.json"
  readonly property string prefsPath: configHome + "/ogm/prefs.json"

  // Prefer the canonical install path; fall back to PATH lookup (Process
  // resolves bare names via execvp). Probed once at startup.
  property string ogmBin: home + "/.local/bin/ogm"

  readonly property var emptyState: ({ version: 1, games: [], errors: [] })
  property var state: emptyState

  readonly property var defaultPrefs: ({
    version: 1,
    cover_scale: 1.0,
    sort_key: "name",
    sort_dir: "asc",
    categories: [],
    query: ""
  })
  property var prefs: defaultPrefs

  readonly property var allCategories: ["port", "decomp", "recomp", "fangame", "wine", "arcade", "emulator", "tool", "custom"]
  readonly property var sortKeys: ["most_played", "name", "date_added", "release_date"]

  readonly property bool refreshing: scanProc.running || refreshProc.running
  property bool refreshAfterScan: false

  Theme { id: theme; coverScale: root.prefs.cover_scale }

  // ---- system scaling: follow the Hyprland output scale ----
  // Layer-shell surfaces render at logical size; without this, fonts/cards
  // shrink physically on scaled (e.g. 1.5x) outputs. Non-Hyprland or a
  // missing hyprctl leaves uiScale at 1.0.
  Process {
    id: scaleProc
    command: ["hyprctl", "monitors", "-j"]
    stdout: StdioCollector {
      onStreamFinished: root.applyMonitorScale(this.text)
    }
  }

  function applyMonitorScale(jsonText) {
    var scale = 1.0
    var current = win.screen ? String(win.screen.name) : ""
    try {
      var monitors = JSON.parse(String(jsonText || ""))
      if (Array.isArray(monitors)) {
        for (var i = 0; i < monitors.length; i++) {
          var m = monitors[i]
          if (m && (m.name === current || (current === "" && m.focused === true))) {
            var s = Number(m.scale)
            if (s > 0) scale = s
            break
          }
        }
      }
    } catch (e) {
      console.warn("monitor scale detect failed:", e)
    }
    theme.uiScale = Math.min(3.0, Math.max(1.0, scale))
  }

  // ---- state.json: backend-owned, watched live ----
  FileView {
    id: stateFile
    path: root.statePath
    watchChanges: true
    printErrors: false
    onLoaded: root.loadState(text())
    onLoadFailed: function(error) { root.loadState("") }
    onFileChanged: reload()
  }

  // ---- prefs.json: UI-owned, debounced atomic writes ----
  FileView {
    id: prefsFile
    path: root.prefsPath
    watchChanges: true
    atomicWrites: true
    printErrors: false
    onLoaded: root.loadPrefs(text())
    onLoadFailed: function(error) { root.loadPrefs("") }
    onFileChanged: reload()
  }

  Timer {
    id: prefsSaveTimer
    interval: 300
    onTriggered: prefsFile.setText(JSON.stringify(root.prefs, null, 2) + "\n")
  }

  function loadState(raw) {
    var text = String(raw || "").trim()
    if (!text) { state = emptyState; return }
    try {
      var parsed = JSON.parse(text)
      state = (parsed && Array.isArray(parsed.games)) ? parsed : emptyState
    } catch (e) {
      console.warn("state.json parse failed:", e)
      state = emptyState
    }
  }

  function loadPrefs(raw) {
    var p = JSON.parse(JSON.stringify(defaultPrefs))
    var text = String(raw || "").trim()
    if (text) {
      try {
        var parsed = JSON.parse(text)
        if (parsed && typeof parsed === "object") {
          if (typeof parsed.cover_scale === "number")
            p.cover_scale = Math.min(2.5, Math.max(0.5, parsed.cover_scale))
          if (sortKeys.indexOf(parsed.sort_key) !== -1) p.sort_key = parsed.sort_key
          if (parsed.sort_dir === "asc" || parsed.sort_dir === "desc") p.sort_dir = parsed.sort_dir
          if (Array.isArray(parsed.categories))
            p.categories = parsed.categories.filter(function(c) { return allCategories.indexOf(c) !== -1 })
          if (typeof parsed.query === "string") p.query = parsed.query
        }
      } catch (e) {
        console.warn("prefs.json parse failed, using defaults:", e)
      }
    }
    prefs = p
  }

  // Reassign (never mutate in place) so bindings reevaluate; save is debounced
  // so rapid Ctrl+wheel zoom doesn't thrash the disk.
  function updatePrefs(patch) {
    var p = JSON.parse(JSON.stringify(prefs))
    for (var k in patch) p[k] = patch[k]
    p.version = 1
    prefs = p
    prefsSaveTimer.restart()
  }

  function toggleCategory(cat) {
    var cats = (prefs.categories || []).slice()
    var i = cats.indexOf(cat)
    if (i === -1) cats.push(cat); else cats.splice(i, 1)
    updatePrefs({ categories: cats })
  }

  function requestSort(key) {
    if (prefs.sort_key === key)
      updatePrefs({ sort_dir: prefs.sort_dir === "asc" ? "desc" : "asc" })
    else
      updatePrefs({ sort_key: key, sort_dir: "asc" })
  }

  function zoomBy(delta) {
    var s = Math.round((prefs.cover_scale + delta) * 10) / 10
    s = Math.min(2.5, Math.max(0.5, s))
    if (s !== prefs.cover_scale) updatePrefs({ cover_scale: s })
  }

  // Global keys (backdrop has focus; skipped while the add dialog is open).
  // Printable characters become a search query immediately, no need to click
  // the search field first; once focused it owns further typing itself.
  function handleKey(event) {
    if (addDialog.isOpen) return
    var ctrl = (event.modifiers & Qt.ControlModifier) !== 0
    if (event.key === Qt.Key_PageUp) { gameGrid.pageUp(); event.accepted = true; return }
    if (event.key === Qt.Key_PageDown) { gameGrid.pageDown(); event.accepted = true; return }
    if (event.key === Qt.Key_Home) { gameGrid.scrollHome(); event.accepted = true; return }
    if (event.key === Qt.Key_End) { gameGrid.scrollEnd(); event.accepted = true; return }
    if (event.key === Qt.Key_F5 || (ctrl && event.key === Qt.Key_R)) {
      rescan()
      event.accepted = true
      return
    }
    if (event.key === Qt.Key_Backspace) {
      var q0 = String(prefs.query || "")
      if (q0.length > 0) {
        updatePrefs({ query: q0.slice(0, -1) })
        topBar.focusSearch()
        event.accepted = true
      }
      return
    }
    if (ctrl || (event.modifiers & Qt.AltModifier) !== 0) return
    var t = event.text
    if (t && t >= " ") {
      updatePrefs({ query: String(prefs.query || "") + t })
      topBar.focusSearch()
      event.accepted = true
    }
  }

  // ---- sorted/filtered model: pure function of state + prefs ----
  // Games, emulators and tools are never mixed: the grid renders one section
  // per group, each internally sorted by the active sort key.
  readonly property var groupOrder: [
    { key: "games", title: "Games", categories: ["port", "decomp", "recomp", "fangame", "wine", "arcade", "custom"] },
    { key: "emulators", title: "Emulators", categories: ["emulator"] },
    { key: "tools", title: "Tools", categories: ["tool"] }
  ]
  readonly property var visibleSections: computeSections()
  readonly property int visibleCount: {
    var n = 0
    for (var i = 0; i < visibleSections.length; i++) n += visibleSections[i].games.length
    return n
  }

  function groupFor(category) {
    for (var i = 0; i < groupOrder.length; i++)
      if (groupOrder[i].categories.indexOf(category) !== -1) return groupOrder[i].key
    return "games"
  }

  // Frecency: play_count weighted by how recently the game was last launched.
  // Decay steps must match ogm-core::sort::frecency_score.
  function frecencyScore(g) {
    var count = (g && typeof g.play_count === "number") ? g.play_count : 0
    if (count <= 0 || !g.last_played) return 0
    var days = (Date.now() - Date.parse(g.last_played)) / 86400000
    if (isNaN(days)) return 0
    var decay = days <= 7 ? 1.0 : days <= 30 ? 0.6 : days <= 90 ? 0.3 : 0.1
    return count * decay
  }

  function computeVisible() {
    var games = (state && state.games) || []
    var cats = prefs.categories || []
    var q = String(prefs.query || "").toLowerCase()
    var filtered = games.filter(function(g) {
      if (!g) return false
      if (cats.length > 0 && cats.indexOf(g.category) === -1) return false
      if (q && String(g.name || "").toLowerCase().indexOf(q) === -1) return false
      return true
    })
    filtered.sort(gameComparator)
    return filtered
  }

  function computeSections() {
    var sorted = computeVisible()
    var out = []
    for (var i = 0; i < groupOrder.length; i++) {
      var group = groupOrder[i]
      var members = sorted.filter(function(g) { return groupFor(g.category) === group.key })
      if (members.length > 0)
        out.push({ key: group.key, title: group.title, games: members })
    }
    return out
  }

  function gameComparator(a, b) {
    var key = prefs.sort_key
    var dir = prefs.sort_dir === "desc" ? -1 : 1
    var cmp = 0
      if (key === "most_played") {
        // Frecency: play count weighted by recency of last launch. Mirrors
        // ogm-core's frecency_score — keep the decay steps in sync.
        var sa = frecencyScore(a), sb = frecencyScore(b)
        // Never-played games always sink to the bottom, either direction.
        if (sa === 0 && sb === 0) cmp = 0
        else if (sa === 0) return 1
        else if (sb === 0) return -1
        else cmp = sa - sb
        // Higher score first is the natural order, so flip the direction.
        return cmp === 0 ? 0 : (cmp < 0 ? dir : -dir)
      } else if (key === "date_added") {
        cmp = String(a.added_at || "").localeCompare(String(b.added_at || ""))
      } else if (key === "release_date") {
        var ra = a.sgdb && a.sgdb.release_date
        var rb = b.sgdb && b.sgdb.release_date
        // Games without a release date always sort last, either direction.
        if (!ra && !rb) return 0
        if (!ra) return 1
        if (!rb) return -1
        cmp = String(ra).localeCompare(String(rb))
      } else {
        cmp = String(a.name || "").toLowerCase().localeCompare(String(b.name || "").toLowerCase())
      }
      return cmp * dir
  }

  // ---- ogm CLI processes ----
  function ogmCmd(args) { return [ogmBin].concat(args) }

  Process {
    id: ogmProbe
    command: ["test", "-x", root.home + "/.local/bin/ogm"]
    onExited: function(exitCode) {
      if (exitCode !== 0) root.ogmBin = "ogm"
      root.runRefresh()
    }
  }

  Process {
    id: scanProc
    onExited: function() {
      if (root.refreshAfterScan) {
        root.refreshAfterScan = false
        refreshProc.command = root.ogmCmd(["refresh"])
        refreshProc.running = true
      }
    }
  }
  Process { id: refreshProc }
  Process { id: launchProc; onExited: Qt.quit() }
  Process { id: addProc; onExited: root.rescan() }
  Process { id: removeProc; onExited: root.rescan() }

  Timer {
    id: refreshTimer
    interval: 6 * 3600 * 1000
    running: true
    repeat: true
    onTriggered: root.runRefresh()
  }

  // Safety net: quit even if `ogm launch` fails to spawn (backend detaches
  // the game itself, so a quick exit is expected).
  Timer { id: quitTimer; interval: 1500; onTriggered: Qt.quit() }

  function runRefresh() { refreshAfterScan = true; startScan() }
  function rescan() { refreshAfterScan = false; startScan() }
  function startScan() {
    if (scanProc.running) return
    scanProc.command = ogmCmd(["scan"])
    scanProc.running = true
  }

  function launchGame(game) {
    if (!game || !game.id) return
    launchProc.command = ogmCmd(["launch", game.id])
    launchProc.running = true
    quitTimer.restart()
  }

  function addGame(args) {
    addProc.command = ogmCmd(args)
    addProc.running = true
  }

  function removeGame(game) {
    if (!game || !game.id) return
    removeProc.command = ogmCmd([game.custom ? "remove" : "hide", game.id])
    removeProc.running = true
  }

  function copyExec(exec) {
    Quickshell.execDetached(["bash", "-c", "printf '%s' \"$1\" | wl-copy", "bash", String(exec || "")])
  }

  Component.onCompleted: {
    ogmProbe.running = true
    scaleProc.running = true
  }

  // ---- fullscreen overlay window ----
  PanelWindow {
    id: win
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "ogm"
    WlrLayershell.layer: WlrLayer.Overlay
    // Exclusive so key events and Ctrl modifier state actually reach us
    // (type-to-search, PgUp/PgDn, Ctrl+wheel zoom) and so the compositor's
    // killactive (Super+W) targets this surface.
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive

    // Compositor-side close (killactive sends layer-shell `closed`, which
    // tears down the backing window): quit instead of lingering headless.
    // Debounced — the visibility flag flickers during surface setup, only a
    // persistent disappearance means the compositor really closed us.
    onBackingWindowVisibleChanged: {
      if (backingWindowVisible) compositorCloseTimer.stop()
      else compositorCloseTimer.restart()
    }

    Timer {
      id: compositorCloseTimer
      interval: 500
      onTriggered: if (!win.backingWindowVisible) Qt.quit()
    }

    onScreenChanged: scaleProc.running = true

    Rectangle {
      id: backdrop
      anchors.fill: parent
      color: theme.overlayBg
      focus: true

      Shortcut {
        sequence: "Escape"
        onActivated: addDialog.isOpen ? addDialog.close() : Qt.quit()
      }
      Shortcut {
        sequence: "Ctrl+Q"
        onActivated: Qt.quit()
      }

      Keys.onPressed: function(event) { root.handleKey(event) }

      // Dimmed backdrop: clicks that miss the content close the overlay.
      MouseArea { anchors.fill: parent; onClicked: Qt.quit() }

      ColumnLayout {
        anchors.fill: parent
        anchors.margins: theme.u(32)
        spacing: theme.u(16)

        TopBar {
          id: topBar
          Layout.fillWidth: true
          theme: theme
          prefs: root.prefs
          categories: root.allCategories
          spinning: root.refreshing
          onQueryEdited: function(q) { root.updatePrefs({ query: q }) }
          onCategoryToggled: function(c) { root.toggleCategory(c) }
          onSortRequested: function(k) { root.requestSort(k) }
          onZoomPresetRequested: function(s) { root.updatePrefs({ cover_scale: s }) }
          onAddRequested: addDialog.open()
        }

        GameGrid {
          id: gameGrid
          Layout.fillWidth: true
          Layout.fillHeight: true
          theme: theme
          sections: root.visibleSections
          visibleCount: root.visibleCount
          totalGames: root.state && root.state.games ? root.state.games.length : 0
          onZoomRequested: function(d) { root.zoomBy(d) }
          onPlayRequested: function(g) { root.launchGame(g) }
          onRemoveRequested: function(g) { root.removeGame(g) }
          onCopyExecRequested: function(e) { root.copyExec(e) }
        }
      }

      AddGameDialog {
        id: addDialog
        anchors.fill: parent
        theme: theme
        categories: root.allCategories
        onSaveRequested: function(args) { root.addGame(args) }
      }
    }
  }
}
