"""A small fake of the Live Object Model, enough to drive the LiveWeb remote
script outside of Live. Observable properties fire listeners on set."""


class Observable(object):
    _observable = ()

    def __init__(self):
        object.__setattr__(self, '_listeners', {})

    def __getattr__(self, name):
        listeners = object.__getattribute__(self, '_listeners')
        for prefix, op in (('add_', 'add'), ('remove_', 'remove')):
            if name.startswith(prefix) and name.endswith('_listener'):
                prop = name[len(prefix):-len('_listener')]
                if prop in type(self)._observable:
                    def fn(cb, prop=prop, op=op):
                        lst = listeners.setdefault(prop, [])
                        if op == 'add':
                            if cb in lst:
                                raise RuntimeError('Listener already connected')
                            lst.append(cb)
                        else:
                            lst.remove(cb)
                    return fn
        if name.endswith('_has_listener'):
            prop = name[:-len('_has_listener')]
            if prop in type(self)._observable:
                return lambda cb: cb in listeners.get(prop, [])
        raise AttributeError(name)

    def __setattr__(self, name, value):
        object.__setattr__(self, name, value)
        if name in type(self)._observable:
            self.notify(name)

    def notify(self, prop):
        for cb in list(self._listeners.get(prop, [])):
            cb()

    def listener_count(self):
        return sum(len(v) for v in self._listeners.values())


class Param(Observable):
    _observable = ('value',)

    def __init__(self, value, lo, hi, kind):
        Observable.__init__(self)
        self.min, self.max, self.kind = lo, hi, kind
        self.value = value

    def str_for_value(self, v):
        if self.kind == 'vol':
            import math
            db = 20 * math.log10(v / 0.85) * 2 if v > 0 else float('-inf')
            return '%.1f dB' % db
        if self.kind == 'pan':
            return 'C' if abs(v) < 0.01 else ('%dL' % (-v * 50) if v < 0 else '%dR' % (v * 50))
        return '%.2f' % v


class Mixer(Observable):
    _observable = ('sends',)

    def __init__(self, n_sends):
        Observable.__init__(self)
        self.volume = Param(0.85, 0.0, 1.0, 'vol')
        self.panning = Param(0.0, -1.0, 1.0, 'pan')
        self.sends = [Param(0.0, 0.0, 1.0, 'send') for _ in range(n_sends)]


class Clip(Observable):
    _observable = ('name', 'color', 'playing_status', 'muted')

    def __init__(self, name, color=0xFF8800):
        Observable.__init__(self)
        self.name, self.color, self.muted = name, color, False
        self.is_playing = self.is_recording = self.is_triggered = False
        self.length = self.loop_end = 4.0
        self.loop_start = 0.0
        self.playing_position = 0.0
        self.start_time, self.end_time = 0.0, 4.0


class ClipSlot(Observable):
    _observable = ('has_clip', 'playing_status', 'is_triggered', 'has_stop_button')

    def __init__(self, track, clip=None):
        Observable.__init__(self)
        self.track = track
        self.clip = clip
        self.has_clip = clip is not None
        self.is_triggered = False
        self.has_stop_button = True
        self.playing_status = 0

    def fire(self):
        if self.clip is None:
            if self.track.arm:
                self.clip = Clip('Rec %d' % id(self))
                self.clip.is_recording = True
                self.has_clip = True
        else:
            self.clip.is_playing = True
            self.clip.notify('playing_status')
        self.track.playing_slot_index = self.track.clip_slots.index(self)
        self.track.song.start_playing()

    def stop(self):
        if self.clip:
            self.clip.is_playing = self.clip.is_recording = False
            self.clip.notify('playing_status')

    def delete_clip(self):
        self.clip = None
        self.has_clip = False


