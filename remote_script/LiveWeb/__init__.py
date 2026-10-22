"""LiveWeb: exposes the Live session to the abletonweb bridge.

Must stay compatible with the Python bundled with Live 11 (3.7).
"""
from __future__ import absolute_import

from .surface import LiveWeb


def create_instance(c_instance):
    return LiveWeb(c_instance)
