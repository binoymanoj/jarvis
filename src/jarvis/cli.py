import argparse
import asyncio
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import sounddevice as sd
from jarvis.audio.recorder import AudioRecorder
from jarvis.audio.stt import SpeechToText
from jarvis.audio.tts import TextToSpeech
from jarvis.audio.wakeword import WakeWordDetector
from jarvis.core.agent import JarvisAgent, is_exit_command
from jarvis.core.config import settings
from jarvis.core.logger import console, log
from jarvis.core.state import (
    DAEMON_PID_FILE,
    PID_FILE,
    read_status,
    set_idle,
    set_processing,
    set_recording,
    set_speaking,
    update_status,
)
from jarvis.ui.hud import JarvisHUD

STATE_FILE = Path(f"/tmp/jarvis-{os.getuid()}.state")
TIME_FILE = Path(f"/tmp/jarvis-{os.getuid()}.time")


def is_process_alive(pid: int) -> bool:
    """Check if a process with given PID exists."""
    try:
        os.kill(pid, 0)
        return True
    except (OSError, ProcessLookupError):
        return False


def get_current_state() -> str:
    """Read the current execution phase of the active Jarvis process."""
    try:
        if STATE_FILE.exists():
            return STATE_FILE.read_text().strip()
    except Exception:
        pass
    return "idle"


def set_current_state(state: str) -> None:
    """Persist the current execution phase for push-to-talk coordination."""
    try:
        STATE_FILE.write_text(state)
    except Exception:
        pass


def stop_running_session() -> bool:
    """Silence audio, interrupt active session, and hide HUD without killing the daemon."""
    killed = False

    # 1. If daemon is running and in an active conversational turn, signal SIGHUP to cancel it
    st = read_status()
    is_turn_active = st.get("active", False) or st.get("state") in ("recording", "processing", "speaking") or st.get("mic_active", False)
    if is_turn_active and DAEMON_PID_FILE.exists():
        try:
            d_pid = int(DAEMON_PID_FILE.read_text().strip())
            if is_process_alive(d_pid) and d_pid != os.getpid():
                os.kill(d_pid, signal.SIGHUP)
                killed = True
        except Exception:
            pass

    # 2. If a standalone session PID exists, terminate that standalone process
    if PID_FILE.exists():
        try:
            pid = int(PID_FILE.read_text().strip())
            if is_process_alive(pid) and pid != os.getpid():
                d_pid = None
                if DAEMON_PID_FILE.exists():
                    try:
                        d_pid = int(DAEMON_PID_FILE.read_text().strip())
                    except Exception:
                        pass
                if pid != d_pid:
                    os.kill(pid, signal.SIGTERM)
                    time.sleep(0.08)
                    if is_process_alive(pid):
                        os.kill(pid, signal.SIGKILL)
                    killed = True
        except Exception:
            pass
        finally:
            for f in (PID_FILE, STATE_FILE, TIME_FILE):
                try:
                    if f.exists():
                        f.unlink()
                except Exception:
                    pass

    # 3. Silence any active PipeWire sounddevice output
    try:
        sd.stop()
    except Exception:
        pass

    # 4. Hide Quickshell HUD immediately via IPC
    try:
        hud = JarvisHUD()
        asyncio.run(hud.hide())
    except Exception:
        pass

    # 5. Reset mic and conversational state to idle or wakeword
    daemon_alive = False
    if DAEMON_PID_FILE.exists():
        try:
            d_pid = int(DAEMON_PID_FILE.read_text().strip())
            daemon_alive = is_process_alive(d_pid)
        except Exception:
            pass
    set_idle(wakeword_active=daemon_alive)

    return killed


async def run_command_headless(prompt: str, speak_reply: bool = True) -> None:
    """Execute a text command through the Jarvis agent."""
    agent = JarvisAgent()
    reply = await agent.process_prompt(prompt)
    console.print(f"[bold cyan]󰚩 Jarvis:[/bold cyan] {reply}")

    if speak_reply:
        tts = TextToSpeech()
        await tts.speak(reply)


