<script>
  import Icon from "@iconify/svelte";
  import { fly, fade, scale } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { t } from "svelte-i18n";

  // Macro state
  let macros = [];
  let showPanel = false;
  let isRunning = false;
  let runningMacroId = null;
  
  // Editor state
  let editingMacro = null;
  let editorName = "";
  let editorScript = "";
  let editorHotkey = "";
  let isRecordingHotkey = false;
  let validationError = "";
  let actionCount = 0;
  
  // UI state
  let showEditor = false;
  let showHelp = false;

  // Cleanup
  let unlistenStarted = null;
  let unlistenStopped = null;
  let unlistenKeyCapture = null;

  async function loadMacros() {
    try {
      macros = await invoke('macro_list');
    } catch (e) {
      console.error('Failed to load macros:', e);
    }
  }

  async function checkRunning() {
    try {
      isRunning = await invoke('macro_is_running');
    } catch (e) {
      isRunning = false;
    }
  }

  onMount(async () => {
    await loadMacros();
    await checkRunning();

    // Listen for macro events
    unlistenStarted = await listen('macro-started', (event) => {
      isRunning = true;
      runningMacroId = event.payload;
    });

    unlistenStopped = await listen('macro-stopped', (event) => {
      isRunning = false;
      runningMacroId = null;
    });

    // Listen for key capture (for hotkey recording)
    unlistenKeyCapture = await listen('key-captured', (event) => {
      if (isRecordingHotkey) {
        const keyName = event.payload;
        if (keyName === 'Escape') {
          stopRecordingHotkey();
        } else {
          editorHotkey = keyName;
          stopRecordingHotkey();
        }
      }
    });
  });

  onDestroy(() => {
    if (unlistenStarted) unlistenStarted();
    if (unlistenStopped) unlistenStopped();
    if (unlistenKeyCapture) unlistenKeyCapture();
  });

  function openNewMacro() {
    editingMacro = null;
    editorName = "New Macro";
    editorScript = "# Example macro\n# Commands: key, hold, wait, repeat\n\nkey f\nwait 100\nkey g\n";
    editorHotkey = "";
    validationError = "";
    actionCount = 0;
    showEditor = true;
  }

  function openEditMacro(macro) {
    editingMacro = macro;
    editorName = macro.name;
    editorScript = macro.script;
    editorHotkey = macro.hotkey;
    validationError = "";
    validateScript();
    showEditor = true;
  }

  async function validateScript() {
    if (!editorScript.trim()) {
      validationError = "";
      actionCount = 0;
      return;
    }
    try {
      actionCount = await invoke('macro_validate', { script: editorScript });
      validationError = "";
    } catch (e) {
      validationError = e.toString();
      actionCount = 0;
    }
  }

  async function saveMacro() {
    if (!editorName.trim()) {
      validationError = "Name is required";
      return;
    }
    
    try {
      await validateScript();
      if (validationError) return;

      if (editingMacro) {
        await invoke('macro_update', {
          id: editingMacro.id,
          name: editorName,
          script: editorScript,
          hotkey: editorHotkey
        });
      } else {
        await invoke('macro_create', {
          name: editorName,
          script: editorScript,
          hotkey: editorHotkey
        });
      }
      
      await loadMacros();
      showEditor = false;
    } catch (e) {
      validationError = e.toString();
    }
  }

  async function deleteMacro(id) {
    try {
      await invoke('macro_delete', { id });
      await loadMacros();
      if (editingMacro && editingMacro.id === id) {
        showEditor = false;
      }
    } catch (e) {
      console.error('Failed to delete macro:', e);
    }
  }

  async function runMacro(id) {
    try {
      await invoke('macro_run', { id });
    } catch (e) {
      console.error('Failed to run macro:', e);
    }
  }

  async function stopMacro() {
    try {
      await invoke('macro_stop');
    } catch (e) {
      console.error('Failed to stop macro:', e);
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

  function clearHotkey() {
    editorHotkey = "";
  }

  // Debounce validation
  let validateTimeout;
  function onScriptChange() {
    clearTimeout(validateTimeout);
    validateTimeout = setTimeout(validateScript, 300);
  }
</script>

<!-- Compact Macro Control (for player bar) -->
<div class="relative">
  <!-- Toggle Button -->
  <button
    class="flex items-center gap-1.5 px-2 py-1 rounded-md transition-all text-xs font-medium {isRunning 
      ? 'bg-purple-500/20 text-purple-400 border border-purple-500/30' 
      : 'text-white/50 hover:text-white hover:bg-white/5'}"
    onclick={() => { showPanel = !showPanel; if (showPanel) loadMacros(); }}
    title={$t("macro.title")}
  >
    <Icon icon={isRunning ? "mdi:code-braces" : "mdi:code-braces-box"} class="w-3.5 h-3.5 {isRunning ? 'animate-pulse' : ''}" />
    <span class="uppercase">MCR</span>
    {#if macros.length > 0}
      <span class="text-[10px] opacity-70">{macros.length}</span>
    {/if}
  </button>

  <!-- Macro Panel Dropdown -->
  {#if showPanel}
    <button class="fixed inset-0 z-40" onclick={() => { showPanel = false; showEditor = false; showHelp = false; }}></button>
    <div
      class="absolute bottom-full right-0 mb-2 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 w-80"
      in:fly={{ y: 10, duration: 150 }}
      out:fade={{ duration: 100 }}
    >
      {#if showEditor}
        <!-- Macro Editor -->
        <div class="p-3 space-y-3" transition:fade={{ duration: 100 }}>
          <!-- Header -->
          <div class="flex items-center justify-between">
            <button
              class="flex items-center gap-1 text-white/60 hover:text-white text-xs"
              onclick={() => showEditor = false}
            >
              <Icon icon="mdi:arrow-left" class="w-4 h-4" />
              {$t("common.cancel")}
            </button>
            <span class="text-sm font-medium text-purple-400">
              {editingMacro ? $t("macro.edit") : $t("macro.create")}
            </span>
            <button
              class="p-1 rounded hover:bg-white/10 text-white/40 hover:text-white"
              onclick={() => showHelp = !showHelp}
              title={$t("macro.help")}
            >
              <Icon icon="mdi:help-circle-outline" class="w-4 h-4" />
            </button>
          </div>

          <!-- Help Panel -->
          {#if showHelp}
            <div class="p-2 rounded bg-white/5 text-xs space-y-1" transition:scale={{ duration: 100 }}>
              <p class="font-medium text-purple-400">{$t("macro.syntax")}</p>
              <p class="text-white/60"><code class="text-purple-300">key f</code> - {$t("macro.syntaxKey")}</p>
              <p class="text-white/60"><code class="text-purple-300">hold f 500</code> - {$t("macro.syntaxHold")}</p>
              <p class="text-white/60"><code class="text-purple-300">wait 100</code> - {$t("macro.syntaxWait")}</p>
              <p class="text-white/60"><code class="text-purple-300">repeat 3 {'{'}...{'}'}</code> - {$t("macro.syntaxRepeat")}</p>
              <p class="text-white/40 mt-1">{$t("macro.syntaxNote")}</p>
            </div>
          {/if}

          <!-- Name Input -->
          <div>
            <label class="text-xs text-white/50 mb-1 block">{$t("macro.name")}</label>
            <input
              type="text"
              bind:value={editorName}
              class="w-full px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white focus:outline-none focus:ring-1 focus:ring-purple-500"
              placeholder="My Macro"
            />
          </div>

          <!-- Script Editor -->
          <div>
            <div class="flex items-center justify-between mb-1">
              <label class="text-xs text-white/50">{$t("macro.script")}</label>
              {#if actionCount > 0}
                <span class="text-xs text-purple-400">{actionCount} {$t("macro.actions")}</span>
              {/if}
            </div>
            <textarea
              bind:value={editorScript}
              oninput={onScriptChange}
              class="w-full h-32 px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white font-mono focus:outline-none focus:ring-1 focus:ring-purple-500 resize-none"
              placeholder="# Enter macro commands..."
              spellcheck="false"
            ></textarea>
            {#if validationError}
              <p class="text-xs text-red-400 mt-1">{validationError}</p>
            {/if}
          </div>

          <!-- Hotkey -->
          <div>
            <label class="text-xs text-white/50 mb-1 block">{$t("macro.hotkey")} ({$t("macro.optional")})</label>
            <div class="flex gap-2">
              <button
                class="flex-1 px-2 py-1.5 rounded text-sm font-mono text-center transition-all {isRecordingHotkey 
                  ? 'bg-purple-500 text-black animate-pulse' 
                  : 'bg-white/10 hover:bg-white/20 text-white'}"
                onclick={startRecordingHotkey}
              >
                {isRecordingHotkey ? '...' : editorHotkey || $t("macro.clickToSet")}
              </button>
              {#if editorHotkey}
                <button
                  class="px-2 py-1.5 rounded bg-white/10 hover:bg-white/20 text-white/60 hover:text-white"
                  onclick={clearHotkey}
                  title={$t("common.clear")}
                >
                  <Icon icon="mdi:close" class="w-4 h-4" />
                </button>
              {/if}
            </div>
          </div>

          <!-- Actions -->
          <div class="flex gap-2 pt-1">
            {#if editingMacro}
              <button
                class="px-3 py-1.5 rounded bg-red-500/20 hover:bg-red-500/30 text-red-400 text-xs font-medium"
                onclick={() => deleteMacro(editingMacro.id)}
              >
                {$t("common.delete")}
              </button>
            {/if}
            <button
              class="flex-1 px-3 py-1.5 rounded bg-purple-500 hover:bg-purple-600 text-white text-xs font-medium disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={saveMacro}
              disabled={!!validationError || !editorName.trim()}
            >
              {$t("common.save")}
            </button>
          </div>
        </div>
      {:else}
        <!-- Macro List -->
        <div class="p-3 space-y-3">
          <!-- Header -->
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Icon icon="mdi:code-braces" class="w-4 h-4 text-purple-400" />
              <span class="font-medium text-sm">{$t("macro.title")}</span>
            </div>
            <button
              class="p-1 rounded hover:bg-white/10 text-white/60 hover:text-purple-400"
              onclick={openNewMacro}
              title={$t("macro.create")}
            >
              <Icon icon="mdi:plus" class="w-5 h-5" />
            </button>
          </div>

          <!-- Running indicator -->
          {#if isRunning}
            <div class="flex items-center justify-between p-2 rounded bg-purple-500/20 border border-purple-500/30">
              <div class="flex items-center gap-2">
                <Icon icon="mdi:loading" class="w-4 h-4 text-purple-400 animate-spin" />
                <span class="text-xs text-purple-300">{$t("macro.running")}</span>
              </div>
              <button
                class="px-2 py-1 rounded bg-red-500/20 hover:bg-red-500/30 text-red-400 text-xs"
                onclick={stopMacro}
              >
                {$t("common.stop")}
              </button>
            </div>
          {/if}

          <!-- Macro List -->
          {#if macros.length === 0}
            <div class="text-center py-4">
              <Icon icon="mdi:code-braces-box" class="w-8 h-8 text-white/20 mx-auto mb-2" />
              <p class="text-xs text-white/40">{$t("macro.noMacros")}</p>
              <button
                class="mt-2 px-3 py-1.5 rounded bg-purple-500/20 hover:bg-purple-500/30 text-purple-400 text-xs"
                onclick={openNewMacro}
              >
                {$t("macro.createFirst")}
              </button>
            </div>
          {:else}
            <div class="space-y-1 max-h-48 overflow-y-auto scrollbar-thin">
              {#each macros as macro}
                <div
                  class="flex items-center gap-2 p-2 rounded bg-white/5 hover:bg-white/10 transition-colors group"
                >
                  <button
                    class="flex-1 flex items-center gap-2 text-left"
                    onclick={() => openEditMacro(macro)}
                  >
                    <Icon icon="mdi:script-text-outline" class="w-4 h-4 text-white/40 group-hover:text-purple-400" />
                    <div class="flex-1 min-w-0">
                      <p class="text-sm text-white/90 truncate">{macro.name}</p>
                      {#if macro.hotkey}
                        <p class="text-xs text-white/40 font-mono">{macro.hotkey}</p>
                      {/if}
                    </div>
                  </button>
                  <button
                    class="p-1.5 rounded opacity-0 group-hover:opacity-100 {runningMacroId === macro.id 
                      ? 'bg-red-500/20 text-red-400' 
                      : 'bg-purple-500/20 text-purple-400 hover:bg-purple-500/30'}"
                    onclick={() => runningMacroId === macro.id ? stopMacro() : runMacro(macro.id)}
                    title={runningMacroId === macro.id ? $t("common.stop") : $t("macro.run")}
                  >
                    <Icon icon={runningMacroId === macro.id ? "mdi:stop" : "mdi:play"} class="w-4 h-4" />
                  </button>
                </div>
              {/each}
            </div>
          {/if}

          <!-- Info -->
          <p class="text-xs text-white/40">
            {$t("macro.description")}
          </p>
        </div>
      {/if}
    </div>
  {/if}
</div>

