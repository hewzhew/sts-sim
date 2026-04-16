#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import queue
import secrets
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any


BOOTSTRAP_PREFIX = "__LIVE_COMM_BOOTSTRAP__ "
REPO_ROOT = Path(__file__).resolve().parents[2]
LOG_ROOT = REPO_ROOT / "logs" / "current"
LATEST_FRAME_PATH = LOG_ROOT / "manual_client_latest.json"
BRIDGE_LOG_PATH = LOG_ROOT / "manual_client_bridge.log"
RAW_FRAME_LOG_PATH = LOG_ROOT / "manual_client_raw.jsonl"
HOST = "127.0.0.1"


def ensure_utf8_stdio() -> None:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")


def append_log(message: str) -> None:
    LOG_ROOT.mkdir(parents=True, exist_ok=True)
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    with BRIDGE_LOG_PATH.open("a", encoding="utf-8") as fh:
        fh.write(f"[{timestamp}] {message}\n")


def summarize_frame(payload: dict[str, Any]) -> str:
    meta = payload.get("protocol_meta") or {}
    state = payload.get("game_state") or {}
    rid = meta.get("response_id")
    frame_id = meta.get("state_frame_id")
    room_type = state.get("room_type")
    room_phase = state.get("room_phase")
    screen_type = state.get("screen_type")
    ready = payload.get("ready_for_command")
    avail = payload.get("available_commands") or []
    err = payload.get("error")
    parts = [
        f"rid={rid}" if rid is not None else "rid=?",
        f"frame={frame_id}" if frame_id is not None else "frame=-",
        f"ready={ready}",
    ]
    if room_type or room_phase:
        parts.append(f"room={room_type}/{room_phase}")
    if screen_type:
        parts.append(f"screen={screen_type}")
    if err:
        parts.append(f"error={err}")
    if avail:
        parts.append("commands=" + ",".join(str(x) for x in avail))
    return " | ".join(parts)


def print_json(value: Any) -> None:
    print(json.dumps(value, ensure_ascii=False, indent=2))


def get_latest_payload(latest_holder: dict[str, Any]) -> dict[str, Any] | None:
    payload = latest_holder.get("payload")
    return payload if isinstance(payload, dict) else None


def get_path(root: Any, path: str) -> Any:
    current = root
    if not path.strip():
        return current
    for segment in path.split("."):
        if not segment:
            continue
        if isinstance(current, list):
            try:
                index = int(segment)
            except ValueError as exc:
                raise KeyError(f"list index required for segment '{segment}'") from exc
            try:
                current = current[index]
            except IndexError as exc:
                raise KeyError(f"list index out of range: {index}") from exc
            continue
        if not isinstance(current, dict):
            raise KeyError(f"cannot descend into non-object at segment '{segment}'")
        if segment not in current:
            raise KeyError(f"missing key '{segment}'")
        current = current[segment]
    return current


def monster_summary(monster: dict[str, Any], index: int) -> dict[str, Any]:
    powers = monster.get("powers") or []
    return {
        "index": index,
        "id": monster.get("id"),
        "name": monster.get("name"),
        "hp": f"{monster.get('current_hp')}/{monster.get('max_hp')}",
        "block": monster.get("block"),
        "intent": monster.get("intent"),
        "move_id": monster.get("move_id"),
        "powers": [
            {
                "id": power.get("id"),
                "amount": power.get("amount"),
            }
            for power in powers
            if isinstance(power, dict)
        ],
        "runtime_state": monster.get("runtime_state"),
    }


def find_matches(value: Any, needle: str, path: str = "$", out: list[dict[str, Any]] | None = None) -> list[dict[str, Any]]:
    if out is None:
        out = []
    lowered = needle.lower()
    if isinstance(value, dict):
        for key, child in value.items():
            child_path = f"{path}.{key}"
            if lowered in str(key).lower():
                out.append({"path": child_path, "match": "key", "value": child})
            find_matches(child, needle, child_path, out)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            find_matches(child, needle, f"{path}.{index}", out)
    else:
        rendered = str(value)
        if lowered in rendered.lower():
            out.append({"path": path, "match": "value", "value": value})
    return out


