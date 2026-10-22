"""Stand-in for Live's _Framework.ControlSurface used by the mock runner."""


class ControlSurface(object):
    def __init__(self, c_instance):
        self._c_instance = c_instance

    def song(self):
        return self._c_instance.song

    def application(self):
        return self._c_instance.app

    def update_display(self):
        pass

    def disconnect(self):
        pass

    def log_message(self, *args):
        print(' '.join(str(a) for a in args), flush=True)

    def show_message(self, message):
        print('[status bar] ' + message, flush=True)
