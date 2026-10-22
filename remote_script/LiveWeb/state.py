"""Reading Live's state: full snapshots, fast tick data and listener binding.

Tracks are addressed by refs: "t<i>" (song.tracks), "r<i>" (return_tracks)
and "m" (master_track).
"""
from __future__ import absolute_import

SONG_STATE_PROPS = (
    'is_playing', 'record_mode', 'session_record', 'session_record_status',
    'metronome', 'tempo', 'arrangement_overdub', 'signature_numerator',
    'signature_denominator', 'loop', 'loop_start', 'loop_length',
    'back_to_arranger', 'is_counting_in', 'session_automation_record',
    'can_capture_midi', 'song_length',
)
SONG_STRUCTURE_PROPS = ('tracks', 'return_tracks', 'scenes', 'visible_tracks')
TRACK_STATE_PROPS = (
    'name', 'color', 'mute', 'solo', 'arm', 'current_monitoring_state',
    'playing_slot_index', 'fired_slot_index', 'fold_state', 'muted_via_solo',
    'arrangement_clips', 'input_routing_type',
)
# Live raises when listening to these on return/master tracks.
TRACK_ONLY_PROPS = ('playing_slot_index', 'fired_slot_index', 'arrangement_clips')
MASTER_MISSING_PROPS = ('mute', 'solo', 'muted_via_solo')
SLOT_STATE_PROPS =('playing_status', 'is_triggered', 'has_stop_button')
CLIP_STATE_PROPS = ('name', 'color', 'playing_status', 'muted')
SCENE_STATE_PROPS = ('name', 'color', 'is_triggered')

MONITOR_NAMES = ('in', 'auto', 'off')


def get(obj, attr, default=None):
    try:
        return getattr(obj, attr)
    except Exception:
        return default


def color_hex(value):
    if value is None:
        return None
    return '#%06x' % (int(value) & 0xFFFFFF)


def track_refs(song):
    """Yields (ref, track) for every addressable track."""
    for i, t in enumerate(song.tracks):
        yield 't%d' % i, t
    for i, t in enumerate(song.return_tracks):
        yield 'r%d' % i, t
    yield 'm', song.master_track


def resolve_track(song, ref):
    if ref == 'm':
        return song.master_track
    if not isinstance(ref, str) or len(ref) < 2 or ref[0] not in 'tr':
        raise ValueError('bad track ref %r' % (ref,))
    index = int(ref[1:])
    tracks = song.tracks if ref[0] == 't' else song.return_tracks
    if not 0 <= index < len(tracks):
        raise ValueError('no track %s' % ref)
    return tracks[index]


# ---------------------------------------------------------------- snapshot

def snapshot(song):
    tracks = list(song.tracks)
    return {
        'song': _song(song, tracks),
        'tracks': [_track(song, 't%d' % i, t, tracks) for i, t in enumerate(tracks)],
        'returns': [_track(song, 'r%d' % i, t, tracks)
                    for i, t in enumerate(song.return_tracks)],
        'master': _track(song, 'm', song.master_track, tracks),
        'scenes': [_scene(s) for s in song.scenes],
    }


def _song(song, tracks):
    s = {}
    for prop in SONG_STATE_PROPS:
        s[_camel(prop)] = get(song, prop)
    s['canUndo'] = get(song, 'can_undo', False)
    s['canRedo'] = get(song, 'can_redo', False)
    view = song.view
    s['selectedTrack'] = _ref_of(song, get(view, 'selected_track'))
    s['selectedScene'] = index_of(song.scenes, get(view, 'selected_scene'))
    return s


def _track(song, ref, track, tracks):
    mixer = track.mixer_device
    volume = mixer.volume
    pan = mixer.panning
    is_master = ref == 'm'
    can_arm = bool(get(track, 'can_be_armed', False))
    data = {
        'ref': ref,
        'kind': _kind(ref, track),
        'name': get(track, 'name', ''),
        'color': color_hex(get(track, 'color')),
        'mute': None if is_master else get(track, 'mute'),
        'solo': None if is_master else get(track, 'solo'),
        'mutedViaSolo': get(track, 'muted_via_solo', False),
        'canArm': can_arm,
        'arm': get(track, 'arm', False) if can_arm else False,
        'monitor': (MONITOR_NAMES[get(track, 'current_monitoring_state', 1)]
                    if can_arm else None),
        'input': _routing_name(get(track, 'input_routing_type')) if can_arm else None,
        'volume': volume.value,
        'volumeStr': _param_str(volume),
        'pan': pan.value,
        'panStr': _param_str(pan),
        'sends': [{'value': p.value, 'str': _param_str(p)}
                  for p in get(mixer, 'sends', ())],
        'playingSlot': get(track, 'playing_slot_index', -1),
        'firedSlot': get(track, 'fired_slot_index', -1),
        'isFoldable': get(track, 'is_foldable', False),
        'folded': bool(get(track, 'fold_state', 0)) if get(track, 'is_foldable', False) else False,
        'isVisible': get(track, 'is_visible', True),
        'group': None,
        'slots': [],
        'arrangementClips': [],
    }
    if ref[0] == 't':
        if get(track, 'is_grouped', False):
            data['group'] = _ref_of(song, track.group_track)
        data['slots'] = [_slot(s) for s in track.clip_slots]
        data['arrangementClips'] = [_arr_clip(c) for c in get(track, 'arrangement_clips', ())]
    return data


def _kind(ref, track):
    if ref == 'm':
        return 'master'
    if ref[0] == 'r':
        return 'return'
    if get(track, 'is_foldable', False):
        return 'group'
    if get(track, 'has_midi_input', False):
        return 'midi'
    return 'audio'


