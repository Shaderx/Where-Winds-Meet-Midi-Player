<script>
  import Icon from "@iconify/svelte";
  import { fly, fade } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { t } from "svelte-i18n";

  // Auto clicker state
  let isEnabled = false;
  let key = "f";
  let delayMs = 100;
  let hotkey = "F8";
  let showSettings = false;
  let isRecordingHotkey = false;

  // Cleanup
  let unlistenToggle = null;
  let unlistenKeyCapture = null;

  onMount(async () => {
    // Load current config
    try {
      const config = await invoke('auto_clicker_get_config');
      key = config.key;
      delayMs = config.delay_ms;
      isEnabled = config.enabled;
      hotkey = config.hotkey;
    } catch (e) {
      console.error('Failed to load auto clicker config:', e);
    }

    // Listen for hotkey toggle events from backend
    unlistenToggle = await listen('auto-clicker-toggled', (event) => {
      isEnabled = event.payload;
    });

    // Listen for key capture (for hotkey recording)
    unlistenKeyCapture = await listen('key-captured', (event) => {
      if (isRecordingHotkey) {
        const keyName = event.payload;
        if (keyName === 'Escape') {
          stopRecordingHotkey();
        } else {
          applyHotkey(keyName);
        }
      }
    });
  });

  onDestroy(() => {
    if (unlistenToggle) unlistenToggle();
    if (unlistenKeyCapture) unlistenKeyCapture();
  });

  async function toggle() {
    try {
      isEnabled = await invoke('auto_clicker_toggle');
    } catch (e) {
      console.error('Failed to toggle auto clicker:', e);
    }
  }

  async function updateKey(newKey) {
    const trimmed = newKey.toLowerCase().trim();
    if (trimmed.length === 0) return;
    
    try {
      await invoke('auto_clicker_set_key', { key: trimmed });
      key = trimmed;
    } catch (e) {
      console.error('Failed to set auto clicker key:', e);
    }
  }

  async function updateDelay(newDelay) {
    const parsed = parseInt(newDelay);
    if (isNaN(parsed) || parsed < 10) return;
    
    try {
      await invoke('auto_clicker_set_delay', { delayMs: parsed });
      delayMs = parsed;
    } catch (e) {
      console.error('Failed to set auto clicker delay:', e);
    }
  }

  async function startRecordingHotkey() {
    await invoke('cmd_set_keybindings_enabled', { enabled: false });
    isRecordingHotkey = true;
    await invoke('cmd_unfocus_window');
  }

  async function stopRecordingHotkey() {
    isRecordingHotkey = false;
    await invoke('cmd_set_keybindings_enabled', { enabled: true });
  }

  async function applyHotkey(keyName) {
    try {
      await invoke('auto_clicker_set_hotkey', { hotkey: keyName });
      hotkey = keyName;
    } catch (e) {
      console.error('Failed to set auto clicker hotkey:', e);
    }
    stopRecordingHotkey();
  }

  function handleKeyInput(e) {
    if (e.key === 'Enter') {
      e.target.blur();
      updateKey(e.target.value);
    }
  }

  function handleDelayInput(e) {
    if (e.key === 'Enter') {
      e.target.blur();
      updateDelay(e.target.value);
    }
  }
</script>

<!-- Compact Auto Clicker Control (for player bar) -->
<div class="relative">
  <!-- Toggle Button -->
  <button
    class="flex items-center gap-1.5 px-2 py-1 rounded-md transition-all text-xs font-medium {isEnabled 
      ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30' 
      : 'text-white/50 hover:text-white hover:bg-white/5'}"
    onclick={() => showSettings = !showSettings}
    title={$t("autoClicker.title")}
  >
    <Icon icon={isEnabled ? "mdi:cursor-default-click" : "mdi:cursor-default-click-outline"} class="w-3.5 h-3.5 {isEnabled ? 'animate-pulse' : ''}" />
    <span class="uppercase">{key}</span>
    {#if isEnabled}
      <span class="text-[10px] opacity-70">{delayMs}ms</span>
    {/if}
  </button>

  <!-- Settings Dropdown -->
  {#if showSettings}
    <button class="fixed inset-0 z-40" onclick={() => showSettings = false}></button>
    <div
      class="absolute bottom-full right-0 mb-2 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 w-64"
      in:fly={{ y: 10, duration: 150 }}
      out:fade={{ duration: 100 }}
    >
      <div class="p-3 space-y-3">
        <!-- Header with Toggle -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Icon icon="mdi:cursor-default-click" class="w-4 h-4 text-orange-400" />
            <span class="font-medium text-sm">{$t("autoClicker.title")}</span>
          </div>
          <button
            class="relative w-10 h-5 rounded-full transition-colors duration-200 {isEnabled
              ? 'bg-orange-500'
              : 'bg-white/20'}"
            onclick={toggle}
          >
            <div
              class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {isEnabled
                ? 'translate-x-5'
                : 'translate-x-0.5'}"
            ></div>
          </button>
        </div>

        <!-- Key Input -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.key")}</label>
          <input
            type="text"
            value={key}
            maxlength="1"
            class="w-full px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white text-center uppercase focus:outline-none focus:ring-1 focus:ring-orange-500"
            onkeydown={handleKeyInput}
            onblur={(e) => updateKey(e.target.value)}
          />
        </div>

        <!-- Delay Input -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.delay")}</label>
          <div class="flex items-center gap-2">
            <input
              type="number"
              value={delayMs}
              min="10"
              max="5000"
              step="10"
              class="flex-1 px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white text-center focus:outline-none focus:ring-1 focus:ring-orange-500"
              onkeydown={handleDelayInput}
              onblur={(e) => updateDelay(e.target.value)}
            />
            <span class="text-xs text-white/40">ms</span>
          </div>
        </div>

        <!-- Hotkey -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.hotkey")}</label>
          <button
            class="w-full px-2 py-1.5 rounded text-sm font-mono text-center transition-all {isRecordingHotkey 
              ? 'bg-orange-500 text-black animate-pulse' 
              : 'bg-white/10 hover:bg-white/20 text-white'}"
            onclick={startRecordingHotkey}
          >
            {isRecordingHotkey ? '...' : hotkey}
          </button>
        </div>

        <!-- Info -->
        <p class="text-xs text-white/40">
          {$t("autoClicker.description")}
        </p>
      </div>
    </div>
  {/if}
</div>
