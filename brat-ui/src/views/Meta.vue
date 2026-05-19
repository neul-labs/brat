<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue';
import { useRepoStore } from '../stores/repo';
import { usePolling } from '../composables/usePolling';
import bratApi from '../api/brat';
import type { MetaStatus, MetaMessage } from '../types/brat';
import LoadingSpinner from '../components/common/LoadingSpinner.vue';

const repoStore = useRepoStore();

const metaStatus = ref<MetaStatus | null>(null);
const messages = ref<MetaMessage[]>([]);
const inputMessage = ref('');
const loading = ref(false);
const sending = ref(false);
const error = ref<string | null>(null);

const messagesContainer = ref<HTMLElement | null>(null);

const isActive = computed(() => metaStatus.value?.active ?? false);
const isEnabled = computed(() => !!repoStore.activeRepoId && isActive.value);

async function fetchStatus() {
  if (!repoStore.activeRepoId) return;
  try {
    metaStatus.value = await bratApi.getMetaStatus(repoStore.activeRepoId);
  } catch (e) {
    metaStatus.value = { active: false };
  }
}

async function fetchHistory() {
  if (!repoStore.activeRepoId || !isActive.value) return;
  try {
    const response = await bratApi.getMetaHistory(repoStore.activeRepoId, 100);
    parseHistoryLines(response.lines);
  } catch (e) {
    // Silently fail
  }
}

function parseHistoryLines(lines: string[]) {
  const newMessages: MetaMessage[] = [];
  let currentMessage: MetaMessage | null = null;

  for (const line of lines) {
    if (line.startsWith('>>> ')) {
      if (currentMessage) newMessages.push(currentMessage);
      currentMessage = { type: 'user', content: line.substring(4) };
    } else if (line.trim() === '') {
      if (currentMessage) {
        newMessages.push(currentMessage);
        currentMessage = null;
      }
    } else {
      if (currentMessage?.type === 'user') {
        newMessages.push(currentMessage);
        currentMessage = { type: 'meta', content: line };
      } else if (currentMessage?.type === 'meta') {
        currentMessage.content += '\n' + line;
      } else {
        currentMessage = { type: 'meta', content: line };
      }
    }
  }

  if (currentMessage) newMessages.push(currentMessage);
  messages.value = newMessages;
}

async function startMeta() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    const response = await bratApi.startMeta(repoStore.activeRepoId);
    metaStatus.value = { active: true, session_id: response.session_id };
    if (response.response.length > 0) {
      messages.value = [{ type: 'meta', content: response.response.join('\n') }];
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to start Meta';
  } finally {
    loading.value = false;
  }
}

async function stopMeta() {
  if (!repoStore.activeRepoId) return;
  loading.value = true;
  error.value = null;
  try {
    await bratApi.stopMeta(repoStore.activeRepoId);
    metaStatus.value = { active: false };
    messages.value = [];
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to stop Meta';
  } finally {
    loading.value = false;
  }
}

async function sendMessage() {
  if (!repoStore.activeRepoId || !inputMessage.value.trim() || sending.value) return;

  const message = inputMessage.value.trim();
  inputMessage.value = '';

  messages.value.push({ type: 'user', content: message });
  await scrollToBottom();

  sending.value = true;
  error.value = null;

  try {
    const response = await bratApi.askMeta(repoStore.activeRepoId, message);
    messages.value.push({ type: 'meta', content: response.response.join('\n') });
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to send message';
    messages.value.push({ type: 'meta', content: `Error: ${error.value}` });
  } finally {
    sending.value = false;
    await scrollToBottom();
  }
}

async function scrollToBottom() {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  }
}

const { } = usePolling(fetchHistory, { interval: 3000, enabled: isEnabled });

onMounted(async () => {
  await fetchStatus();
  if (isActive.value) {
    await fetchHistory();
  }
});

watch(isActive, async (active) => {
  if (active) {
    await fetchHistory();
  }
});
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-4">
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold text-gray-900">Meta Agent</h1>
        <span
          :class="[
            'flex items-center gap-2 px-3 py-1 rounded-full text-sm font-medium',
            isActive ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
          ]"
        >
          <span :class="['w-2 h-2 rounded-full', isActive ? 'bg-green-500' : 'bg-gray-400']"></span>
          {{ isActive ? 'Active' : 'Inactive' }}
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          v-if="!isActive"
          @click="startMeta"
          class="btn-primary"
          :disabled="loading || !repoStore.activeRepoId"
        >
          <LoadingSpinner v-if="loading" size="sm" />
          <span v-else>Start Meta</span>
        </button>
        <button
          v-else
          @click="stopMeta"
          class="btn-danger"
          :disabled="loading"
        >
          <LoadingSpinner v-if="loading" size="sm" />
          <span v-else>Stop Meta</span>
        </button>
      </div>
    </div>

    <div v-if="error" class="bg-red-50 text-red-700 p-3 rounded-lg mb-4">
      {{ error }}
    </div>

    <div class="flex-1 bg-white rounded-lg shadow flex flex-col overflow-hidden">
      <div
        ref="messagesContainer"
        class="flex-1 overflow-y-auto p-4 space-y-4"
      >
        <div v-if="!isActive && messages.length === 0" class="text-center py-16 text-gray-500">
          <p class="text-lg mb-2">Meta Agent is not active</p>
          <p class="text-sm">Start the Meta Agent to begin interacting with the AI orchestrator.</p>
        </div>

        <div
          v-for="(msg, index) in messages"
          :key="index"
          :class="['max-w-3xl', msg.type === 'user' ? 'ml-auto' : '']"
        >
          <div
            v-if="msg.type === 'user'"
            class="bg-blue-600 text-white px-4 py-2 rounded-lg rounded-br-none"
          >
            <span class="text-blue-200 font-mono mr-2">>>></span>
            {{ msg.content }}
          </div>

          <div
            v-else
            class="bg-gray-100 px-4 py-3 rounded-lg rounded-bl-none"
          >
            <div class="prose prose-sm max-w-none" v-html="formatMessage(msg.content)"></div>
          </div>
        </div>

        <div v-if="sending" class="flex items-center gap-2 text-gray-500">
          <LoadingSpinner size="sm" />
          <span>Meta is thinking...</span>
        </div>
      </div>

      <div class="border-t border-gray-200 p-4">
        <div class="flex gap-2">
          <input
            v-model="inputMessage"
            type="text"
            placeholder="Ask the Meta Agent..."
            class="flex-1 input"
            :disabled="!isActive || sending"
            @keydown.enter="sendMessage"
          />
          <button
            @click="sendMessage"
            class="btn-primary"
            :disabled="!isActive || !inputMessage.trim() || sending"
          >
            <span v-if="sending">...</span>
            <span v-else>Send</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
function formatMessage(content: string): string {
  return content
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/```([\s\S]*?)```/g, '<pre class="bg-gray-800 text-green-400 p-2 rounded mt-2 mb-2 overflow-x-auto"><code>$1</code></pre>')
    .replace(/`([^`]+)`/g, '<code class="bg-gray-200 px-1 rounded">$1</code>')
    .replace(/\n/g, '<br />');
}

export { formatMessage };
</script>
