import type {
  LiveState, PatchOp, ServerMessage, Tick, Track, TrackRef,
} from './protocol.ts';

export type { LiveState, Tick, Track, TrackRef };

const TOKEN_KEY = 'abletonweb.token';
const THROTTLE_MS = 40;

function readToken(): string | null {
  const fromUrl = new URLSearchParams(location.search).get('t');
  try {
    if (fromUrl) localStorage.setItem(TOKEN_KEY, fromUrl);
    return fromUrl ?? localStorage.getItem(TOKEN_KEY);
  } catch {
    return fromUrl;
  }
}

function applyPatch(root: unknown, ops: PatchOp[]): unknown {
  for (const [path, value] of ops) {
    if (path.length === 0) {
      root = value;
      continue;
    }
    let node = root as Record<string | number, unknown>;
    for (let i = 0; i < path.length - 1; i++) node = node[path[i]] as Record<string | number, unknown>;
    node[path[path.length - 1]] = value;
  }
  return root;
}

interface Pending {
  resolve: (v: unknown) => void;
  reject: (e: Error) => void;
}

class LiveClient {
  socket = $state<'connecting' | 'open' | 'closed' | 'unauthorized'>('connecting');
  liveConnected = $state(false);
  version = $state('');
  state = $state<LiveState | null>(null);
  tick = $state.raw<Tick | null>(null);
  error = $state<string | null>(null);
  token = $state(readToken());

  private ws: WebSocket | null = null;
  private nextId = 1;
  private pending = new Map<number, Pending>();
  private throttled = new Map<string, { timer: number | null; last: number; args: () => [string, Record<string, unknown>] }>();
  private retry = 0;
  private errorTimer = 0;

  connect(): void {
    if (this.ws && this.ws.readyState <= WebSocket.OPEN) return;
    this.socket = 'connecting';
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    const query = this.token ? `?t=${encodeURIComponent(this.token)}` : '';
    const ws = new WebSocket(`${proto}://${location.host}/ws${query}`);
    this.ws = ws;
    let opened = false;
    ws.onopen = () => {
      opened = true;
      this.retry = 0;
      this.socket = 'open';
    };
    ws.onmessage = (ev) => this.onMessage(JSON.parse(ev.data));
    ws.onclose = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      this.liveConnected = false;
      for (const p of this.pending.values()) p.reject(new Error('disconnected'));
      this.pending.clear();
      // A rejected upgrade never opens; after a few tries assume the bridge
      // requires a token we don't have (or have wrong).
      if (!opened && ++this.retry >= 3) {
        this.socket = 'unauthorized';
        return;
      }
      this.socket = 'closed';
      setTimeout(() => this.connect(), Math.min(5000, 500 * (this.retry + 1)));
    };
  }

  setToken(token: string): void {
    try {
      localStorage.setItem(TOKEN_KEY, token);
    } catch { /* private mode */ }
    this.token = token;
    this.retry = 0;
    this.connect();
  }

  // Reconnect right away when an iOS tab comes back from the background.
  reconnectIfIdle(): void {
    if (!this.ws && this.socket !== 'unauthorized') {
      this.retry = 0;
      this.connect();
    }
  }

  cmd(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
    const ws = this.ws;
    if (!ws || ws.readyState !== WebSocket.OPEN) return Promise.reject(new Error('offline'));
    const id = this.nextId++;
    ws.send(JSON.stringify({ id, cmd, args }));
    const p = new Promise<unknown>((resolve, reject) => this.pending.set(id, { resolve, reject }));
    p.catch((e: Error) => this.showError(e.message));
    return p;
  }

  // For continuous controls (faders): at most one send per THROTTLE_MS per
  // key, always delivering the latest value.
  cmdThrottled(key: string, cmd: string, args: Record<string, unknown>): void {
    let entry = this.throttled.get(key);
    if (!entry) {
      entry = { timer: null, last: 0, args: () => [cmd, args] };
      this.throttled.set(key, entry);
    }
    entry.args = () => [cmd, args];
    if (entry.timer !== null) return;
    const fire = () => {
      entry!.timer = null;
      entry!.last = performance.now();
      const [c, a] = entry!.args();
      this.cmd(c, a).catch(() => {});
    };
    const wait = THROTTLE_MS - (performance.now() - entry.last);
    if (wait <= 0) fire();
    else entry.timer = window.setTimeout(fire, wait);
  }

  track(ref: TrackRef | null | undefined): Track | null {
    const s = this.state;
    if (!s || !ref) return null;
    if (ref === 'm') return s.master;
    const list = ref[0] === 't' ? s.tracks : s.returns;
    return list[Number(ref.slice(1))] ?? null;
  }

  meter(ref: TrackRef): [number, number] {
    const tk = this.tick;
    if (!tk) return [0, 0];
    if (ref === 'm') return tk.masterMeter;
    const i = Number(ref.slice(1));
    return (ref[0] === 't' ? tk.meters[i] : tk.returnMeters[i]) ?? [0, 0];
  }

  private showError(message: string): void {
    this.error = message;
    clearTimeout(this.errorTimer);
    this.errorTimer = window.setTimeout(() => (this.error = null), 4000);
  }

  private onMessage(msg: ServerMessage): void {
    switch (msg.t) {
      case 'live':
        this.liveConnected = msg.connected;
        this.version = msg.version ?? '';
        if (!msg.connected) this.state = null;
        break;
      case 'state':
        this.liveConnected = true;
        this.state = msg.state;
        break;
      case 'patch':
        if (this.state) this.state = applyPatch(this.state, msg.ops) as LiveState;
        break;
      case 'tick': {
        const { t: _, ...tick } = msg;
        this.tick = tick;
        break;
      }
      case 'reply': {
        const p = this.pending.get(msg.id);
        if (!p) return;
        this.pending.delete(msg.id);
        if (msg.ok) p.resolve(msg.result);
        else p.reject(new Error(msg.error ?? 'error'));
        break;
      }
    }
  }
}

export const live = new LiveClient();