class BridgeState:
    def __init__(self, port: int, token: str) -> None:
        self.port = port
        self.token = token
        self.command_queue: queue.Queue[str] = queue.Queue()
        self.shutdown = threading.Event()
        self.client_lock = threading.Lock()
        self.client_conn: socket.socket | None = None
        self.latest_payload: dict[str, Any] | None = None

    def set_client(self, conn: socket.socket | None) -> None:
        with self.client_lock:
            previous = self.client_conn
            self.client_conn = conn
        if previous is not None and previous is not conn:
            try:
                previous.close()
            except OSError:
                pass

    def send_to_client(self, message: dict[str, Any]) -> None:
        encoded = (json.dumps(message, ensure_ascii=False) + "\n").encode("utf-8")
        with self.client_lock:
            conn = self.client_conn
        if conn is None:
            return
        try:
            conn.sendall(encoded)
        except OSError:
            self.set_client(None)


def start_terminal_process(port: int, token: str) -> None:
    creationflags = getattr(subprocess, "CREATE_NEW_CONSOLE", 0)
    cmd = [
        sys.executable,
        str(Path(__file__).resolve()),
        "--terminal",
        "--port",
        str(port),
        "--token",
        token,
    ]
    append_log(f"spawning terminal client: {cmd!r}")
    subprocess.Popen(cmd, creationflags=creationflags, cwd=str(Path(__file__).parent))


def accept_loop(server: socket.socket, state: BridgeState) -> None:
    while not state.shutdown.is_set():
        try:
            conn, addr = server.accept()
        except OSError:
            break
        conn_file = conn.makefile("r", encoding="utf-8", errors="replace")
        try:
            hello = conn_file.readline()
            if not hello:
                conn.close()
                continue
            try:
                hello_obj = json.loads(hello)
            except json.JSONDecodeError:
                conn.close()
                continue
            if hello_obj.get("type") != "hello" or hello_obj.get("token") != state.token:
                conn.close()
                continue
            state.set_client(conn)
            state.send_to_client(
                {
                    "type": "system",
                    "message": "manual scenario terminal attached",
                    "port": state.port,
                }
            )
            state.command_queue.put("STATE")
            if state.latest_payload is not None:
                state.send_to_client({"type": "frame", "payload": state.latest_payload})
            append_log(f"terminal client connected from {addr!r}")
            while not state.shutdown.is_set():
                line = conn_file.readline()
                if not line:
                    break
                try:
                    message = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if message.get("type") == "command":
                    command = str(message.get("command") or "").strip()
                    if command:
                        state.command_queue.put(command)
                elif message.get("type") == "shutdown":
                    state.shutdown.set()
                    break
        finally:
            try:
                conn_file.close()
            except OSError:
                pass
            state.set_client(None)
            try:
                conn.close()
            except OSError:
                pass
            append_log("terminal client disconnected")


def game_reader_loop(state: BridgeState) -> None:
    for raw_line in sys.stdin:
        if state.shutdown.is_set():
            break
        raw_line = raw_line.strip()
        if not raw_line:
            continue
        LOG_ROOT.mkdir(parents=True, exist_ok=True)
        with RAW_FRAME_LOG_PATH.open("a", encoding="utf-8") as fh:
            fh.write(raw_line + "\n")
        try:
            payload = json.loads(raw_line)
        except json.JSONDecodeError:
            state.send_to_client({"type": "raw", "payload": raw_line})
            continue
        state.latest_payload = payload
        LATEST_FRAME_PATH.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        state.send_to_client({"type": "frame", "payload": payload})
    state.shutdown.set()
    state.send_to_client({"type": "system", "message": "game pipe closed"})


