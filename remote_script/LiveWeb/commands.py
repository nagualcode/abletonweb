"""Commands the bridge can run. Each takes the args dict and returns a
JSON-serialisable result (or None)."""
from __future__ import absolute_import

from . import state

# Song properties the bridge may set directly, with their coercion.
SONG_SETTABLE = {
    'record_mode': bool,
    'session_record': bool,
    'metronome': bool,
    'arrangement_overdub': bool,
    'session_automation_record': bool,
    'loop': bool,
    'back_to_arranger': bool,
    'tempo': float,
    'current_song_time': float,
    'loop_start': float,
    'loop_length': float,
}


class CommandError(Exception):
    pass


class Commands(object):
    def __init__(self, song_getter):
        self._song = song_getter

    def run(self, name, args):
        method = getattr(self, 'cmd_' + str(name), None)
        if method is None:
            raise CommandError('unknown command %r' % (name,))
        return method(args or {})

    # ------------------------------------------------------------ transport

    def cmd_ping(self, args):
        return 'pong'

    def cmd_get_state(self, args):
        return state.snapshot(self._song())

    def cmd_play(self, args):
        self._song().start_playing()

    def cmd_continue(self, args):
        self._song().continue_playing()

    def cmd_stop(self, args):
        self._song().stop_playing()

    def cmd_toggle_play(self, args):
        song = self._song()
        if song.is_playing:
            song.stop_playing()
        else:
            song.start_playing()

    def cmd_record(self, args):
        """Arrangement record, like the transport Record button: when
        enabling while stopped, playback starts too."""
        song = self._song()
        enable = args.get('value', not song.record_mode)
        song.record_mode = bool(enable)
        if enable and not song.is_playing:
            song.start_playing()

    def cmd_session_record(self, args):
        """Session Record button: records into the selected scene of armed
        tracks (or toggles overdub when clips are already playing)."""
        song = self._song()
        length = args.get('length')
        if length is None:
            song.trigger_session_record()
        else:
            song.trigger_session_record(float(length))

    def cmd_set_song(self, args):
        prop = args.get('prop')
        if prop not in SONG_SETTABLE:
            raise CommandError('song property %r is not settable' % (prop,))
        setattr(self._song(), prop, SONG_SETTABLE[prop](args.get('value')))

    def cmd_tap_tempo(self, args):
        self._song().tap_tempo()

    def cmd_undo(self, args):
        song = self._song()
        if song.can_undo:
            song.undo()

    def cmd_redo(self, args):
        song = self._song()
        if song.can_redo:
            song.redo()

    def cmd_stop_all_clips(self, args):
        self._song().stop_all_clips(args.get('quantized', True))

    def cmd_capture_midi(self, args):
        song = self._song()
        if song.can_capture_midi:
            song.capture_midi()

    def cmd_re_enable_automation(self, args):
        self._song().re_enable_automation()

    # ---------------------------------------------------------------- tracks

    def cmd_set_track(self, args):
        song = self._song()
        track = self._track(args)
        prop = args.get('prop')
        value = args.get('value')
        mixer = track.mixer_device
        if prop == 'volume':
            _set_param(mixer.volume, value)
        elif prop == 'pan':
            _set_param(mixer.panning, value)
        elif prop == 'send':
            sends = mixer.sends
            index = int(args.get('index', -1))
            if not 0 <= index < len(sends):
                raise CommandError('no send %r' % (index,))
            _set_param(sends[index], value)
        elif prop == 'mute':
            track.mute = bool(value)
        elif prop == 'solo':
            if value and args.get('exclusive'):
                for other in list(song.tracks) + list(song.return_tracks):
                    if other != track and other.solo:
                        other.solo = False
            track.solo = bool(value)
        elif prop == 'arm':
            if not track.can_be_armed:
                raise CommandError('track cannot be armed')
            if value and args.get('exclusive'):
                for other in song.tracks:
                    if other != track and other.can_be_armed and other.arm:
                        other.arm = False
            track.arm = bool(value)
        elif prop == 'monitor':
            if value not in state.MONITOR_NAMES:
                raise CommandError('monitor must be one of %s' % (state.MONITOR_NAMES,))
            track.current_monitoring_state = state.MONITOR_NAMES.index(value)
        elif prop == 'folded':
            if not track.is_foldable:
                raise CommandError('track is not a group')
            track.fold_state = bool(value)
        elif prop == 'name':
            track.name = str(value)
        else:
            raise CommandError('track property %r is not settable' % (prop,))

    def cmd_select_track(self, args):
        self._song().view.selected_track = self._track(args)

    def cmd_stop_track(self, args):
        self._track(args).stop_all_clips()

    # ------------------------------------------------------- clips & scenes

    def cmd_fire_slot(self, args):
        self._slot(args).fire()

    def cmd_stop_slot(self, args):
        self._slot(args).stop()

    def cmd_delete_clip(self, args):
        slot = self._slot(args)
        if slot.has_clip:
            slot.delete_clip()

    def cmd_fire_scene(self, args):
        scene = self._scene(args)
        scene.fire()

    def cmd_select_scene(self, args):
        self._song().view.selected_scene = self._scene(args)

    def cmd_record_into_track(self, args):
        """Remote-recording helper: arms the track (exclusively by default),
        selects it and fires its first empty slot at or after the selected
        scene. Returns the scene index it records into."""
        song = self._song()
        track = self._track(args)
        if not track.can_be_armed or not hasattr(track, 'clip_slots'):
            raise CommandError('track cannot record clips')
        if args.get('exclusive', True):
            for other in song.tracks:
                if other != track and other.can_be_armed and other.arm:
                    other.arm = False
        track.arm = True
        song.view.selected_track = track
        start = args.get('scene')
        if start is None:
            start = state.index_of(song.scenes, song.view.selected_scene) or 0
        slots = list(track.clip_slots)
        for index in range(int(start), len(slots)):
            if not slots[index].has_clip:
                break
        else:
            song.create_scene(-1)
            index = len(song.scenes) - 1
        track.clip_slots[index].fire()
        song.view.selected_scene = song.scenes[index]
        return {'scene': index}

    # --------------------------------------------------------------- helpers

    def _track(self, args):
        try:
            return state.resolve_track(self._song(), args.get('track'))
        except ValueError as e:
            raise CommandError(str(e))

    def _slot(self, args):
        track = self._track(args)
        slots = getattr(track, 'clip_slots', ())
        index = int(args.get('scene', -1))
        if not 0 <= index < len(slots):
            raise CommandError('no clip slot %r on %s' % (index, args.get('track')))
        return slots[index]

    def _scene(self, args):
        scenes = self._song().scenes
        index = int(args.get('scene', -1))
        if not 0 <= index < len(scenes):
            raise CommandError('no scene %r' % (index,))
        return scenes[index]


def _set_param(param, value):
    value = float(value)
    param.value = max(param.min, min(param.max, value))
