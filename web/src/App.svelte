<script lang="ts">
  import { onMount } from 'svelte';
  import ArrangementView from './lib/ArrangementView.svelte';
  import { live } from './lib/live.svelte.ts';
  import PhoneMixer from './lib/PhoneMixer.svelte';
  import RecordView from './lib/RecordView.svelte';
  import SessionView from './lib/SessionView.svelte';
  import Transport from './lib/Transport.svelte';
  import type { View } from './lib/util.ts';

  const VIEW_KEY = 'abletonweb.view';
  const phoneQuery = matchMedia('(max-width: 640px)');
  let phone = $state(phoneQuery.matches);
  let view = $state<View>(initialView());
  let tokenInput = $state('');

  const views = $derived<{ id: View; label: string }[]>(phone
    ? [{ id: 'record', label: 'Record' }, { id: 'mixer', label: 'Mixer' }]
    : [{ id: 'session', label: 'Session' }, { id: 'arrangement', label: 'Arrangement' }, { id: 'record', label: 'Record' }]);

  function initialView(): View {
    try {
      const v = localStorage.getItem(VIEW_KEY) as View | null;
      if (v) return v;
    } catch { /* ignore */ }
    return phoneQuery.matches ? 'record' : 'session';
  }

  // Keep the chosen view valid for the current screen size.
  $effect(() => {
    if (!views.some((v) => v.id === view)) view = views[0].id;
    try {
      localStorage.setItem(VIEW_KEY, view);
    } catch { /* ignore */ }
  });

  let wakeLock: WakeLockSentinel | null = null;
  async function keepAwake() {
    // Needs a secure context (https or localhost); silently skipped otherwise.
    if (document.visibilityState !== 'visible' || wakeLock || !('wakeLock' in navigator)) return;
    try {
      wakeLock = await navigator.wakeLock.request('screen');
      wakeLock.addEventListener('release', () => (wakeLock = null));
    } catch { /* not allowed */ }
  }

  onMount(() => {
    live.connect();
    const onChange = () => (phone = phoneQuery.matches);
    phoneQuery.addEventListener('change', onChange);
    const onVisible = () => {
      if (document.visibilityState === 'visible') {
        live.reconnectIfIdle();
        keepAwake();
      }
    };
    document.addEventListener('visibilitychange', onVisible);
    document.addEventListener('pointerdown', keepAwake, { once: true });
    return () => {
      phoneQuery.removeEventListener('change', onChange);
      document.removeEventListener('visibilitychange', onVisible);
    };
  });
</script>

<div class="app">
  {#if live.socket === 'unauthorized'}
    <form class="center" onsubmit={(e) => { e.preventDefault(); live.setToken(tokenInput.trim()); }}>
      <h1>Live Remote</h1>
      <p>Open the address shown in the Live Remote app (or scan its QR code), or paste the token:</p>
      <input bind:value={tokenInput} placeholder="token" autocapitalize="off" autocomplete="off" />
      <button type="submit">Connect</button>
    </form>
  {:else}
    <Transport bind:view {views} compact={phone} />
    <main>
      {#if !live.state}
        <div class="center">
          {#if live.socket !== 'open'}
            <p>Connecting to the bridge…</p>
          {:else}
            <p>Bridge connected. Waiting for Live…</p>
            <p class="hint">Make sure <b>LiveWeb</b> is selected as Control Surface in Live's Settings → Link, Tempo &amp; MIDI.</p>
          {/if}
        </div>
      {:else if view === 'session'}
        <SessionView data={live.state} />
      {:else if view === 'arrangement'}
        <ArrangementView data={live.state} />
      {:else if view === 'record'}
        <RecordView data={live.state} />
      {:else}
        <PhoneMixer data={live.state} />
      {/if}
    </main>
  {/if}
  {#if live.error}
    <div class="toast">{live.error}</div>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100dvh;
  }
  main {
    flex: 1;
    min-height: 0;
  }
  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    padding: 24px 16px;
    text-align: center;
    color: var(--muted);
  }
  .center input {
    width: min(320px, 100%);
    padding: 10px;
    font-size: 16px;
    border-radius: 6px;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--text);
  }
  .hint {
    font-size: 13px;
  }
  .toast {
    position: fixed;
    left: 50%;
    bottom: max(20px, env(safe-area-inset-bottom));
    transform: translateX(-50%);
    background: var(--rec);
    color: #fff;
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 13px;
    max-width: calc(100% - 32px);
  }
</style>
