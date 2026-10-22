<script lang="ts">
  // Touch-friendly fader with relative dragging, so touching it never makes
  // the value jump. Double-tap resets to `defaultValue`.
  let {
    value,
    min = 0,
    max = 1,
    defaultValue,
    vertical = true,
    bipolar = false,
    color = 'var(--accent)',
    onchange,
  }: {
    value: number;
    min?: number;
    max?: number;
    defaultValue?: number;
    vertical?: boolean;
    bipolar?: boolean;
    color?: string;
    onchange: (v: number) => void;
  } = $props();

  let el: HTMLDivElement;
  let dragging = $state(false);
  let local = $state(0);
  let startPos = 0;
  let startValue = 0;
  let lastTap = 0;

  // While dragging, show our own value instead of the (laggy) echo from Live.
  const shown = $derived(dragging ? local : value);
  const frac = $derived((shown - min) / (max - min));

  function pos(e: PointerEvent): number {
    return vertical ? -e.clientY : e.clientX;
  }

  function down(e: PointerEvent) {
    e.preventDefault();
    const now = performance.now();
    if (defaultValue !== undefined && now - lastTap < 300) {
      onchange(defaultValue);
      lastTap = 0;
      return;
    }
    lastTap = now;
    el.setPointerCapture(e.pointerId);
    dragging = true;
    local = value;
    startPos = pos(e);
    startValue = value;
  }

  function move(e: PointerEvent) {
    if (!dragging) return;
    const rect = el.getBoundingClientRect();
    const length = vertical ? rect.height : rect.width;
    // Shift (desktop) gives fine control.
    const scale = e.shiftKey ? 0.2 : 1;
    const delta = ((pos(e) - startPos) / length) * (max - min) * scale;
    const next = Math.max(min, Math.min(max, startValue + delta));
    if (next !== local) {
      local = next;
      onchange(next);
    }
  }

  function up() {
    dragging = false;
  }
</script>

<div
  bind:this={el}
  class="fader"
  class:vertical
  class:dragging
  role="slider"
  tabindex="-1"
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={shown}
  style:--f={frac}
  style:--c={color}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
>
  <div class="track">
    {#if bipolar}
      <div class="bi" style:--a={Math.min(frac, 0.5)} style:--b={Math.max(frac, 0.5)}></div>
    {:else}
      <div class="fill"></div>
    {/if}
  </div>
  <div class="thumb"></div>
</div>

<style>
  .fader {
    position: relative;
    touch-action: none;
    cursor: ew-resize;
    height: 28px;
    width: 100%;
  }
  .fader.vertical {
    cursor: ns-resize;
    width: 28px;
    height: 100%;
  }
  .track {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 6px;
    margin-top: -3px;
    background: var(--meter-bg);
    border-radius: 3px;
    overflow: hidden;
  }
  .vertical .track {
    top: 0;
    bottom: 0;
    left: 50%;
    width: 6px;
    height: auto;
    margin: 0 0 0 -3px;
  }
  .fill {
    position: absolute;
    inset: 0;
    background: var(--c);
    transform-origin: left center;
    transform: scaleX(var(--f));
  }
  .vertical .fill {
    transform-origin: center bottom;
    transform: scaleY(var(--f));
  }
  .bi {
    position: absolute;
    top: 0;
    bottom: 0;
    left: calc(var(--a) * 100%);
    right: calc((1 - var(--b)) * 100%);
    background: var(--c);
  }
  .thumb {
    position: absolute;
    top: 50%;
    left: calc(var(--f) * (100% - 14px));
    width: 14px;
    height: 22px;
    margin-top: -11px;
    background: var(--thumb);
    border-radius: 3px;
    box-shadow: 0 1px 3px #0008;
  }
  .vertical .thumb {
    left: 50%;
    top: auto;
    bottom: calc(var(--f) * (100% - 14px));
    width: 24px;
    height: 14px;
    margin: 0 0 0 -12px;
  }
  .dragging .thumb {
    background: var(--accent);
  }
</style>