class Track(Observable):
    _observable = ('name', 'color', 'mute', 'solo', 'arm', 'current_monitoring_state',
                   'playing_slot_index', 'fired_slot_index', 'fold_state', 'muted_via_solo',
                   'arrangement_clips', 'input_routing_type', 'clip_slots', 'is_visible')

    def __init__(self, song, name, kind='audio', n_scenes=0, n_sends=0):
        Observable.__init__(self)
        self.song = song
        self.name, self.color = name, 0x3399FF
        self.mute = self.solo = self.arm = False
        self.muted_via_solo = False
        self.current_monitoring_state = 1
        self.can_be_armed = kind in ('audio', 'midi')
        self.has_midi_input = kind == 'midi'
        self.is_foldable = kind == 'group'
        self.fold_state = 0
        self.is_grouped = False
        self.group_track = None
        self.is_visible = True
        self.playing_slot_index = -1
        self.fired_slot_index = -1
        self.mixer_device = Mixer(n_sends)
        self.clip_slots = [ClipSlot(self) for _ in range(n_scenes)]
        self.arrangement_clips = []
        self.output_meter_left = self.output_meter_right = 0.0
        self.input_meter_left = self.input_meter_right = 0.0

    def stop_all_clips(self):
        for s in self.clip_slots:
            s.stop()
        self.playing_slot_index = -1


class Scene(Observable):
    _observable = ('name', 'color', 'is_triggered')

    def __init__(self, name):
        Observable.__init__(self)
        self.name, self.color, self.is_triggered = name, 0x777777, False

    def fire(self):
        pass


class SongView(Observable):
    _observable = ('selected_track', 'selected_scene')

    def __init__(self):
        Observable.__init__(self)
        self.selected_track = None
        self.selected_scene = None


class Song(Observable):
    _observable = ('tracks', 'return_tracks', 'scenes', 'visible_tracks', 'is_playing',
                   'record_mode', 'session_record', 'session_record_status', 'metronome',
                   'tempo', 'arrangement_overdub', 'signature_numerator',
                   'signature_denominator', 'loop', 'loop_start', 'loop_length',
                   'back_to_arranger', 'is_counting_in', 'session_automation_record',
                   'can_capture_midi', 'song_length')

    def __init__(self, n_tracks=4, n_scenes=4, n_returns=2):
        Observable.__init__(self)
        self.view = SongView()
        self.is_playing = False
        self.record_mode = self.session_record = self.metronome = False
        self.session_record_status = 0
        self.tempo = 120.0
        self.arrangement_overdub = False
        self.signature_numerator, self.signature_denominator = 4, 4
        self.loop, self.loop_start, self.loop_length = False, 0.0, 16.0
        self.back_to_arranger = False
        self.is_counting_in = False
        self.session_automation_record = False
        self.can_capture_midi = False
        self.song_length = 64.0
        self.can_undo = self.can_redo = False
        self.current_song_time = 0.0
        self.scenes = [Scene('Scene %d' % (i + 1)) for i in range(n_scenes)]
        self.return_tracks = [Track(self, 'Return %s' % 'ABCD'[i], 'return', 0, n_returns)
                              for i in range(n_returns)]
        self.master_track = Track(self, 'Master', 'master', 0, 0)
        kinds = ['audio', 'midi', 'group', 'audio']
        self.tracks = [Track(self, '%d-%s' % (i + 1, kinds[i % 4]), kinds[i % 4], n_scenes,
                             n_returns) for i in range(n_tracks)]
        if n_tracks >= 4:
            self.tracks[3].is_grouped = True
            self.tracks[3].group_track = self.tracks[2]
            self.tracks[0].clip_slots[0] = ClipSlot(self.tracks[0], Clip('Guitar'))
            self.tracks[0].arrangement_clips = [Clip('Take 1')]
        self.view.selected_track = self.tracks[0]
        self.view.selected_scene = self.scenes[0]

    def start_playing(self):
        self.is_playing = True

    def continue_playing(self):
        self.is_playing = True

    def stop_playing(self):
        self.is_playing = False

    def trigger_session_record(self, length=None):
        self.session_record = True

    def tap_tempo(self):
        self.tempo = self.tempo + 1

    def undo(self):
        pass

    def redo(self):
        pass

    def stop_all_clips(self, quantized=True):
        for t in self.tracks:
            t.stop_all_clips()

    def create_scene(self, index):
        self.scenes = self.scenes + [Scene('Scene %d' % (len(self.scenes) + 1))]
        for t in self.tracks:
            t.clip_slots = t.clip_slots + [ClipSlot(t)]

    def advance(self, dt):
        """Simulates playback and meters."""
        import random
        if self.is_playing:
            self.current_song_time += dt * self.tempo / 60.0
        for t in self.tracks + self.return_tracks + [self.master_track]:
            level = random.random() * 0.8 if self.is_playing else 0.0
            t.output_meter_left = level
            t.output_meter_right = level * 0.9
            t.input_meter_left = t.input_meter_right = random.random() * 0.5 if t.arm else 0.0
