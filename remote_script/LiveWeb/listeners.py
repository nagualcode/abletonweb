"""Bookkeeping for Live API listeners so they can all be dropped at once."""
from __future__ import absolute_import


class ListenerRegistry(object):
    def __init__(self, log):
        self._log = log
        self._items = []

    def add(self, obj, prop, callback):
        adder = getattr(obj, 'add_%s_listener' % prop, None)
        if adder is None:
            return False
        try:
            adder(callback)
        except Exception as e:
            self._log('cannot listen to %s: %s' % (prop, e))
            return False
        self._items.append((obj, prop, callback))
        return True

    def clear(self):
        for obj, prop, callback in self._items:
            try:
                if getattr(obj, '%s_has_listener' % prop)(callback):
                    getattr(obj, 'remove_%s_listener' % prop)(callback)
            except Exception:
                # The object may already be gone (deleted track, clip...).
                pass
        self._items = []

    def __len__(self):
        return len(self._items)