async def run_voice_activation(
    speak_reply: bool = True,
    is_daemon: bool = False,
    cancel_event: Optional[asyncio.Event] = None,
    recorder_holder: Optional[list] = None,
    tts_holder: Optional[list] = None,
) -> None:
    """Run full continuous multi-turn voice interaction cycle with transparent wave HUD."""
    hud = JarvisHUD()
    recorder = AudioRecorder()
    stt = SpeechToText()
    agent = JarvisAgent()
    tts = TextToSpeech() if speak_reply else None

    if recorder_holder is not None:
        recorder_holder.append(recorder)
    if tts_holder is not None and tts:
        tts_holder.append(tts)

    loop = asyncio.get_running_loop()

    if not is_daemon:
        PID_FILE.write_text(str(os.getpid()))
        TIME_FILE.write_text(str(time.monotonic()))
        set_current_state("recording")

        def handle_interrupt(*_):
            log.info("[yellow]Interrupt received. Halting Jarvis immediately...[/yellow]")
            if tts:
                tts.stop()
            recorder.request_stop()
            asyncio.create_task(hud.hide())
            for f in (PID_FILE, STATE_FILE, TIME_FILE):
                try:
                    if f.exists():
                        f.unlink()
                except Exception:
                    pass
            sys.exit(0)

        def handle_ptt_stop(*_):
            log.info("[cyan]Push-to-Talk stop signal received (SIGUSR1). Finalizing audio...[/cyan]")
            recorder.request_stop()

        for sig in (signal.SIGINT, signal.SIGTERM):
            try:
                loop.add_signal_handler(sig, handle_interrupt)
            except NotImplementedError:
                signal.signal(sig, handle_interrupt)

        try:
            loop.add_signal_handler(signal.SIGUSR1, handle_ptt_stop)
        except NotImplementedError:
            signal.signal(signal.SIGUSR1, handle_ptt_stop)
    else:
        TIME_FILE.write_text(str(time.monotonic()))
        set_current_state("recording")

    last_vol_time = 0.0

    def on_volume_update(volume: float) -> None:
        nonlocal last_vol_time
        now = time.monotonic()
        # Throttle IPC volume calls to ~16 fps (every 60ms) to keep Wayland IPC smooth
        if now - last_vol_time >= 0.06:
            last_vol_time = now
            asyncio.run_coroutine_threadsafe(hud.update_volume(volume), loop)

    is_first_turn = True

    try:
        while True:
            if cancel_event and cancel_event.is_set():
                log.info("Voice turn cancelled by event.")
                break

            # Step 1: Open animated transparent wave HUD in Listening state
            set_current_state("recording")
            caption = "Listening..." if is_first_turn else "Listening... (or say 'That's it')"
            set_recording(caption)
            TIME_FILE.write_text(str(time.monotonic()))
            await hud.show_listening(caption)

            initial_timeout = (
                settings.initial_listen_timeout if is_first_turn else settings.followup_listen_timeout
            )

            # Step 2: Record speech until fast VAD detects natural silence (~1.1s) or PTT release
            wav_bytes = await recorder.record_phrase(
                on_volume=on_volume_update,
                initial_timeout=initial_timeout,
            )

            if cancel_event and cancel_event.is_set():
                log.info("Voice turn cancelled during recording.")
                break

            if not wav_bytes:
                if is_first_turn:
                    log.info("No initial audio detected.")
                else:
                    log.info("Follow-up silence timeout elapsed. Concluding conversation session.")
                break

            # Step 3: Transition HUD to Thinking state & transcribe via Groq Turbo
            set_current_state("processing")
            set_processing("Transcribing...")
            await hud.show_thinking("Transcribing...")
            transcript = await stt.transcribe(wav_bytes)

            if cancel_event and cancel_event.is_set():
                break

            if not transcript or not transcript.strip():
                if is_first_turn:
                    log.warning("No speech transcribed.")
                    set_speaking("I didn't catch that, sir.")
                    await hud.show_speaking("I didn't catch that, sir.")
                    if tts:
                        await tts.speak("I didn't catch that, sir.")
                break

            console.print(f'[bold green]User:[/bold green] "{transcript}"')
            set_processing(transcript)
            await hud.show_thinking(f'"{transcript}"')

            # Step 4: Check for kill words / dismissal phrases
            if is_exit_command(transcript):
                farewell = "Very well, sir. Have a wonderful day."
                console.print(f"[bold cyan]󰚩 Jarvis:[/bold cyan] {farewell}")
                set_current_state("speaking")
                set_speaking(farewell)
                await hud.show_speaking(farewell)
                if tts:
                    await tts.speak(farewell)
                break

            # Step 5: Reason and execute tools via Gemini Flash (maintains multi-turn context)
            if cancel_event and cancel_event.is_set():
                break
            reply = await agent.process_prompt(transcript)
            if cancel_event and cancel_event.is_set():
                break
            console.print(f"[bold cyan]󰚩 Jarvis:[/bold cyan] {reply}")

            # Step 6: Speak reply while HUD pulses
            if reply:
                set_current_state("speaking")
                set_speaking(reply)
                await hud.show_speaking(reply)
                if tts:
                    await tts.speak(reply)

            # Check if Gemini agent concluded the session via dismiss_session tool
            if agent.session_ended:
                log.info("Agent concluded conversation session.")
                break

            # Mark first turn complete and pause briefly before next listen
            is_first_turn = False
            await asyncio.sleep(0.15)

    except Exception as e:
        log.error(f"Voice activation error: {e}")
    finally:
        if recorder_holder is not None:
            recorder_holder.clear()
        if tts_holder is not None:
            tts_holder.clear()

        daemon_alive = False
        if DAEMON_PID_FILE.exists():
            try:
                d_pid = int(DAEMON_PID_FILE.read_text().strip())
                daemon_alive = is_process_alive(d_pid)
            except Exception:
                pass
        set_idle(wakeword_active=daemon_alive)
        try:
            await hud.hide()
        except Exception:
            pass
        cleanup_files = [STATE_FILE, TIME_FILE]
        if not is_daemon:
            cleanup_files.append(PID_FILE)
        for f in cleanup_files:
            try:
                if f.exists():
                    f.unlink()
            except Exception:
                pass