def bridge_main() -> int:
    LOG_ROOT.mkdir(parents=True, exist_ok=True)
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.bind((HOST, 0))
    server.listen(1)
    port = server.getsockname()[1]
    token = secrets.token_hex(12)
    state = BridgeState(port, token)

    accept_thread = threading.Thread(target=accept_loop, args=(server, state), daemon=True)
    accept_thread.start()
    reader_thread = threading.Thread(target=game_reader_loop, args=(state,), daemon=True)
    reader_thread.start()

    try:
        start_terminal_process(port, token)
    except Exception as exc:  # pragma: no cover - operator-visible fallback
        append_log(f"failed to spawn terminal client: {exc!r}")

    bootstrap_payload = {
        "kind": "manual_scenario_bridge_bootstrap",
        "protocol_version": 2,
        "client": "manual_scenario_bridge",
        "port": port,
    }
    sys.stdout.write(BOOTSTRAP_PREFIX + json.dumps(bootstrap_payload, ensure_ascii=False) + "\n")
    sys.stdout.flush()
    append_log(f"bridge bootstrapped on {HOST}:{port}")

    while not state.shutdown.is_set():
        try:
            command = state.command_queue.get(timeout=0.25)
        except queue.Empty:
            continue
        sys.stdout.write(command + "\n")
        sys.stdout.flush()
        append_log(f"sent command: {command}")

    try:
        server.close()
    except OSError:
        pass
    return 0


def terminal_reader(sock: socket.socket, latest_holder: dict[str, Any]) -> None:
    sock_file = sock.makefile("r", encoding="utf-8", errors="replace")
    try:
        for line in sock_file:
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                print(f"[manual] invalid bridge payload: {line.rstrip()}")
                continue
            msg_type = message.get("type")
            if msg_type == "system":
                print(f"[manual] {message.get('message')}")
            elif msg_type == "raw":
                print(f"[raw] {message.get('payload')}")
            elif msg_type == "frame":
                payload = message.get("payload")
                if isinstance(payload, dict):
                    latest_holder["payload"] = payload
                    print(f"\n[{summarize_frame(payload)}]")
                    print("manual> ", end="", flush=True)
            else:
                print(f"[manual] {message}")
    except (TimeoutError, OSError):
        pass
    finally:
        sock_file.close()
        print("\n[manual] bridge connection closed")


