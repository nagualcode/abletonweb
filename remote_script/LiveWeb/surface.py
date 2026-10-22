from __future__ import absolute_import

import traceback

from _Framework.ControlSurface import ControlSurface

from . import state
from .commands import CommandError, Commands
from .listeners import ListenerRegistry
from .server import LineServer

HOST = '127.0.0.1'
PORT = 9877
PROTOCOL_VERSION = 1


class LiveWeb(ControlSurface):
    def __init__(self, c_instance):
        ControlSurface.__init__(self, c_instance)
        self._listeners = ListenerRegistry(self._log)
        self._structure_dirty = True
        self._state_dirty = True
        # Keep one bound method per callback so has/remove_listener match.
        self._on_structure = self._mark_structure
        self._on_state = self._mark_state
        self._commands = Commands(self.song)
        self._server = LineServer(HOST, PORT, self._on_message, self._on_connect, self._log)
        try:
            self._server.start()
            self._log('listening on %s:%d' % (HOST, PORT))
            self.show_message('LiveWeb: listening on port %d' % PORT)
        except Exception as e:
            self._log('cannot listen on port %d: %s' % (PORT, e))
            self.show_message('LiveWeb: port %d unavailable' % PORT)

    def disconnect(self):
        self._listeners.clear()
        self._server.close()
        ControlSurface.disconnect(self)

    def update_display(self):
        ControlSurface.update_display(self)
        try:
            self._update()
        except Exception:
            self._log(traceback.format_exc())

    def _update(self):
        self._server.poll()
        if self._structure_dirty:
            self._structure_dirty = False
            self._listeners.clear()
            state.bind(self.song(), self._listeners, self._on_structure, self._on_state)
            self._state_dirty = True
        if not self._server.has_client:
            return
        if self._state_dirty:
            self._state_dirty = False
            self._server.send({'t': 'state', 'state': state.snapshot(self.song())})
        # 't' first: the bridge forwards ticks without parsing them.
        tick = {'t': 'tick'}
        tick.update(state.tick(self.song()))
        self._server.send(tick)
        self._server.poll()

    def _mark_structure(self):
        self._structure_dirty = True

    def _mark_state(self):
        self._state_dirty = True

    def _on_connect(self):
        app = self.application()
        self._server.send({
            't': 'hello',
            'protocol': PROTOCOL_VERSION,
            'live': '%d.%d.%d' % (app.get_major_version(), app.get_minor_version(),
                                  app.get_bugfix_version()),
        })
        self._state_dirty = True

    def _on_message(self, msg):
        msg_id = msg.get('id')
        try:
            result = self._commands.run(msg.get('cmd'), msg.get('args'))
            reply = {'t': 'reply', 'id': msg_id, 'ok': True, 'result': result}
        except CommandError as e:
            reply = {'t': 'reply', 'id': msg_id, 'ok': False, 'error': str(e)}
        except Exception as e:
            self._log(traceback.format_exc())
            reply = {'t': 'reply', 'id': msg_id, 'ok': False, 'error': repr(e)}
        self._server.send(reply)
        # Some state (undo history, ...) has no listener; refresh after commands.
        self._state_dirty = True

    def _log(self, message):
        self.log_message('LiveWeb: ' + message)