async def run_daemon(speak_reply: bool = True) -> None:
    """Run Jarvis background daemon listening for 'Hey Jarvis' wake word."""
    if DAEMON_PID_FILE.exists():
        try:
            old_pid = int(DAEMON_PID_FILE.read_text().strip())
            if is_process_alive(old_pid) and old_pid != os.getpid():
                console.print(f"[yellow]Jarvis daemon is already running (PID: {old_pid}).[/yellow]")
                return
        except Exception:
            pass

    DAEMON_PID_FILE.write_text(str(os.getpid()))
    update_status(daemon_running=True, state="wakeword", wakeword_enabled=True)
    detector = WakeWordDetector()
    stop_event = asyncio.Event()
    pause_event = asyncio.Event()
    trigger_event = asyncio.Event()
    cancel_event = asyncio.Event()

    active_recorders: list[AudioRecorder] = []
    active_ttss: list[TextToSpeech] = []

    loop = asyncio.get_running_loop()

    def handle_stop(*_):
        log.info("[yellow]Stopping Jarvis background daemon...[/yellow]")
        stop_event.set()
        cancel_event.set()
        for r in active_recorders:
            try:
                r.request_stop()
            except Exception:
                pass
        for t in active_ttss:
            try:
                t.stop()
            except Exception:
                pass
        try:
            sd.stop()
        except Exception:
            pass
        update_status(daemon_running=False, state="idle", mic_active=False)
        try:
            if DAEMON_PID_FILE.exists():
                DAEMON_PID_FILE.unlink()
        except Exception:
            pass
        sys.exit(0)

    def handle_ptt_stop(*_):
        """Handle SIGUSR1: Push-to-Talk release / finalize recording."""
        log.info("[cyan]Daemon: PTT stop signal received (SIGUSR1). Finalizing audio...[/cyan]")
        for r in active_recorders:
            try:
                r.request_stop()
            except Exception:
                pass

    def handle_toggle(*_):
        """Handle SIGUSR2: Hotkey / menubar trigger voice session toggle."""
        state = get_current_state()
        log.info(f"[cyan]Daemon: Voice toggle signal received (SIGUSR2, state={state}).[/cyan]")
        if state == "recording":
            # Already recording -> submit speech
            for r in active_recorders:
                try:
                    r.request_stop()
                except Exception:
                    pass
        elif state in ("processing", "speaking"):
            # Active thinking/speaking -> interrupt and silence
            cancel_event.set()
            for t in active_ttss:
                try:
                    t.stop()
                except Exception:
                    pass
            try:
                sd.stop()
            except Exception:
                pass
            try:
                hud = JarvisHUD()
                asyncio.run_coroutine_threadsafe(hud.hide(), loop)
            except Exception:
                pass
        else:
            # Idle or wakeword -> trigger voice session immediately
            loop.call_soon_threadsafe(trigger_event.set)

    def handle_cancel(*_):
        """Handle SIGHUP: Emergency kill switch / silence active voice turn without killing daemon."""
        log.info("[yellow]Daemon: Emergency silence signal received (SIGHUP). Interrupting voice turn...[/yellow]")
        cancel_event.set()
        for r in active_recorders:
            try:
                r.request_stop()
            except Exception:
                pass
        for t in active_ttss:
            try:
                t.stop()
            except Exception:
                pass
        try:
            sd.stop()
        except Exception:
            pass
        try:
            hud = JarvisHUD()
            asyncio.run_coroutine_threadsafe(hud.hide(), loop)
        except Exception:
            pass

    for sig in (signal.SIGINT, signal.SIGTERM):
        try:
            loop.add_signal_handler(sig, handle_stop)
        except NotImplementedError:
            signal.signal(sig, handle_stop)

    try:
        loop.add_signal_handler(signal.SIGUSR1, handle_ptt_stop)
    except NotImplementedError:
        signal.signal(signal.SIGUSR1, handle_ptt_stop)

    try:
        loop.add_signal_handler(signal.SIGUSR2, handle_toggle)
    except NotImplementedError:
        signal.signal(signal.SIGUSR2, handle_toggle)

    try:
        loop.add_signal_handler(signal.SIGHUP, handle_cancel)
    except NotImplementedError:
        signal.signal(signal.SIGHUP, handle_cancel)

    async def on_detected():
        st = read_status()
        if not st.get("wakeword_enabled", True):
            log.info("Wake word detected but disabled in settings. Ignoring.")
            return

        log.info("[bold cyan]󰚩 Wake word 'Hey Jarvis' activated![/bold cyan]")
        cancel_event.clear()
        try:
            await run_voice_activation(
                speak_reply=speak_reply,
                is_daemon=True,
                cancel_event=cancel_event,
                recorder_holder=active_recorders,
                tts_holder=active_ttss,
            )
        finally:
            set_idle(wakeword_active=True)

    console.print("[bold cyan]󰚩 Jarvis Daemon active.[/bold cyan] Listening for [bold green]'Hey Jarvis'[/bold green] in the background...")
    try:
        await detector.listen_loop(
            on_detected=on_detected,
            stop_event=stop_event,
            pause_event=pause_event,
            trigger_event=trigger_event,
        )
    finally:
        update_status(daemon_running=False, state="idle", mic_active=False)
        try:
            if DAEMON_PID_FILE.exists():
                DAEMON_PID_FILE.unlink()
        except Exception:
            pass


