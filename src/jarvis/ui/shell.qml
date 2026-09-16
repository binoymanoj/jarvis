import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

ShellRoot {
  id: shell

  // Execution state: "idle", "listening", "thinking", "speaking"
  property string currentState: "idle"
  property real currentVolume: 0.0
  property string currentText: ""
  property bool isVisible: false

  // Dynamics & Animation phases
  property real wavePhase: 0.0
  property real telemetryPhase: 0.0
  property real displayAmp: 0.0
  property real displaySpeed: 0.0
  property real targetAmp: 0.0
  property real targetSpeed: 0.0

  // -------------------------------------------------------------
  // Omarchy Theme Synchronization
  // -------------------------------------------------------------
  readonly property string homeDir: Quickshell.env("HOME")
  readonly property string currentThemePath: homeDir + "/.local/state/omarchy/current/theme"

  property string themeName: "Solitude"
  property color themeAccent: "#798186"
  property color themeForeground: "#cacccc"
  property color themeBackground: "#101315"
  property color themeDarkBackground: "#0c0e10"
  property color themeMuted: "#4b4e55"
  property color themeSelection: "#343d41"
  property color themeActiveBorder: "#a8adb0"
  property color themeCyan: "#707070"
  property color themeMagenta: "#aeaeae"
  property color themeBlue: "#798186"
  property string themeMode: "dark"

  // Active state-driven glow & highlight color
  readonly property color activeStateColor: {
    if (currentState === "thinking") return themeMagenta;
    if (currentState === "speaking") return (themeCyan !== "#707070" ? themeCyan : themeAccent);
    return themeAccent;
  }

  function hexToRgba(colorVal, alpha) {
    if (!colorVal) return "rgba(121, 129, 134, " + alpha + ")";
    var c = Qt.color(colorVal);
    return "rgba(" + Math.round(c.r * 255) + ", " + Math.round(c.g * 255) + ", " + Math.round(c.b * 255) + ", " + alpha + ")";
  }

  function parseToml(text) {
    var map = {};
    var lines = String(text || "").split("\n");
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].replace(/^\s+|\s+$/g, "");
      if (!line || line.charAt(0) === "#" || line.charAt(0) === "[") continue;
      var parts = line.split("=");
      if (parts.length >= 2) {
        var k = parts[0].trim();
        var rawVal = parts[1].trim();
        if (rawVal.charAt(0) === '"' || rawVal.charAt(0) === "'") {
          var q = rawVal.charAt(0);
          var endQ = rawVal.indexOf(q, 1);
          map[k] = (endQ !== -1) ? rawVal.substring(1, endQ) : rawVal.replace(/['"]/g, "");
        } else {
          map[k] = rawVal.split(/\s+/)[0].trim();
        }
      }
    }
    return map;
  }

  function applyThemeColors(raw) {
    if (!raw) return;
    var toml = parseToml(raw);
    if (toml.accent) themeAccent = toml.accent;
    if (toml.foreground) themeForeground = toml.foreground;
    if (toml.background) themeBackground = toml.background;
    if (toml.dark_background) themeDarkBackground = toml.dark_background;
    else if (toml.darker_background) themeDarkBackground = toml.darker_background;
    if (toml.muted) themeMuted = toml.muted;
    if (toml.selection) themeSelection = toml.selection;
    if (toml.active_border_color) themeActiveBorder = toml.active_border_color;
    if (toml.bright_cyan) themeCyan = toml.bright_cyan;
    else if (toml.cyan) themeCyan = toml.cyan;
    if (toml.bright_magenta) themeMagenta = toml.bright_magenta;
    else if (toml.magenta) themeMagenta = toml.magenta;
    if (toml.bright_blue) themeBlue = toml.bright_blue;
    else if (toml.blue) themeBlue = toml.blue;
    if (toml.mode) themeMode = toml.mode;
    canvas.requestPaint();
  }

  FileView {
    id: themeColorsFile
    path: shell.currentThemePath + "/colors.toml"
    watchChanges: true
    printErrors: false
    onLoaded: shell.applyThemeColors(text())
    onFileChanged: reload()
  }

  FileView {
    id: themeNameFile
    path: shell.homeDir + "/.local/state/omarchy/current/theme.name"
    watchChanges: true
    printErrors: false
    onLoaded: {
      shell.themeName = text().trim();
      themeColorsFile.reload();
    }
    onFileChanged: reload()
  }

  // -------------------------------------------------------------
  // Animation Dynamics
  // -------------------------------------------------------------
  function updateTargets() {
    if (currentState === "listening") {
      if (currentVolume > 0.02) {
        // Voice active: smoothly scale amplitude and wave frequency
        targetAmp = Math.min(16.0, 3.0 + currentVolume * 22.0);
        targetSpeed = 0.08 + currentVolume * 0.12;
      } else {
        // Voice paused: settle to calm glowing horizon line
        targetAmp = 0.0;
        targetSpeed = 0.0;
      }
    } else if (currentState === "thinking") {
      targetAmp = 4.5;
      targetSpeed = 0.14;
    } else if (currentState === "speaking") {
      targetAmp = 6.5;
      targetSpeed = 0.09;
    } else {
      targetAmp = 0.0;
      targetSpeed = 0.0;
    }
  }

  onCurrentVolumeChanged: updateTargets()
  onCurrentStateChanged: updateTargets()

  // 60 FPS animation timer: drives wave smoothing, telemetry rotation, and visualizer bars
  Timer {
    interval: 16
    running: shell.isVisible
    repeat: true
    onTriggered: {
      shell.displayAmp += (shell.targetAmp - shell.displayAmp) * 0.22;
      shell.displaySpeed += (shell.targetSpeed - shell.displaySpeed) * 0.20;

      if (shell.displaySpeed > 0.001) {
        shell.wavePhase = (shell.wavePhase + shell.displaySpeed) % 62.8318;
      }

      var telemSpeed = (shell.currentState === "thinking") ? 0.045 : 0.009;
      shell.telemetryPhase = (shell.telemetryPhase + telemSpeed) % 62.8318;

      canvas.requestPaint();
    }
  }

  // -------------------------------------------------------------
  // IPC Interface
  // -------------------------------------------------------------
  IpcHandler {
    target: "jarvis"

    function show(): string {
      shell.isVisible = true;
      shell.currentState = "listening";
      shell.currentText = "Listening...";
      shell.updateTargets();
      canvas.requestPaint();
      return "ok";
    }

    function hide(): string {
      shell.currentState = "idle";
      shell.isVisible = false;
      shell.currentVolume = 0.0;
      shell.targetAmp = 0.0;
      shell.targetSpeed = 0.0;
      shell.displayAmp = 0.0;
      shell.displaySpeed = 0.0;
      shell.currentText = "";
      return "ok";
    }

    function setListening(text: string): string {
      shell.isVisible = true;
      shell.currentState = "listening";
      shell.currentText = text ? text : "Listening...";
      shell.updateTargets();
      canvas.requestPaint();
      return "ok";
    }

    function setThinking(text: string): string {
      shell.isVisible = true;
      shell.currentState = "thinking";
      shell.currentText = text ? text : "Thinking...";
      shell.updateTargets();
      canvas.requestPaint();
      return "ok";
    }

    function setSpeaking(text: string): string {
      shell.isVisible = true;
      shell.currentState = "speaking";
      shell.currentText = text ? text : "";
      shell.updateTargets();
      canvas.requestPaint();
      return "ok";
    }

    function setVolume(v: real): string {
      shell.currentVolume = Math.max(0.0, Math.min(1.0, v));
      shell.updateTargets();
      return "ok";
    }

    function setTheme(accent: string, fg: string, bg: string, darkBg: string, cyan: string, magenta: string, blue: string, name: string): string {
      if (accent) shell.themeAccent = accent;
      if (fg) shell.themeForeground = fg;
      if (bg) shell.themeBackground = bg;
      if (darkBg) shell.themeDarkBackground = darkBg;
      if (cyan) shell.themeCyan = cyan;
      if (magenta) shell.themeMagenta = magenta;
      if (blue) shell.themeBlue = blue;
      if (name) shell.themeName = name;
      canvas.requestPaint();
      return "ok";
    }

    function ping(): string {
      return "pong";
    }
  }

  // -------------------------------------------------------------
  // Floating Layer-Shell Window (Sleek Omarchy Glass Capsule)
  // -------------------------------------------------------------
  PanelWindow {
    id: hudWindow
    visible: shell.isVisible

    anchors {
      bottom: true
    }
    margins {
      bottom: 32
    }

    implicitWidth: 640
    implicitHeight: 80
    color: "transparent"

    WlrLayershell.namespace: "jarvis-hud"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore

    // Centered hardware glass capsule container
    Rectangle {
      id: capsule
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.bottom: parent.bottom

      // Responsive width adapting to content
      width: Math.min(620, Math.max(340, contentRow.implicitWidth + 36))
      height: 64
      radius: 18

      // Deep dark translucent glass matching Omarchy theme
      color: shell.hexToRgba(shell.themeDarkBackground, 0.92)
      border.color: shell.hexToRgba(shell.activeStateColor, 0.40)
      border.width: 1

      // Smooth entry & exit motion
      opacity: shell.isVisible ? 1.0 : 0.0
      y: shell.isVisible ? 0 : 12

      Behavior on opacity {
        NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
      }
      Behavior on y {
        NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
      }
      Behavior on width {
        NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
      }
      Behavior on border.color {
        ColorAnimation { duration: 200 }
      }

      // 1. Subtle Specular Highlight Rim (Inner top glass reflection)
      Rectangle {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: 18
        height: 1
        color: "transparent"

        Rectangle {
          anchors.fill: parent
          gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: "transparent" }
            GradientStop { position: 0.5; color: Qt.rgba(1.0, 1.0, 1.0, 0.14) }
            GradientStop { position: 1.0; color: "transparent" }
          }
        }
      }

      // 2. Main Capsule Layout (AI Orb + Divider + Information Module)
      Row {
        id: contentRow
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: 12
        spacing: 12

        // A. FUTURISTIC AI RESONATOR ORB (48x48 px)
        Item {
          id: orbItem
          width: 48
          height: 48
          anchors.verticalCenter: parent.verticalCenter

          Canvas {
            id: canvas
            anchors.fill: parent

            onPaint: {
              var ctx = getContext("2d");
              var w = width;
              var h = height;
              var cx = w / 2;
              var cy = h / 2;

              ctx.clearRect(0, 0, w, h);

              var state = shell.currentState;
              var amp = shell.displayAmp;
              var phase = shell.wavePhase;
              var rot = shell.telemetryPhase;
              var glowColor = shell.activeStateColor;

              // -------------------------------------------------------
              // 1. OUTER PRECISION TELEMETRY RETICLE
              // -------------------------------------------------------
              ctx.save();
              ctx.translate(cx, cy);
              ctx.rotate(rot);

              // 4 cardinal calibration ticks
              ctx.strokeStyle = shell.hexToRgba(glowColor, 0.40);
              ctx.lineWidth = 1.0;
              for (var t = 0; t < 4; t++) {
                var ta = t * (Math.PI / 2);
                ctx.beginPath();
                ctx.moveTo(Math.cos(ta) * 20.5, Math.sin(ta) * 20.5);
                ctx.lineTo(Math.cos(ta) * 23.0, Math.sin(ta) * 23.0);
                ctx.stroke();
              }

              // Subtle dashed orbital arcs
              ctx.strokeStyle = shell.hexToRgba(shell.themeForeground, 0.16);
              ctx.lineWidth = 0.8;
              for (var a = 0; a < 4; a++) {
                var sa = a * (Math.PI / 2) + 0.28;
                var ea = sa + (Math.PI / 2) - 0.56;
                ctx.beginPath();
                ctx.arc(0, 0, 21.8, sa, ea);
                ctx.stroke();
              }
              ctx.restore();

              // -------------------------------------------------------
              // 2. RESONATOR AURA (Breathes with audio energy)
              // -------------------------------------------------------
              var auraR = 19.0 + (amp * 0.14);
              ctx.beginPath();
              ctx.arc(cx, cy, auraR, 0, Math.PI * 2);
              ctx.strokeStyle = shell.hexToRgba(glowColor, 0.18 + (amp / 16.0) * 0.40);
              ctx.lineWidth = 1.0 + (amp / 16.0) * 0.5;
              ctx.stroke();

              // -------------------------------------------------------
              // 3. CORE OBSIDIAN GLASS LENS (Radius = 17.5px)
              // -------------------------------------------------------
              var rLens = 17.5;

              // Outer rim stroke
              ctx.beginPath();
              ctx.arc(cx, cy, rLens, 0, Math.PI * 2);
              ctx.strokeStyle = shell.hexToRgba(glowColor, 0.70);
              ctx.lineWidth = 1.2;
              ctx.stroke();

              // Clip inside the lens
              ctx.save();
              ctx.beginPath();
              ctx.arc(cx, cy, rLens - 0.6, 0, Math.PI * 2);
              ctx.clip();

              // Lens background fill
              ctx.fillStyle = shell.hexToRgba(shell.themeDarkBackground, 0.95);
              ctx.fill();

              // -------------------------------------------------------
              // 4. FLUID SINUSOIDAL WAVEFORMS
              // -------------------------------------------------------
              ctx.globalCompositeOperation = "lighter";

              var waves = [
                { color: shell.hexToRgba(glowColor, 0.85), freq: 0.09, speed: 1.0, amp: 1.0, offset: 0.0 },
                { color: shell.hexToRgba(shell.themeCyan, 0.80), freq: 0.13, speed: 1.35, amp: 0.80, offset: 1.3 },
                { color: shell.hexToRgba(shell.themeMagenta, 0.70), freq: 0.08, speed: 0.75, amp: 0.85, offset: 2.5 }
              ];

              if (amp <= 0.08) {
                // Silent: razor-sharp glowing horizon line
                for (var k = 0; k < waves.length; k++) {
                  ctx.beginPath();
                  ctx.strokeStyle = waves[k].color;
                  ctx.lineWidth = 1.4;
                  ctx.moveTo(cx - rLens * 0.80, cy);
                  ctx.lineTo(cx + rLens * 0.80, cy);
                  ctx.stroke();
                }
              } else {
                // Speaking / Active: fluid sinusoidal undulation
                for (var i = 0; i < waves.length; i++) {
                  var wave = waves[i];
                  ctx.beginPath();
                  ctx.strokeStyle = wave.color;
                  ctx.lineWidth = 1.6;

                  var startX = cx - rLens;
                  var endX = cx + rLens;

                  for (var x = startX; x <= endX; x += 1.5) {
                    var normX = (x - cx) / rLens;
                    var env = Math.max(0.0, 1.0 - (normX * normX));
                    var yOffset = Math.sin((x - startX) * wave.freq + (phase * wave.speed) + wave.offset)
                                * (amp * wave.amp * env);
                    var y = cy + yOffset;

                    if (x === startX) {
                      ctx.moveTo(x, y);
                    } else {
                      ctx.lineTo(x, y);
                    }
                  }
                  ctx.stroke();
                }
              }

              // -------------------------------------------------------
              // 5. QUANTUM AI SINGULARITY CORE
              // -------------------------------------------------------
              if (state === "thinking") {
                // Pulsing central orb with 2 orbiting micro-sparks
                var pulseR = 3.2 + Math.sin(phase * 3.5) * 1.2;
                ctx.beginPath();
                ctx.arc(cx, cy, pulseR, 0, Math.PI * 2);
                ctx.fillStyle = "rgba(255, 255, 255, 0.95)";
                ctx.fill();

                var satDist = 7.5;
                var satA1 = phase * 2.8;
                var satA2 = satA1 + Math.PI;

                ctx.beginPath();
                ctx.arc(cx + Math.cos(satA1) * satDist, cy + Math.sin(satA1) * satDist, 1.3, 0, Math.PI * 2);
                ctx.arc(cx + Math.cos(satA2) * satDist, cy + Math.sin(satA2) * satDist, 1.3, 0, Math.PI * 2);
                ctx.fillStyle = shell.hexToRgba(shell.themeCyan, 0.92);
                ctx.fill();

              } else if (state === "speaking") {
                // Gentle rhythmic breathing rhythm
                var spkR = 2.6 + Math.sin(phase * 2.0) * 0.9;
                ctx.beginPath();
                ctx.arc(cx, cy, spkR, 0, Math.PI * 2);
                ctx.fillStyle = "rgba(255, 255, 255, 0.92)";
                ctx.fill();

              } else {
                // Focused listening photon core
                var pR = 2.0 + (amp * 0.08);
                ctx.beginPath();
                ctx.arc(cx, cy, pR, 0, Math.PI * 2);
                ctx.fillStyle = "#ffffff";
                ctx.fill();

                ctx.beginPath();
                ctx.arc(cx, cy, pR + 1.5, 0, Math.PI * 2);
                ctx.fillStyle = shell.hexToRgba(glowColor, 0.35);
                ctx.fill();
              }

              ctx.restore();
            }
          }
        }

        // B. ELEGANT VERTICAL HAIRLINE DIVIDER
        Rectangle {
          width: 1
          height: 32
          anchors.verticalCenter: parent.verticalCenter
          color: shell.hexToRgba(shell.themeForeground, 0.12)
          radius: 1
        }

        // C. INFORMATION & TELEMETRY MODULE
        Column {
          anchors.verticalCenter: parent.verticalCenter
          spacing: 3
          width: Math.min(520, Math.max(220, mainText.paintedWidth + 8))

          // Top Header Row (Status Pill + Audio Level Bars + Telemetry)
          Row {
            spacing: 8
            anchors.left: parent.left
            anchors.right: parent.right

            // 1. Status Indicator Pill Badge
            Rectangle {
              height: 18
              width: statusRow.implicitWidth + 12
              radius: 9
              color: shell.hexToRgba(shell.activeStateColor, 0.14)
              border.color: shell.hexToRgba(shell.activeStateColor, 0.30)
              border.width: 1

              Row {
                id: statusRow
                anchors.centerIn: parent
                spacing: 5

                // Glowing Status Dot
                Rectangle {
                  width: 6
                  height: 6
                  radius: 3
                  color: shell.activeStateColor
                  anchors.verticalCenter: parent.verticalCenter

                  // Breathing animation
                  SequentialAnimation on opacity {
                    loops: Animation.Infinite
                    running: shell.isVisible
                    NumberAnimation { from: 0.6; to: 1.0; duration: 600; easing.type: Easing.InOutQuad }
                    NumberAnimation { from: 1.0; to: 0.6; duration: 600; easing.type: Easing.InOutQuad }
                  }
                }

                // Status Label
                Text {
                  text: (shell.currentState === "thinking")
                    ? "REASONING"
                    : (shell.currentState === "speaking" ? "JARVIS" : "LISTENING")
                  font.pixelSize: 9
                  font.weight: Font.Bold
                  font.family: "JetBrainsMono Nerd Font, monospace"
                  font.letterSpacing: 1.0
                  color: shell.activeStateColor
                  anchors.verticalCenter: parent.verticalCenter
                }
              }
            }

            // 2. Real-time Audio Level Equalizer Bars (4 bouncing bars)
            Row {
              anchors.verticalCenter: parent.verticalCenter
              spacing: 2
              visible: shell.currentState === "listening" || shell.currentState === "speaking"

              Rectangle {
                width: 2
                height: Math.max(3, Math.min(13, 3 + shell.displayAmp * 0.45))
                radius: 1
                color: shell.hexToRgba(shell.activeStateColor, 0.80)
                anchors.verticalCenter: parent.verticalCenter
              }
              Rectangle {
                width: 2
                height: Math.max(3, Math.min(14, 4 + shell.displayAmp * 0.70))
                radius: 1
                color: shell.hexToRgba(shell.activeStateColor, 0.90)
                anchors.verticalCenter: parent.verticalCenter
              }
              Rectangle {
                width: 2
                height: Math.max(3, Math.min(13, 3 + shell.displayAmp * 0.55))
                radius: 1
                color: shell.hexToRgba(shell.activeStateColor, 0.80)
                anchors.verticalCenter: parent.verticalCenter
              }
              Rectangle {
                width: 2
                height: Math.max(3, Math.min(11, 3 + shell.displayAmp * 0.35))
                radius: 1
                color: shell.hexToRgba(shell.activeStateColor, 0.65)
                anchors.verticalCenter: parent.verticalCenter
              }
            }

            // 3. Omarchy Theme & Model Telemetry
            Text {
              text: "// " + shell.themeName.toUpperCase() + " • GEMINI"
              font.pixelSize: 9
              font.weight: Font.Medium
              font.family: "JetBrainsMono Nerd Font, monospace"
              font.letterSpacing: 0.8
              color: shell.hexToRgba(shell.themeForeground, 0.36)
              anchors.verticalCenter: parent.verticalCenter
            }
          }

          // Bottom Query / Full Response Text
          Text {
            id: mainText
            text: shell.currentText ? shell.currentText : "Ready for your command, sir."
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.family: "Inter, Liberation Sans, Roboto, sans-serif"
            color: shell.themeForeground
            horizontalAlignment: Text.AlignLeft
            width: parent.width
            elide: Text.ElideRight
            maximumLineCount: 2
            wrapMode: Text.WrapAtWordBoundaryOrAnywhere

            Behavior on opacity {
              NumberAnimation { duration: 120 }
            }
          }
        }
      }
    }
  }
}
