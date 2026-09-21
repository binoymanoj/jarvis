import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

BarWidget {
  id: root
  moduleName: "jarvis"

  readonly property string runtimeDir: Quickshell.env("XDG_RUNTIME_DIR") || "/run/user/1000"
  readonly property string statusPath: runtimeDir + "/jarvis-status.json"

  property var statusData: ({
    active: false,
    daemon_running: false,
    state: "idle",
    mic_active: false,
    wakeword_enabled: true,
    cli_tool: "agy",
    last_transcript: "",
    last_reply: "",
    is_busy: false,
    current_task: ""
  })

  readonly property bool micActive: statusData && statusData.mic_active === true
  readonly property string currentState: statusData ? (statusData.state || "idle") : "idle"
  readonly property bool wakewordEnabled: statusData ? (statusData.wakeword_enabled !== false) : true
  readonly property string cliTool: statusData ? (statusData.cli_tool || "agy") : "agy"
  readonly property color barForeground: bar ? bar.barForeground : Color.foreground
  readonly property bool isBusy: statusData ? (statusData.is_busy === true || currentState === "processing") : false
  readonly property string currentTask: statusData ? (statusData.current_task || "") : ""

  property bool previewOpen: false
  property bool popoutSwitchClosing: false

  readonly property bool opened: previewOpen

  function open() {
    previewOpen = true
  }

  function close() {
    previewOpen = false
  }

  function toggle() {
    opened ? close() : open()
  }

  function closeForPopoutSwitch() {
    popoutSwitchClosing = true
    close()
    Qt.callLater(function() { root.popoutSwitchClosing = false })
  }

  FileView {
    id: watcher
    path: root.statusPath
    watchChanges: true
    printErrors: false
    onFileChanged: reload()
    onLoaded: {
      try {
        var parsed = JSON.parse(text())
        if (parsed && typeof parsed === "object") {
          root.statusData = parsed
        }
      } catch (e) {
      }
    }
  }

  Timer {
    interval: 800
    running: true
    repeat: true
    onTriggered: watcher.reload()
  }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    tooltipText: root.micActive ? "Jarvis: Listening (Mic On)" :
                 (root.isBusy ? ("Jarvis: Working" + (root.currentTask ? (" - " + root.currentTask) : "...")) :
                 (root.currentState === "speaking" ? "Jarvis: Speaking..." : "Jarvis: Ready"))

    onPressed: function(buttonCode) {
      if (buttonCode === Qt.RightButton) {
        if (root.bar) root.bar.run("jarvis -t")
      } else {
        root.previewOpen = !root.previewOpen
      }
    }

    iconComponent: Component {
      Item {
        anchors.fill: parent

        Text {
          id: botIcon
          anchors.centerIn: parent
          text: "󰚩"
          font.family: Style.font.family
          font.pixelSize: Style.bar.iconFont
          color: root.micActive ? Color.accent :
                 (root.isBusy ? Color.accent :
                 (root.currentState !== "idle" ? Color.accent : root.barForeground))

          SequentialAnimation on scale {
            running: root.isBusy && !root.micActive
            loops: Animation.Infinite
            NumberAnimation { from: 1.0; to: 1.15; duration: 450; easing.type: Easing.InOutSine }
            NumberAnimation { from: 1.15; to: 1.0; duration: 450; easing.type: Easing.InOutSine }
          }
        }

        // Active Working / Background Task Spinner Indicator
        Item {
          id: busyIndicator
          visible: root.isBusy && !root.micActive
          anchors.top: parent.top
          anchors.topMargin: -Style.space(2)
          anchors.right: parent.right
          anchors.rightMargin: -Style.space(3)
          width: Style.space(10)
          height: Style.space(10)

          Text {
            id: spinnerGlyph
            anchors.centerIn: parent
            text: "󰑐"
            font.family: Style.font.family
            font.pixelSize: Style.space(9)
            color: Color.accent

            RotationAnimation on rotation {
              running: busyIndicator.visible
              loops: Animation.Infinite
              from: 0
              to: 360
              duration: 800
            }
          }
        }

        // Microphone Active Indicator Dot
        Rectangle {
          id: micDot
          visible: root.micActive
          width: Style.space(6)
          height: Style.space(6)
          radius: width / 2
          color: "#ef4444"
          border.width: 1
          border.color: Qt.rgba(1.0, 1.0, 1.0, 0.8)
          anchors.top: parent.top
          anchors.topMargin: -Style.space(1)
          anchors.right: parent.right
          anchors.rightMargin: -Style.space(2)

          SequentialAnimation on opacity {
            running: root.micActive
            loops: Animation.Infinite
            NumberAnimation { from: 1.0; to: 0.35; duration: 500; easing.type: Easing.InOutQuad }
            NumberAnimation { from: 0.35; to: 1.0; duration: 500; easing.type: Easing.InOutQuad }
          }
        }
      }
    }
  }

  KeyboardPanel {
    id: previewPanel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.previewOpen
    focusTarget: keyCatcher
    contentWidth: previewPanel.fittedContentWidth(Style.space(350))
    contentHeight: previewPanel.fittedContentHeight(contentCol.implicitHeight + Style.space(24), Style.space(560))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onCloseRequested: root.close()

      Column {
        id: contentCol
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: Style.space(4)
        spacing: Style.space(10)

      // 1. Header Hero Card
      Item {
        width: parent.width
        implicitHeight: Style.space(42)

        Rectangle {
          id: heroIconBox
          width: Style.space(38)
          height: Style.space(38)
          radius: Style.cornerRadius > 0 ? Style.cornerRadius : Style.space(6)
          color: Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.15)
          border.width: 1
          border.color: Color.accent
          anchors.left: parent.left
          anchors.verticalCenter: parent.verticalCenter

          Text {
            anchors.centerIn: parent
            text: "󰚩"
            font.family: Style.font.family
            font.pixelSize: Style.font.display
            color: Color.accent
          }
        }

        Column {
          anchors.left: heroIconBox.right
          anchors.leftMargin: Style.space(10)
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.space(2)

          Text {
            text: "Jarvis Assistant"
            font.family: Style.font.family
            font.pixelSize: Style.font.title
            font.bold: true
            color: Color.popups.text
          }

          Row {
            spacing: Style.space(6)

            Rectangle {
              width: Style.space(6)
              height: Style.space(6)
              radius: width / 2
              anchors.verticalCenter: parent.verticalCenter
              color: root.micActive ? "#ef4444" :
                     (root.isBusy ? Color.accent :
                     (root.currentState === "speaking" ? Color.accent : "#22c55e"))
            }

            Text {
              text: root.micActive ? "Mic Active (Listening)" :
                    (root.isBusy ? ("Working: " + (root.currentTask || "Processing...")) :
                    (root.currentState === "speaking" ? "Speaking..." :
                    (root.wakewordEnabled ? "Ready (Wake word active)" : "Standby (Idle)")))
              font.family: Style.font.family
              font.pixelSize: Style.font.caption
              color: Qt.darker(Color.popups.text, 1.3)
              elide: Text.ElideRight
              width: parent.parent.width - Style.space(30)
            }
          }
        }
      }

      PanelSeparator { foreground: Color.popups.text }

      // 2. Action Buttons (Talk/Listen & Silence)
      Row {
        width: parent.width
        spacing: Style.space(8)

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: root.micActive ? "Stop Mic" : "Talk / Listen"
          iconText: root.micActive ? "󰍭" : "󰍬"
          selected: root.micActive
          onClicked: {
            if (root.bar) root.bar.run("jarvis -t")
            root.close()
          }
        }

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: "Silence Audio"
          iconText: "󰚌"
          onClicked: {
            if (root.bar) root.bar.run("jarvis -k")
            root.close()
          }
        }
      }

      PanelSeparator { foreground: Color.popups.text }

      // 3. Settings: Wake Word Toggle
      PanelSectionHeader {
        text: "VOICE SETTINGS"
        foreground: Color.popups.text
      }

      Item {
        width: parent.width
        implicitHeight: Style.space(34)

        Column {
          anchors.left: parent.left
          anchors.right: wakeToggle.left
          anchors.rightMargin: Style.space(10)
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.space(1)

          Text {
            text: "Wake Word (\"Hey Jarvis\")"
            font.family: Style.font.family
            font.pixelSize: Style.font.body
            color: Color.popups.text
          }

          Text {
            text: root.wakewordEnabled ? "Continuous hands-free detection" : "Wake word detection disabled"
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            color: Qt.darker(Color.popups.text, 1.4)
          }
        }

        ToggleSwitch {
          id: wakeToggle
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          checked: root.wakewordEnabled
          onToggled: {
            if (root.bar) root.bar.run("jarvis --wakeword-toggle")
          }
        }
      }

      PanelSeparator { foreground: Color.popups.text }

      // 4. Quick Actions
      PanelSectionHeader {
        text: "QUICK ACTIONS"
        foreground: Color.popups.text
      }

      Grid {
        width: parent.width
        columns: 2
        spacing: Style.space(8)

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: "Dev Setup"
          iconText: "󰅩"
          onClicked: {
            if (root.bar) root.bar.run("jarvis -c \"open dev workflow\" --no-speech")
            root.close()
          }
        }

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: "Screen Vision"
          iconText: "󰹑"
          onClicked: {
            if (root.bar) root.bar.run("jarvis -c \"look at my screen and tell me what is visible\"")
            root.close()
          }
        }

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: "Take Note"
          iconText: "󰈙"
          onClicked: {
            if (root.bar) root.bar.run("jarvis -c \"open writing workflow\" --no-speech")
            root.close()
          }
        }

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: "Media Hub"
          iconText: "󰝚"
          onClicked: {
            if (root.bar) root.bar.run("jarvis -c \"open media workflow\" --no-speech")
            root.close()
          }
        }
      }

      PanelSeparator { foreground: Color.popups.text }

      // 5. System Management & Quit Jarvis (User requirement)
      PanelSectionHeader {
        text: "SYSTEM CONTROL"
        foreground: Color.popups.text
      }

      Row {
        width: parent.width
        spacing: Style.space(8)

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: "Restart"
          iconText: "󰑐"
          onClicked: {
            if (root.bar) root.bar.run("jarvis --restart")
            root.close()
          }
        }

        Button {
          width: Math.floor((parent.width - Style.space(8)) / 2)
          text: "Quit Jarvis"
          iconText: "󰗼"
          accent: "#ef4444"
          onClicked: {
            if (root.bar) root.bar.run("jarvis --quit")
            root.close()
          }
        }
      }

      PanelSeparator { foreground: Color.popups.text }

      // 6. Footer
      Item {
        width: parent.width
        implicitHeight: Style.space(16)

        Text {
          anchors.left: parent.left
          anchors.verticalCenter: parent.verticalCenter
          text: "Engine: " + root.cliTool.toUpperCase()
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          color: Qt.darker(Color.popups.text, 1.5)
        }

        Text {
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          text: "PTT: Super+C"
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          color: Qt.darker(Color.popups.text, 1.5)
        }
      }
    }
    }
  }
}