def quit_all() -> None:
    """Completely stop all Jarvis processes, systemd service, background daemon, and release all resources."""
    console.print("[bold yellow]󰚩 Stopping all Jarvis processes and services...[/bold yellow]")

    # 1. Stop systemd service
    try:
        subprocess.run(["systemctl", "--user", "stop", "jarvis"], capture_output=True, timeout=5)
    except Exception:
        pass

    # 2. Stop running session & any spawned process
    stop_running_session()

    # 3. Terminate daemon PID if still alive
    if DAEMON_PID_FILE.exists():
        try:
            d_pid = int(DAEMON_PID_FILE.read_text().strip())
            if is_process_alive(d_pid) and d_pid != os.getpid():
                os.kill(d_pid, signal.SIGTERM)
                time.sleep(0.1)
                if is_process_alive(d_pid):
                    os.kill(d_pid, signal.SIGKILL)
        except Exception:
            pass
        try:
            DAEMON_PID_FILE.unlink()
        except Exception:
            pass

    # 4. Silence any active audio
    try:
        sd.stop()
    except Exception:
        pass

    # 5. Hide Quickshell HUD
    try:
        hud = JarvisHUD()
        asyncio.run(hud.hide())
    except Exception:
        pass

    # 6. Mark state as idle and inactive
    update_status(
        active=False,
        daemon_running=False,
        state="idle",
        mic_active=False,
    )
    console.print("[bold red]󰚩 Jarvis completely terminated.[/bold red] Background daemon and systemd service stopped.")


