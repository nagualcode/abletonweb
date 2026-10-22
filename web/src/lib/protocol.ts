// Shapes exchanged with the LiveWeb remote script and relayed by the bridge
// (app/src/bridge.rs). Mirrors remote_script/LiveWeb/state.py.

export type TrackRef = string; // "t<i>", "r<i>" or "m"
export type SlotState = 'empty' | 'stopped' | 'playing' | 'triggered' | 'recording';

export interface ClipInfo {
  name: string;
  color: string | null;
  muted: boolean;
  length: number;
}

export interface Slot {
  clip: ClipInfo | null;
  group?: boolean;
  state: SlotState;
  hasStop: boolean;
}

export interface ArrangementClip {
  name: string;
  color: string | null;
  start: number;
  end: number;
}

export interface Track {
  ref: TrackRef;
  kind: 'audio' | 'midi' | 'group' | 'return' | 'master';
  name: string;
  color: string | null;
  mute: boolean | null;
  solo: boolean | null;
  mutedViaSolo: boolean;
  canArm: boolean;
  arm: boolean;
  monitor: 'in' | 'auto' | 'off' | null;
  input: string | null;
  volume: number;
  volumeStr: string;
  pan: number;
  panStr: string;
  sends: { value: number; str: string }[];
  playingSlot: number;
  firedSlot: number;
  isFoldable: boolean;
  folded: boolean;
  isVisible: boolean;
  group: TrackRef | null;
  slots: Slot[];
  arrangementClips: ArrangementClip[];
}

export interface SongState {
  isPlaying: boolean;
  recordMode: boolean;
  sessionRecord: boolean;
  sessionRecordStatus: number;
  metronome: boolean;
  tempo: number;
  arrangementOverdub: boolean;
  signatureNumerator: number;
  signatureDenominator: number;
  loop: boolean;
  loopStart: number;
  loopLength: number;
  backToArranger: boolean;
  isCountingIn: boolean | null;
  sessionAutomationRecord: boolean;
  canCaptureMidi: boolean | null;
  songLength: number;
  canUndo: boolean;
  canRedo: boolean;
  selectedTrack: TrackRef | null;
  selectedScene: number | null;
}

export interface Scene {
  name: string;
  color: string | null;
  triggered: boolean;
}

export interface LiveState {
  song: SongState;
  tracks: Track[];
  returns: Track[];
  master: Track;
  scenes: Scene[];
}

export type Meter = [number, number];

export interface Tick {
  time: number;
  meters: Meter[];
  returnMeters: Meter[];
  masterMeter: Meter;
  inputMeters: Record<TrackRef, Meter>;
  clipPos: Record<TrackRef, number>;
}

export type LiveMessage =
  | { t: 'hello'; protocol: number; live: string }
  | { t: 'state'; state: LiveState }
  | ({ t: 'tick' } & Tick)
  | { t: 'reply'; id: number; ok: true; result: unknown }
  | { t: 'reply'; id: number; ok: false; error: string };

// A patch op: replace the value at `path` (array of keys/indices).
export type PatchOp = [path: (string | number)[], value: unknown];

// Bridge <-> web client messages.
export type ServerMessage =
  | { t: 'live'; connected: boolean; version?: string }
  | { t: 'state'; state: LiveState }
  | { t: 'patch'; ops: PatchOp[] }
  | ({ t: 'tick' } & Tick)
  | { t: 'reply'; id: number; ok: boolean; result?: unknown; error?: string };

export interface ClientCommand {
  id: number;
  cmd: string;
  args?: Record<string, unknown>;
}