def _slot(slot):
    if slot.has_clip:
        clip = slot.clip
        if get(clip, 'is_recording', False):
            state = 'recording'
        elif get(clip, 'is_triggered', False):
            state = 'triggered'
        elif get(clip, 'is_playing', False):
            state = 'playing'
        else:
            state = 'stopped'
        return {
            'clip': {'name': get(clip, 'name', ''), 'color': color_hex(get(clip, 'color')),
                     'muted': get(clip, 'muted', False), 'length': get(clip, 'length', 0)},
            'state': state,
            'hasStop': get(slot, 'has_stop_button', True),
        }
    if get(slot, 'controls_other_clips', False):
        # Group track slot: playing_status 0 stopped, 1 playing, 2 recording.
        status = get(slot, 'playing_status', 0)
        state = ('stopped', 'playing', 'recording')[status] if status in (0, 1, 2) else 'stopped'
        if get(slot, 'is_triggered', False):
            state = 'triggered'
        return {'clip': None, 'group': True, 'state': state,
                'hasStop': get(slot, 'has_stop_button', True)}
    return {'clip': None,
            'state': 'triggered' if get(slot, 'is_triggered', False) else 'empty',
            'hasStop': get(slot, 'has_stop_button', True)}


def _arr_clip(clip):
    return {'name': get(clip, 'name', ''), 'color': color_hex(get(clip, 'color')),
            'start': get(clip, 'start_time', 0), 'end': get(clip, 'end_time', 0)}


def _scene(scene):
    return {'name': get(scene, 'name', ''), 'color': color_hex(get(scene, 'color')),
            'triggered': get(scene, 'is_triggered', False)}


def _param_str(param):
    try:
        return param.str_for_value(param.value)
    except Exception:
        return str(param)


def _routing_name(routing):
    return get(routing, 'display_name') if routing is not None else None


def _ref_of(song, track):
    if track is None:
        return None
    for ref, t in track_refs(song):
        if t == track:
            return ref
    return None


def index_of(seq, item):
    if item is None:
        return None
    for i, x in enumerate(seq):
        if x == item:
            return i
    return None


def _camel(name):
    head, _, rest = name.partition('_')
    return head + ''.join(w.title() for w in rest.split('_')) if rest else head


# -------------------------------------------------------------- fast tick

def tick(song):
    """Data that changes continuously and is sent every update cycle."""
    tracks = list(song.tracks)
    returns = list(song.return_tracks)
    master = song.master_track
    return {
        'time': round(song.current_song_time, 3),
        'meters': [_meter(t) for t in tracks],
        'returnMeters': [_meter(t) for t in returns],
        'masterMeter': _meter(master),
        'inputMeters': dict(('t%d' % i, _input_meter(t)) for i, t in enumerate(tracks)
                            if get(t, 'arm', False)),
        'clipPos': dict(('t%d' % i, p) for i, p in
                        ((i, _clip_pos(t)) for i, t in enumerate(tracks)) if p is not None),
    }


def _meter(track):
    return [round(get(track, 'output_meter_left', 0.0), 3),
            round(get(track, 'output_meter_right', 0.0), 3)]


def _input_meter(track):
    return [round(get(track, 'input_meter_left', 0.0), 3),
            round(get(track, 'input_meter_right', 0.0), 3)]


def _clip_pos(track):
    index = get(track, 'playing_slot_index', -1)
    if index is None or index < 0:
        return None
    try:
        clip = track.clip_slots[index].clip
        if clip is None or not (clip.is_playing or clip.is_recording):
            return None
        start, end = clip.loop_start, clip.loop_end
        if end <= start:
            return None
        return round((clip.playing_position - start) / (end - start), 3)
    except Exception:
        return None


# ---------------------------------------------------------------- binding

def bind(song, registry, on_structure, on_state):
    """Attaches listeners: structural changes require a rebind, value
    changes only a new snapshot."""
    for prop in SONG_STRUCTURE_PROPS:
        registry.add(song, prop, on_structure)
    for prop in SONG_STATE_PROPS:
        registry.add(song, prop, on_state)
    registry.add(song.view, 'selected_track', on_state)
    registry.add(song.view, 'selected_scene', on_state)

    for ref, track in track_refs(song):
        is_song_track = ref[0] == 't'
        for prop in TRACK_STATE_PROPS:
            if prop in ('arm', 'current_monitoring_state', 'input_routing_type') \
                    and not get(track, 'can_be_armed', False):
                continue
            if prop == 'fold_state' and not get(track, 'is_foldable', False):
                continue
            if prop in TRACK_ONLY_PROPS and not is_song_track:
                continue
            if prop == 'arrangement_clips' and get(track, 'is_foldable', False):
                continue
            if prop in MASTER_MISSING_PROPS and ref == 'm':
                continue
            registry.add(track, prop, on_state)
        mixer = track.mixer_device
        registry.add(mixer.volume, 'value', on_state)
        registry.add(mixer.panning, 'value', on_state)
        if ref != 'm':
            registry.add(mixer, 'sends', on_structure)
            for send in get(mixer, 'sends', ()):
                registry.add(send, 'value', on_state)
        if not is_song_track:
            continue
        registry.add(track, 'clip_slots', on_structure)
        registry.add(track, 'is_visible', on_state)
        for slot in track.clip_slots:
            registry.add(slot, 'has_clip', on_structure)
            for prop in SLOT_STATE_PROPS:
                registry.add(slot, prop, on_state)
            if slot.has_clip:
                for prop in CLIP_STATE_PROPS:
                    registry.add(slot.clip, prop, on_state)

    for scene in song.scenes:
        for prop in SCENE_STATE_PROPS:
            registry.add(scene, prop, on_state)
