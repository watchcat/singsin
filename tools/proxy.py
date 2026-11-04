#!/usr/bin/env python3
"""A tiny CORS relay for the words.hk reference recordings.

`words.hk` does not send an `Access-Control-Allow-Origin` header, so a
browser cannot fetch its MP3s directly. This relay fetches them server-side
and hands them back with a permissive CORS header, so SingSin can decode them
with `AudioContext.decodeAudioData`.

Usage
-----

    python3 tools/proxy.py                # listens on 127.0.0.1:8081
    python3 tools/proxy.py --port 9000

SingSin tries this relay first, so with it running the reference audio is
fast and dependable. Only the standard library is required.
"""

from __future__ import annotations

import argparse
import http.server
import socketserver
import sys
import urllib.error
import urllib.parse
import urllib.request

UPSTREAM = "https://words.hk/static/jyutping/"
TIMEOUT = 15
ALLOW_ORIGIN = "*"


class Relay(http.server.BaseHTTPRequestHandler):
    server_version = "SingSinRelay/1.0"

    def _resolve(self) -> str:
        """Work out which upstream URL was asked for."""
        parsed = urllib.parse.urlparse(self.path)
        query = urllib.parse.parse_qs(parsed.query)
        if "url" in query and query["url"]:
            return query["url"][0]
        # Also accept /<jyutping>.mp3 for convenience.
        name = parsed.path.strip("/")
        if name.endswith(".mp3"):
            name = name[:-4]
        if name:
            return f"{UPSTREAM}{name}.mp3"
        raise ValueError("no target")

    def _cors(self) -> None:
        self.send_header("Access-Control-Allow-Origin", ALLOW_ORIGIN)
        self.send_header("Access-Control-Allow-Methods", "GET, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "*")
        self.send_header("Access-Control-Max-Age", "86400")

    def do_OPTIONS(self) -> None:  # noqa: N802 (http.server API)
        self.send_response(204)
        self._cors()
        self.end_headers()

    def do_GET(self) -> None:  # noqa: N802 (http.server API)
        try:
            target = self._resolve()
        except ValueError:
            self.send_error(400, "no target")
            return

        try:
            with urllib.request.urlopen(target, timeout=TIMEOUT) as upstream:
                body = upstream.read()
                content_type = upstream.headers.get("Content-Type", "audio/mpeg")
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError) as exc:
            self.send_error(502, f"upstream failed: {exc}")
            return

        self.send_response(200)
        self._cors()
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "public, max-age=86400")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args) -> None:
        sys.stderr.write("relay: " + fmt % args + "\n")


class Server(socketserver.ThreadingTCPServer):
    daemon_threads = True
    allow_reuse_address = True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8081)
    args = parser.parse_args()

    with Server((args.host, args.port), Relay) as httpd:
        print(f"SingSin relay listening on http://{args.host}:{args.port}/", flush=True)
        print(f"Try: http://{args.host}:{args.port}/?url={UPSTREAM}nei5.mp3", flush=True)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nstopped", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
