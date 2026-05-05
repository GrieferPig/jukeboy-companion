<script setup lang="ts">
import { computed, reactive, ref } from "vue";

import {
  buildInitialFormState,
  buildInitialRunState,
  commandSections,
  isFieldVisible,
  type CommandSpec,
} from "../../debug/commands";
import { runCommand } from "../../services/companion";
import { useCompanionStore } from "../../stores/companion";
import { formatJson } from "../../utils/formatting";

interface ActivityEntry {
  id: string;
  command: string;
  status: "success" | "error";
  time: string;
  payload: unknown;
}

const store = useCompanionStore();
const formState = reactive(buildInitialFormState());
const commandRuns = reactive(buildInitialRunState());
const busyCommand = ref<string | null>(null);
const latestCommandResult = ref<ActivityEntry | null>(null);

let resultCounter = 0;

const eventLog = computed(() => store.frameLog.slice(0, 24));

function nextId(): string {
  resultCounter += 1;
  return `debug-result-${resultCounter}`;
}

function timestamp(): string {
  return new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

async function executeCommand(command: CommandSpec): Promise<void> {
  const request = command.buildRequest(formState[command.name]);
  busyCommand.value = command.name;
  commandRuns[command.name] = { state: "running", at: timestamp() };

  try {
    const response = await runCommand(
      command.name,
      command.requestMode === "none" ? undefined : request,
    );

    latestCommandResult.value = {
      id: nextId(),
      command: command.name,
      status: "success",
      time: timestamp(),
      payload: { request, response },
    };
    commandRuns[command.name] = { state: "success", at: timestamp() };
    await store.syncAfterDebugCommand(command.name);
  } catch (error) {
    latestCommandResult.value = {
      id: nextId(),
      command: command.name,
      status: "error",
      time: timestamp(),
      payload: {
        request,
        error: error instanceof Error ? error.message : String(error),
      },
    };
    commandRuns[command.name] = { state: "error", at: timestamp() };
  } finally {
    busyCommand.value = null;
  }
}
</script>

<template>
  <div class="debug-panel">
    <v-card class="debug-hero" color="surface-variant">
      <div>
        <p class="eyebrow">Debug Menu</p>
        <h2 class="debug-title">Backend Smoke Console</h2>
        <p class="debug-copy">
          Every Tauri command is still available here, but it no longer dominates the primary app surface.
        </p>
      </div>

      <div class="debug-stats">
        <div>
          <span>Event stream</span>
          <strong>{{ store.frameListenerState }}</strong>
        </div>
        <div>
          <span>Last command</span>
          <strong>{{ latestCommandResult?.command ?? "none yet" }}</strong>
        </div>
        <div>
          <span>Recent events</span>
          <strong>{{ eventLog.length }}</strong>
        </div>
      </div>
    </v-card>

    <div class="debug-layout">
      <div class="debug-sections">
        <v-card v-for="section in commandSections" :key="section.title" class="debug-section" color="surface">
          <div class="debug-section__header">
            <div>
              <p class="eyebrow">{{ section.title }}</p>
              <h3>{{ section.description }}</h3>
            </div>
          </div>

          <div class="debug-command-grid">
            <v-card v-for="command in section.commands" :key="command.name" class="debug-command" color="surface-variant">
              <div class="debug-command__header">
                <div>
                  <h4>{{ command.title }}</h4>
                  <p>{{ command.description }}</p>
                </div>
                <v-chip :color="commandRuns[command.name].state === 'error' ? 'surface' : 'primary'" class="text-uppercase" size="small">
                  {{ commandRuns[command.name].state }}
                </v-chip>
              </div>

              <form class="debug-command__form" @submit.prevent="executeCommand(command)">
                <div v-if="command.fields.length > 0" class="debug-fields">
                  <template v-for="field in command.fields" :key="field.key">
                    <v-switch
                      v-if="field.type === 'checkbox' && isFieldVisible(command, field, formState[command.name])"
                      v-model="formState[command.name][field.key]"
                      :label="field.label"
                      color="primary"
                    />

                    <v-select
                      v-else-if="field.type === 'select' && isFieldVisible(command, field, formState[command.name])"
                      v-model="formState[command.name][field.key]"
                      :label="field.label"
                      :items="field.options ?? []"
                      item-title="label"
                      item-value="value"
                    />

                    <v-text-field
                      v-else-if="isFieldVisible(command, field, formState[command.name])"
                      v-model="formState[command.name][field.key]"
                      :label="field.label"
                      :type="field.type"
                      :min="field.min"
                      :step="field.step"
                      :placeholder="field.placeholder"
                    />
                  </template>
                </div>

                <p v-else class="debug-empty-copy">No request payload required.</p>

                <div class="debug-command__footer">
                  <code>{{ command.name }}</code>
                  <v-btn type="submit" color="primary" :loading="busyCommand === command.name">
                    Run
                  </v-btn>
                </div>
              </form>
            </v-card>
          </div>
        </v-card>
      </div>

      <div class="debug-sidebar">
        <v-card class="debug-sidebar__panel" color="surface">
          <div class="debug-sidebar__header">
            <div>
              <p class="eyebrow">Latest Output</p>
              <h3>{{ latestCommandResult?.command ?? 'No command fired yet' }}</h3>
            </div>
            <v-chip v-if="latestCommandResult" :color="latestCommandResult.status === 'error' ? 'surface' : 'primary'" size="small">
              {{ latestCommandResult.status }}
            </v-chip>
          </div>

          <p v-if="latestCommandResult" class="debug-panel__time">{{ latestCommandResult.time }}</p>
          <pre class="debug-payload">{{ latestCommandResult ? formatJson(latestCommandResult.payload) : 'Run a command to inspect the request and response here.' }}</pre>
        </v-card>

        <v-card class="debug-sidebar__panel" color="surface">
          <div class="debug-sidebar__header">
            <div>
              <p class="eyebrow">Frame Log</p>
              <h3>companion://frame</h3>
            </div>
            <v-btn variant="text" color="primary" size="small" @click="store.clearFrameLog()">
              Clear
            </v-btn>
          </div>

          <div v-if="eventLog.length === 0" class="debug-empty-copy">
            No unsolicited frames received yet.
          </div>

          <div v-else class="debug-event-list">
            <article v-for="entry in eventLog" :key="entry.id" class="debug-event-item">
              <div class="debug-event-item__meta">
                <strong>{{ entry.command }}</strong>
                <span>{{ entry.time }}</span>
              </div>
              <pre class="debug-payload debug-payload--compact">{{ formatJson(entry.payload) }}</pre>
            </article>
          </div>
        </v-card>
      </div>
    </div>
  </div>
</template>

<style scoped>
.debug-panel {
  display: grid;
  gap: 1.25rem;
}

.debug-hero {
  display: grid;
  gap: 1rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: var(--jukeboy-shadow);
  padding: 1.2rem;
}

.debug-title {
  margin: 0;
  font-size: clamp(1.8rem, 3vw, 2.7rem);
  font-weight: 800;
  letter-spacing: -0.05em;
}

.debug-copy,
.debug-empty-copy,
.debug-command__header p,
.debug-panel__time {
  color: rgba(255, 255, 255, 0.7);
}

.debug-stats {
  display: grid;
  gap: 0.8rem;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
}

.debug-stats div,
.debug-event-item {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 0.85rem 1rem;
  background: rgba(255, 255, 255, 0.03);
}

.debug-stats span {
  display: block;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: rgba(255, 255, 255, 0.58);
}

.debug-layout {
  display: grid;
  gap: 1rem;
}

.debug-sections,
.debug-sidebar {
  display: grid;
  gap: 1rem;
}

.debug-section,
.debug-sidebar__panel,
.debug-command {
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: var(--jukeboy-shadow);
}

.debug-section {
  padding: 1rem;
}

.debug-section__header h3,
.debug-sidebar__header h3,
.debug-command__header h4 {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
}

.debug-command-grid {
  display: grid;
  gap: 0.85rem;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  margin-top: 1rem;
}

.debug-command {
  padding: 1rem;
}

.debug-command__header,
.debug-sidebar__header,
.debug-event-item__meta,
.debug-command__footer {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.debug-command__form,
.debug-fields {
  display: grid;
  gap: 0.75rem;
}

.debug-command__footer {
  align-items: center;
}

.debug-command__footer code {
  color: rgba(255, 255, 255, 0.62);
  word-break: break-all;
}

.debug-sidebar__panel {
  padding: 1rem;
}

.debug-event-list {
  display: grid;
  gap: 0.75rem;
  max-height: 580px;
  overflow: auto;
}

.debug-event-item__meta span {
  color: rgba(255, 255, 255, 0.55);
  font-size: 0.78rem;
}

.debug-payload {
  margin: 0;
  padding: 0.95rem;
  border-radius: 18px;
  background: rgba(0, 0, 0, 0.35);
  color: #f0f0f0;
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 0.8rem;
  line-height: 1.5;
  overflow: auto;
}

.debug-payload--compact {
  max-height: 220px;
}

@media (min-width: 1200px) {
  .debug-layout {
    grid-template-columns: minmax(0, 1fr) 360px;
  }

  .debug-sidebar {
    position: sticky;
    top: 1rem;
    align-self: start;
  }
}
</style>