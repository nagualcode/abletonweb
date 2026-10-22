"""Runs the real LiveWeb remote script against a fake Live song.

    python3 tests/mock_live/run_mock_live.py

Then run the bridge against it:
    LIVE_PORT=9878 PORT=6091 live-remote serve   (with LIVEWEB_PORT=9878 here)
"""
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..', 'remote_script'))

from mock_song import Song  # noqa: E402
import LiveWeb  # noqa: E402


class App(object):
    def get_major_version(self):
        return 11

    def get_minor_version(self):
        return 3

    def get_bugfix_version(self):
        return 43


class CInstance(object):
    def __init__(self):
        self.song = Song(n_tracks=8, n_scenes=6)
        self.app = App()


def main():
    # LIVEWEB_PORT lets the mock run next to a real Live using 9877.
    from LiveWeb import surface as surface_module
    surface_module.PORT = int(os.environ.get('LIVEWEB_PORT', surface_module.PORT))
    c = CInstance()
    surface = LiveWeb.create_instance(c)
    interval = 0.1  # Live calls update_display roughly every 100 ms.
    try:
        while True:
            c.song.advance(interval)
            surface.update_display()
            time.sleep(interval)
    except KeyboardInterrupt:
        surface.disconnect()


if __name__ == '__main__':
    main()
