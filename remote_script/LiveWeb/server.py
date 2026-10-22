"""Non-blocking, single-client TCP server speaking newline-delimited JSON.

Everything runs on Live's main thread: `poll()` is called from the control
surface's update tick, so commands can touch the Live API directly.
"""
from __future__ import absolute_import

import errno
import json
import socket

MAX_BUFFERED_OUT = 4 * 1024 * 1024


class LineServer(object):
    def __init__(self, host, port, on_message, on_connect, log):
        self._addr = (host, port)
        self._on_message = on_message
        self._on_connect = on_connect
        self._log = log
        self._listener = None
        self._client = None
        self._inbuf = b''
        self._outbuf = b''

    @property
    def has_client(self):
        return self._client is not None

    def start(self):
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        s.bind(self._addr)
        s.listen(1)
        s.setblocking(False)
        self._listener = s

    def close(self):
        self._drop_client()
        if self._listener is not None:
            try:
                self._listener.close()
            except Exception:
                pass
            self._listener = None

    def poll(self):
        self._accept()
        self._read()
        self._flush()

    def send(self, obj):
        if self._client is None:
            return
        if len(self._outbuf) > MAX_BUFFERED_OUT:
            self._log('client not reading, dropping connection')
            self._drop_client()
            return
        data = json.dumps(obj, separators=(',', ':')) + '\n'
        self._outbuf += data.encode('utf-8')

    def _accept(self):
        if self._listener is None:
            return
        try:
            conn, addr = self._listener.accept()
        except socket.error as e:
            if e.args and e.args[0] in (errno.EAGAIN, errno.EWOULDBLOCK):
                return
            raise
        # Latest connection wins: a restarted bridge replaces a stale one.
        self._drop_client()
        conn.setblocking(False)
        conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self._client = conn
        self._log('bridge connected from %s:%d' % addr)
        self._on_connect()

    def _read(self):
        while self._client is not None:
            try:
                chunk = self._client.recv(65536)
            except socket.error as e:
                if e.args and e.args[0] in (errno.EAGAIN, errno.EWOULDBLOCK):
                    break
                self._drop_client()
                return
            if not chunk:
                self._drop_client()
                return
            self._inbuf += chunk
        while b'\n' in self._inbuf:
            line, self._inbuf = self._inbuf.split(b'\n', 1)
            if not line.strip():
                continue
            try:
                msg = json.loads(line.decode('utf-8'))
            except ValueError:
                self._log('invalid json from bridge')
                continue
            self._on_message(msg)

    def _flush(self):
        while self._client is not None and self._outbuf:
            try:
                n = self._client.send(self._outbuf)
            except socket.error as e:
                if e.args and e.args[0] in (errno.EAGAIN, errno.EWOULDBLOCK):
                    return
                self._drop_client()
                return
            self._outbuf = self._outbuf[n:]

    def _drop_client(self):
        if self._client is not None:
            try:
                self._client.close()
            except Exception:
                pass
            self._client = None
            self._log('bridge disconnected')
        self._inbuf = b''
        self._outbuf = b''
