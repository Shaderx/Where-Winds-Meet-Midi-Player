<script>
  import Icon from "@iconify/svelte";
  import { fade, fly } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import {
    noteMode,
    setNoteMode,
    smartPause,
  } from "../stores/player.js";

  let isTesting = false;

  // Note calculation mode options
  const noteModes = [
    {
      id: "Closest",
      name: "Closest",
      description: "Find closest available note (original, best for most songs)",
    },
    {
      id: "Quantize",
      name: "Quantize",
      description: "Snap to exact scale notes only",
    },
    {
      id: "TransposeOnly",
      name: "Transpose Only",
      description: "Direct mapping with octave shifting",
    },
    {
      id: "Pentatonic",
      name: "Pentatonic",
      description: "Map to 5-note pentatonic scale (do-re-mi-so-la)",
    },
    {
      id: "Chromatic",
      name: "Chromatic",
      description: "Detailed 12-semitone to 7-key mapping",
    },
    {
      id: "Raw",
      name: "Raw",
      description: "Direct 1:1 mapping, no auto-transpose (MIDI note % 21)",
    },
  ];

  async function handleModeChange(mode) {
    await setNoteMode(mode);
  }

  function toggleSmartPause() {
    smartPause.update((v) => !v);
  }

  async function testAllKeys() {
    if (isTesting) return;
    isTesting = true;
    try {
      await invoke("test_all_keys");
    } catch (error) {
      console.error("Failed to test keys:", error);
    } finally {
      isTesting = false;
    }
  }
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="mb-6">
    <div class="flex items-center gap-4">
      <div>
        <h2 class="text-2xl font-bold">Settings</h2>
        <p class="text-sm text-white/60">Configure your playback preferences</p>
      </div>
    </div>
  </div>

  <!-- Settings Sections -->
  <div class="flex-1 overflow-y-auto space-y-6 pr-2">
    <!-- Note Mode Section -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 50 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:music-note" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Note Calculation Mode</h3>
      </div>

      <p class="text-sm text-white/60 mb-4">
        Choose how MIDI notes are mapped to game keys
      </p>

      <div class="space-y-3">
        {#each noteModes as mode}
          <button
            class="w-full p-4 rounded-lg border-2 transition-all duration-200 text-left {$noteMode ===
            mode.id
              ? 'border-[#1db954] bg-[#1db954]/10'
              : 'border-white/10 bg-white/5 hover:border-white/20'}"
            onclick={() => handleModeChange(mode.id)}
          >
            <div class="flex items-center justify-between mb-2">
              <span class="font-semibold text-white">{mode.name}</span>
              {#if $noteMode === mode.id}
                <Icon icon="mdi:check-circle" class="w-5 h-5 text-[#1db954]" />
              {:else}
                <div
                  class="w-5 h-5 rounded-full border-2 border-white/30"
                ></div>
              {/if}
            </div>
            <p class="text-sm text-white/60">{mode.description}</p>
          </button>
        {/each}
      </div>

      <!-- Test All Keys Button -->
      <div class="mt-4 pt-4 border-t border-white/10">
        <button
          class="w-full py-3 px-4 rounded-lg bg-white/10 hover:bg-white/15 transition-colors flex items-center justify-center gap-2 {isTesting
            ? 'opacity-50 cursor-not-allowed'
            : ''}"
          onclick={testAllKeys}
          disabled={isTesting}
        >
          <Icon
            icon={isTesting ? "mdi:loading" : "mdi:piano"}
            class="w-5 h-5 {isTesting ? 'animate-spin' : ''}"
          />
          <span class="font-medium"
            >{isTesting ? "Testing..." : "Test All Keys"}</span
          >
        </button>
        <p class="text-xs text-white/40 mt-2 text-center">
          Plays all 21 keys. Focus game window first.
        </p>
      </div>
    </div>

    <!-- Keyboard Layout Info -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 100 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:keyboard" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Keyboard Layout</h3>
      </div>

      <div class="space-y-3 text-sm">
        <div class="bg-white/5 rounded-lg p-3">
          <p class="font-semibold text-white mb-2">21 Keys (3 Octaves)</p>
          <div class="grid grid-cols-3 gap-2 text-xs">
            <div>
              <span class="text-white/40">High:</span>
              <span class="font-mono">Q W E R T Y U</span>
            </div>
            <div>
              <span class="text-white/40">Mid:</span>
              <span class="font-mono">A S D F G H J</span>
            </div>
            <div>
              <span class="text-white/40">Low:</span>
              <span class="font-mono">Z X C V B N M</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Playback Settings Section -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 150 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:play-circle-outline" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">Playback Settings</h3>
      </div>

      <!-- Smart Pause Toggle -->
      <div class="flex items-center justify-between py-3">
        <div>
          <p class="font-medium text-white">Smart Pause</p>
          <p class="text-sm text-white/60">Auto-pause when game loses focus</p>
        </div>
        <button
          class="relative w-12 h-6 rounded-full transition-colors duration-200 {$smartPause
            ? 'bg-[#1db954]'
            : 'bg-white/20'}"
          onclick={toggleSmartPause}
        >
          <div
            class="absolute top-1 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {$smartPause
              ? 'translate-x-7'
              : 'translate-x-1'}"
          ></div>
        </button>
      </div>
    </div>

    <!-- About Section -->
    <div
      class="bg-white/5 rounded-xl p-4"
      in:fly={{ y: 10, duration: 200, delay: 200 }}
    >
      <div class="flex items-center gap-2 mb-4">
        <Icon icon="mdi:information-outline" class="w-5 h-5 text-[#1db954]" />
        <h3 class="text-lg font-semibold">About</h3>
      </div>

      <div class="text-sm text-white/60 space-y-2">
        <p>Midi Player for Where Winds Meet</p>
        <p class="text-xs text-white/40">By YueLyn</p>
      </div>
    </div>
  </div>
</div>