def restart_all() -> None:
    """Restart Jarvis background daemon and refresh Omarchy menubar plugin."""
    console.print("[bold cyan]󰚩 Restarting Jarvis...[/bold cyan]")

    # 1. Stop active sessions
    stop_running_session()

    # 2. Restart systemd service
    try:
        res = subprocess.run(["systemctl", "--user", "restart", "jarvis"], capture_output=True, text=True, timeout=5)
        if res.returncode == 0:
            console.print("[green]✔ Background systemd service (jarvis.service) restarted.[/green]")
        else:
            console.print(f"[yellow]Warning restarting systemd: {res.stderr.strip()}[/yellow]")
    except Exception as e:
        console.print(f"[yellow]Warning restarting service: {e}[/yellow]")

    # 3. Rescan Omarchy plugins
    try:
        subprocess.run(["omarchy-shell", "shell", "rescanPlugins"], capture_output=True, timeout=5)
        console.print("[green]✔ Omarchy menubar plugin rescanned.[/green]")
    except Exception:
        pass

    console.print("[bold green]✔ Jarvis restart complete.[/bold green]")


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="jarvis",
        description="Omarchy Jarvis - Intelligent Voice & System Automation Assistant",
    )
    parser.add_argument(
        "-t", "--trigger",
        action="store_true",
        help="Trigger voice listening mode (acts as push-to-talk toggle & interrupt)",
    )
    parser.add_argument(
        "-s", "--stop-recording",
        action="store_true",
        help="Stop recording immediately and process speech (Push-to-Talk key release)",
    )
    parser.add_argument(
        "-k", "--kill",
        action="store_true",
        help="Instantly kill any active voice session, silence audio, and dismiss HUD",
    )
    parser.add_argument(
        "-q", "--quit",
        action="store_true",
        help="Completely stop all Jarvis processes, background daemon, and systemd service",
    )
    parser.add_argument(
        "-r", "--restart",
        action="store_true",
        help="Restart Jarvis systemd daemon and refresh Omarchy menubar plugin",
    )
    parser.add_argument(
        "-d", "--daemon",
        action="store_true",
        help="Run as background daemon continuously listening for 'Hey Jarvis' wake word",
    )
    parser.add_argument(
        "--wakeword-toggle",
        action="store_true",
        help="Toggle background wake word detection on or off",
    )
    parser.add_argument(
        "--wakeword-status",
        action="store_true",
        help="Print current wake word detection status",
    )
    parser.add_argument(
        "-c", "--command",
        type=str,
        help="Execute a direct text command (headless mode)",
    )
    parser.add_argument(
        "--no-speech",
        action="store_true",
        help="Suppress spoken audio responses (text-only mode)",
    )
    parser.add_argument(
        "--status",
        action="store_true",
        help="Check Jarvis daemon status, API keys, and environment",
    )
    parser.add_argument(
        "--workflow-list",
        action="store_true",
        help="List all configured multi-workspace workflows",
    )
    parser.add_argument(
        "--workflow-show",
        type=str,
        metavar="NAME",
        help="Show detailed launch steps of a workflow",
    )
    parser.add_argument(
        "--workflow-capture",
        type=str,
        metavar="NAME",
        help="Capture currently open application windows into a new workflow preset",
    )
    parser.add_argument(
        "--workflow-delete",
        type=str,
        metavar="NAME",
        help="Delete a configured workflow preset",
    )
    parser.add_argument(
        "--workflow-launch",
        type=str,
        metavar="NAME",
        help="Launch a workflow preset across Hyprland workspaces",
    )

    args = parser.parse_args()


    # Complete Quit flag (-q / --quit)
    if args.quit:
        quit_all()
        return

    # Restart flag (-r / --restart)
    if args.restart:
        restart_all()
        return

    # Emergency Kill switch flag (-k / --kill)
    if args.kill:
        killed = stop_running_session()
        if killed:
            console.print("[bold red]󰚩 Jarvis stopped and silenced.[/bold red]")
        else:
            console.print("[dim]No active Jarvis session found to stop.[/dim]")
        return

    # Wake word toggle flag
    if args.wakeword_toggle:
        st = read_status()
        new_val = not st.get("wakeword_enabled", True)
        update_status(wakeword_enabled=new_val)
        status_str = "ENABLED" if new_val else "DISABLED"
        console.print(f"[bold cyan]󰚩 Jarvis Wake Word:[/bold cyan] [bold]{status_str}[/bold]")
        return

    # Wake word status flag
    if args.wakeword_status:
        st = read_status()
        enabled = st.get("wakeword_enabled", True)
        daemon_on = st.get("daemon_running", False)
        status_str = "[bold green]ON[/bold green]" if enabled else "[bold red]OFF[/bold red]"
        daemon_str = "[bold green]Running[/bold green]" if daemon_on else "[dim]Stopped[/dim]"
        console.print(f"Wake Word: {status_str} | Daemon: {daemon_str}")
        return

    # Daemon mode (-d / --daemon)
    if args.daemon:
        asyncio.run(run_daemon(speak_reply=not args.no_speech))
        return

    # Push-to-Talk key release handler (--stop-recording)
    if args.stop_recording:
        if DAEMON_PID_FILE.exists():
            try:
                d_pid = int(DAEMON_PID_FILE.read_text().strip())
                if is_process_alive(d_pid):
                    os.kill(d_pid, signal.SIGUSR1)
                    return
            except Exception:
                pass

        if PID_FILE.exists():
            try:
                pid = int(PID_FILE.read_text().strip())
                if is_process_alive(pid):
                    state = get_current_state()
                    if state == "recording":
                        os.kill(pid, signal.SIGUSR1)
                        return
            except Exception:
                pass
        return

    # Trigger hotkey flag (acts as PTT toggle or interrupt if already running)
    if args.trigger:
        # If daemon is running, forward toggle signal directly to daemon
        if DAEMON_PID_FILE.exists():
            try:
                d_pid = int(DAEMON_PID_FILE.read_text().strip())
                if is_process_alive(d_pid) and d_pid != os.getpid():
                    os.kill(d_pid, signal.SIGUSR2)
                    return
            except Exception:
                pass

        if PID_FILE.exists():
            try:
                pid = int(PID_FILE.read_text().strip())
                if is_process_alive(pid) and pid != os.getpid():
                    # Debounce rapid duplicate trigger within 1.2s of start (key repeat / chord bounce)
                    started_at = 0.0
                    if TIME_FILE.exists():
                        try:
                            started_at = float(TIME_FILE.read_text().strip())
                        except Exception:
                            pass
                    elapsed_since_start = time.monotonic() - started_at
                    if elapsed_since_start < 1.2:
                        return

                    state = get_current_state()
                    if state == "recording":
                        # User deliberately tapped hotkey again while recording -> submit speech immediately
                        os.kill(pid, signal.SIGUSR1)
                        console.print("[bold green]󰚩 Submitted audio for processing.[/bold green]")
                        return
                    else:
                        # User tapped hotkey while Jarvis was processing or speaking -> interrupt & silence
                        stop_running_session()
                        console.print("[bold yellow]󰚩 Jarvis interrupted and silenced.[/bold yellow]")
                        return
            except Exception:
                pass

        asyncio.run(run_voice_activation(speak_reply=not args.no_speech, is_daemon=False))
        return

    if args.command:
        asyncio.run(run_command_headless(args.command, speak_reply=not args.no_speech))
        return

    # Workflow CLI flags
    if args.workflow_list:
        from jarvis.tools.workflow import WorkflowManager
        mgr = WorkflowManager()
        console.print(mgr.list_workflows())
        return

    if args.workflow_show:
        from jarvis.tools.workflow import WorkflowManager
        mgr = WorkflowManager()
        console.print(mgr.get_workflow_details(args.workflow_show))
        return

    if args.workflow_capture:
        from jarvis.tools.workflow import WorkflowManager
        mgr = WorkflowManager()
        res = asyncio.run(mgr.capture_current_setup(args.workflow_capture))
        console.print(f"[bold green]󰚩 Workflow Captured:[/bold green]\n{res}")
        return

    if args.workflow_delete:
        from jarvis.tools.workflow import WorkflowManager
        mgr = WorkflowManager()
        res = mgr.delete_workflow(args.workflow_delete)
        console.print(f"[bold yellow]󰚩 Workflow Manager:[/bold yellow] {res}")
        return

    if args.workflow_launch:
        from jarvis.tools.workflow import WorkflowManager
        mgr = WorkflowManager()
        res = asyncio.run(mgr.launch_workflow(args.workflow_launch))
        console.print(f"[bold green]󰚩 Workflow Manager:[/bold green] {res}")
        return

    if args.status or len(sys.argv) == 1:

        console.print("[bold cyan]󰚩 Omarchy Jarvis[/bold cyan] v0.1.0")
        console.print(f"[dim]Python:[/dim] {sys.version.split()[0]}")
        console.print(f"[dim]Model:[/dim]  {settings.model_name}")
        console.print(f"[dim]STT:[/dim]    {settings.stt_engine} ({settings.whisper_model})")
        console.print(f"[dim]TTS:[/dim]    {settings.tts_engine} ({settings.piper_voice})")

        gemini_status = "[green]Configured[/green]" if settings.gemini_api_key else "[yellow]Missing (set in .env)[/yellow]"
        groq_status = "[green]Configured[/green]" if settings.groq_api_key else "[yellow]Missing (set in .env)[/yellow]"
        console.print(f"[dim]Google Gemini API Key:[/dim] {gemini_status}")
        console.print(f"[dim]Groq Whisper API Key:[/dim]  {groq_status}")
        return


if __name__ == "__main__":
    main()