def terminal_main(port: int, token: str) -> int:
    ensure_utf8_stdio()
    sock = socket.create_connection((HOST, port), timeout=10.0)
    sock.settimeout(None)
    sock.sendall(
        (json.dumps({"type": "hello", "token": token}, ensure_ascii=False) + "\n").encode("utf-8")
    )
    latest_holder: dict[str, Any] = {}
    reader = threading.Thread(target=terminal_reader, args=(sock, latest_holder), daemon=True)
    reader.start()

    print("Manual scenario console attached.")
    print("Type raw CommunicationMod commands such as:")
    print("  START ironclad 0")
    print("  STATE")
    print("  scenario fight jaw_worm")
    print("  scenario deck add combust 1 0")
    print("Local commands: /help /show /commands /state /combat /player /monsters /monster N /relics /protocol /path a.b.c /find term /quit")

    try:
        while True:
            try:
                command = input("manual> ").strip()
            except EOFError:
                command = "/quit"
            if not command:
                continue
            if command == "/help":
                print("Raw commands are forwarded to CommunicationMod.")
                print("Local commands:")
                print("  /show")
                print("  /commands")
                print("  /combat")
                print("  /player")
                print("  /monsters")
                print("  /monster N")
                print("  /relics")
                print("  /protocol")
                print("  /path game_state.combat_state.monsters.0.runtime_state")
                print("  /find guardian_threshold")
                print("  /state")
                print("  /quit")
                continue
            payload = get_latest_payload(latest_holder)
            if command == "/show":
                if payload is None:
                    print("No frame received yet.")
                else:
                    print_json(payload)
                continue
            if command == "/commands":
                payload = payload or {}
                commands = payload.get("available_commands") or []
                print_json(commands)
                continue
            if command == "/combat":
                if payload is None:
                    print("No frame received yet.")
                    continue
                try:
                    print_json(get_path(payload, "game_state.combat_state"))
                except KeyError as exc:
                    print(f"Missing combat_state: {exc}")
                continue
            if command == "/player":
                if payload is None:
                    print("No frame received yet.")
                    continue
                try:
                    print_json(get_path(payload, "game_state.combat_state.player"))
                except KeyError as exc:
                    print(f"Missing player combat state: {exc}")
                continue
            if command == "/monsters":
                if payload is None:
                    print("No frame received yet.")
                    continue
                try:
                    monsters = get_path(payload, "game_state.combat_state.monsters")
                except KeyError as exc:
                    print(f"Missing monsters: {exc}")
                    continue
                if not isinstance(monsters, list):
                    print("combat_state.monsters is not a list")
                    continue
                print_json([monster_summary(monster, index) for index, monster in enumerate(monsters) if isinstance(monster, dict)])
                continue
            if command.startswith("/monster "):
                if payload is None:
                    print("No frame received yet.")
                    continue
                parts = command.split(maxsplit=1)
                try:
                    monster_index = int(parts[1])
                except (IndexError, ValueError):
                    print("Usage: /monster <index>")
                    continue
                try:
                    monster = get_path(payload, f"game_state.combat_state.monsters.{monster_index}")
                except KeyError as exc:
                    print(f"Missing monster {monster_index}: {exc}")
                    continue
                print_json(monster)
                continue
            if command == "/relics":
                if payload is None:
                    print("No frame received yet.")
                    continue
                try:
                    print_json(get_path(payload, "game_state.relics"))
                except KeyError as exc:
                    print(f"Missing relics: {exc}")
                continue
            if command == "/protocol":
                if payload is None:
                    print("No frame received yet.")
                    continue
                try:
                    print_json(get_path(payload, "protocol_meta"))
                except KeyError as exc:
                    print(f"Missing protocol_meta: {exc}")
                continue
            if command.startswith("/path "):
                if payload is None:
                    print("No frame received yet.")
                    continue
                path = command.split(maxsplit=1)[1].strip()
                if not path:
                    print("Usage: /path a.b.c")
                    continue
                try:
                    print_json(get_path(payload, path))
                except KeyError as exc:
                    print(f"Missing path {path}: {exc}")
                continue
            if command.startswith("/find "):
                if payload is None:
                    print("No frame received yet.")
                    continue
                needle = command.split(maxsplit=1)[1].strip()
                if not needle:
                    print("Usage: /find term")
                    continue
                matches = find_matches(payload, needle)
                if not matches:
                    print(f"No matches for '{needle}'.")
                else:
                    print_json(matches[:50])
                continue
            if command == "/state":
                command = "STATE"
            if command == "/quit":
                sock.sendall((json.dumps({"type": "shutdown"}, ensure_ascii=False) + "\n").encode("utf-8"))
                return 0
            sock.sendall(
                (json.dumps({"type": "command", "command": command}, ensure_ascii=False) + "\n").encode("utf-8")
            )
    finally:
        try:
            sock.close()
        except OSError:
            pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Manual scenario bridge/client for CommunicationMod")
    parser.add_argument("--terminal", action="store_true", help="Run the human-facing REPL terminal")
    parser.add_argument("--port", type=int, default=0, help="Loopback port for terminal mode")
    parser.add_argument("--token", default="", help="Bridge auth token for terminal mode")
    return parser.parse_args()


def main() -> int:
    ensure_utf8_stdio()
    args = parse_args()
    if args.terminal:
        if not args.port or not args.token:
            raise SystemExit("--terminal requires --port and --token")
        return terminal_main(args.port, args.token)
    return bridge_main()


if __name__ == "__main__":
    raise SystemExit(main())
